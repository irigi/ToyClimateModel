//! `toyclim`: headless runner (SPEC.md §6).
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use toyclim_core::config::{Config, ModelKind};
use toyclim_core::grid::GaussianGrid;
use toyclim_core::io::{meta::RunMeta, npy};

#[derive(Parser)]
#[command(name = "toyclim", version, about = "Toy spectral climate model")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a case and write output.
    Run {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Benchmark a case (from M2).
    Bench {
        #[arg(long)]
        config: PathBuf,
    },
    /// List the configs in a cases directory.
    ListCases {
        #[arg(long, default_value = "cases")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    match Cli::parse().cmd {
        Cmd::Run { config, out } => run(&config, &out),
        Cmd::Bench { config } => {
            let c = Config::from_file(&config)?;
            bail!(
                "bench is not implemented yet for model {:?}",
                c.model.as_str()
            )
        }
        Cmd::ListCases { dir } => list_cases(&dir),
    }
}

fn list_cases(dir: &Path) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    entries.sort();
    for p in entries {
        let name = p.file_stem().unwrap_or_default().to_string_lossy();
        match Config::from_file(&p) {
            Ok(c) => println!("{name:<28} {:<14} {}", c.model.as_str(), c.description),
            Err(e) => println!("{name:<28} INVALID: {e}"),
        }
    }
    Ok(())
}

fn run(config_path: &Path, out: &Path) -> Result<()> {
    let config = Config::from_file(config_path)?;
    let planet = config.planet.resolve()?;
    match config.model {
        ModelKind::Dummy => {}
        other => bail!("model {:?} is not implemented yet", other.as_str()),
    }
    let grid = GaussianGrid::for_truncation(config.truncation);
    log::info!(
        "{}: T{} grid {}x{}, planet {}",
        config_path.display(),
        config.truncation,
        grid.nlat,
        grid.nlon,
        planet.name
    );

    let fields_dir = out.join("fields");
    fs::create_dir_all(&fields_dir).with_context(|| format!("creating {}", out.display()))?;
    let meta = RunMeta {
        toyclim_version: env!("CARGO_PKG_VERSION"),
        git_hash: env!("TOYCLIM_GIT_HASH"),
        precision: RunMeta::precision(),
        model: config.model.as_str(),
        truncation: config.truncation,
        grid: &grid,
        sigma: &[],
        planet: &planet,
        output_interval_s: config.output_every.seconds(),
        config: serde_json::to_value(&config)?,
    };
    meta.write(&out.join("meta.json"))?;

    let fields = if config.output_fields.is_empty() {
        vec!["analytic".to_string()]
    } else {
        config.output_fields.clone()
    };
    let mut diag = fs::File::create(out.join("diagnostics.csv"))?;
    writeln!(diag, "step,time_s,global_mean")?;

    let steps_per_output = config.steps_per_output();
    let total = config.total_steps();
    let mut step = 0u64;
    loop {
        let t = step as f64 * config.dt.seconds();
        for f in &fields {
            let data = dummy_field(f, &grid, planet.rotation_rate, t)?;
            let mean = grid.integrate(&data, planet.radius)
                / (4.0 * std::f64::consts::PI * planet.radius * planet.radius);
            let data32: Vec<f32> = data.iter().map(|&x| x as f32).collect();
            let path = fields_dir.join(format!("{f}_{step:06}.npy"));
            npy::write(&path, &data32, &[grid.nlat, grid.nlon])?;
            if f == &fields[0] {
                writeln!(diag, "{step},{t},{mean:.17e}")?;
            }
        }
        if step >= total {
            break;
        }
        step = (step + steps_per_output).min(total);
    }
    log::info!("wrote output to {}", out.display());
    Ok(())
}

/// Analytic test field for the dummy model: a zonal wave-3 pattern on top of a
/// pole-to-pole gradient, drifting eastward at the planet's rotation rate.
/// f = sin φ + cos²φ cos 3(λ − Ω t)
fn dummy_field(name: &str, grid: &GaussianGrid, omega: f64, t: f64) -> Result<Vec<f64>> {
    if name != "analytic" {
        bail!("dummy model only provides the field \"analytic\", not {name:?}");
    }
    let mut out = Vec::with_capacity(grid.len());
    for j in 0..grid.nlat {
        let (mu, c2) = (grid.mu[j], 1.0 - grid.mu[j] * grid.mu[j]);
        for &lon in &grid.lon {
            out.push(mu + c2 * (3.0 * (lon - omega * t)).cos());
        }
    }
    Ok(out)
}
