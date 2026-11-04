//! `toyclim`: headless runner (SPEC.md §6).
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use toyclim_core::build::build_model;
use toyclim_core::climatology::Climatology;
use toyclim_core::config::{Config, ModelKind};
use toyclim_core::grid::GaussianGrid;
use toyclim_core::io::{
    climatology as clim_io, meta::RunMeta, npy, restart, stations as station_io,
};
use toyclim_core::model::Model;
use toyclim_core::stations::StationRecorder;

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
        /// Continue from a restart directory (written as <out>/restart by a previous run).
        #[arg(long)]
        restart: Option<PathBuf>,
        /// Override the run length (e.g. "2day").
        #[arg(long)]
        length: Option<String>,
    },
    /// Benchmark a case: steps without output, reports model days per wall second.
    Bench {
        #[arg(long)]
        config: PathBuf,
        /// Override the run length (e.g. "1day").
        #[arg(long)]
        length: Option<String>,
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
        Cmd::Run {
            config,
            out,
            restart,
            length,
        } => {
            let mut c = Config::from_file(&config)?;
            if let Some(l) = length {
                c.length = l.parse()?;
            }
            if c.model == ModelKind::Dummy {
                run_dummy(&config, &out)
            } else {
                run_model(&c, &out, restart.as_deref())
            }
        }
        Cmd::Bench { config, length } => {
            let mut c = Config::from_file(&config)?;
            if let Some(l) = length {
                c.length = l.parse()?;
            }
            bench(&c)
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

fn run_dummy(config_path: &Path, out: &Path) -> Result<()> {
    let config = Config::from_file(config_path)?;
    let planet = config.planet.resolve()?;
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
        calendar: None,
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

fn write_fields(model: &dyn Model, fields: &[String], dir: &Path) -> Result<()> {
    let g = model.grid();
    for f in fields {
        let field = model
            .field(f)
            .with_context(|| format!("model {} has no field {f:?}", model.kind()))?;
        let data32: Vec<f32> = field.data.iter().map(|&x| x as f32).collect();
        let shape: Vec<usize> = if field.nlev == 1 {
            vec![g.nlat, g.nlon]
        } else {
            vec![field.nlev, g.nlat, g.nlon]
        };
        npy::write(
            &dir.join(format!("{f}_{:06}.npy", model.step_count())),
            &data32,
            &shape,
        )?;
    }
    Ok(())
}

fn dump_restart(model: &dyn Model, config: &Config, dir: &Path) -> Result<()> {
    let (arrays, scalars) = model.restart_state();
    restart::write(dir, &arrays, &scalars, serde_json::to_value(config)?)?;
    Ok(())
}

fn run_model(config: &Config, out: &Path, restart_dir: Option<&Path>) -> Result<()> {
    let planet = config.planet.resolve()?;
    let mut model = build_model(config).map_err(anyhow::Error::msg)?;
    if let Some(dir) = restart_dir {
        let r = restart::read(dir).with_context(|| format!("reading restart {}", dir.display()))?;
        model
            .load_restart_state(&r.arrays, &r.scalars)
            .map_err(anyhow::Error::msg)?;
        log::info!(
            "restarted from {} at step {}",
            dir.display(),
            model.step_count()
        );
    }
    let fields_dir = out.join("fields");
    fs::create_dir_all(&fields_dir).with_context(|| format!("creating {}", out.display()))?;
    let sigma = model.sigma();
    let meta = RunMeta {
        toyclim_version: env!("CARGO_PKG_VERSION"),
        git_hash: env!("TOYCLIM_GIT_HASH"),
        precision: RunMeta::precision(),
        model: config.model.as_str(),
        truncation: config.truncation,
        grid: model.grid(),
        sigma: &sigma,
        planet: &planet,
        output_interval_s: config.output_every.seconds(),
        config: serde_json::to_value(config)?,
        calendar: config.orbit.as_ref().map(|o| {
            let orb = o.resolve(planet.rotation_rate);
            toyclim_core::orbit::Calendar::new(orb.year, orb.solar_day(), o.months.clone())
        }),
    };
    meta.write(&out.join("meta.json"))?;

    let fields: Vec<String> = if config.output_fields.is_empty() {
        model.field_names().iter().map(|s| s.to_string()).collect()
    } else {
        config.output_fields.clone()
    };
    let diag_path = out.join("diagnostics.csv");
    let append = restart_dir.is_some() && diag_path.exists();
    let mut diag = fs::OpenOptions::new()
        .create(true)
        .append(append)
        .write(true)
        .truncate(!append)
        .open(&diag_path)?;
    let header: Vec<&str> = model.diagnostics().iter().map(|(n, _)| *n).collect();
    if !append {
        writeln!(diag, "step,time_s,{}", header.join(","))?;
    }
    let write_diag = |model: &dyn Model, diag: &mut fs::File| -> Result<()> {
        let vals: Vec<String> = model
            .diagnostics()
            .iter()
            .map(|(_, v)| format!("{v:.17e}"))
            .collect();
        writeln!(
            diag,
            "{},{},{}",
            model.step_count(),
            model.time(),
            vals.join(",")
        )?;
        Ok(())
    };

    let every = config.steps_per_output();
    let total = config.total_steps();
    let mut stations =
        StationRecorder::new(model.as_ref(), &config.stations, &config.station_fields);
    let station_every = config.steps_per_station_sample();
    let station_dir = out.join("stations");
    let clim_dir = out.join("climatology");
    let mut clim = match &config.climatology {
        Some(c) => {
            let every = (c.every.seconds() / config.dt.seconds()).round() as u64;
            Some(
                Climatology::new(model.as_ref(), c.start.seconds(), every, &c.maps)
                    .map_err(anyhow::Error::msg)?,
            )
        }
        None => None,
    };
    if restart_dir.is_some() {
        station_io::read_into(&mut stations, &station_dir, model.time())?;
        if let Some(c) = clim.as_mut() {
            clim_io::read_into(c, &clim_dir)?;
        }
    }
    if model.step_count() == 0 {
        write_fields(model.as_ref(), &fields, &fields_dir)?;
        write_diag(model.as_ref(), &mut diag)?;
        stations
            .record(model.as_ref())
            .map_err(anyhow::Error::msg)?;
    }
    let t0 = Instant::now();
    let start_step = model.step_count();
    while model.step_count() < total {
        model.step();
        let s = model.step_count();
        if s % station_every == 0 {
            stations
                .record(model.as_ref())
                .map_err(anyhow::Error::msg)?;
        }
        if let Some(c) = clim.as_mut() {
            c.maybe_sample(model.as_ref());
        }
        // Checking every step is cheap (a scan of the spectral state) and stops a blow-up
        // before it pollutes outputs and statistics.
        if !model.is_finite() {
            let crash = out.join("restart_crash");
            dump_restart(model.as_ref(), config, &crash)?;
            bail!(
                "non-finite state at step {s} (t = {:.3} d); state dumped to {}",
                model.time() / 86400.0,
                crash.display()
            );
        }
        if s % every == 0 || s == total {
            write_fields(model.as_ref(), &fields, &fields_dir)?;
            write_diag(model.as_ref(), &mut diag)?;
            station_io::write(&stations, &station_dir)?;
            if let Some(c) = &clim {
                clim_io::write(c, &clim_dir, model.grid().nlon)?;
            }
            let d = model.diagnostics();
            let courant = d
                .iter()
                .find(|(n, _)| *n == "max_courant")
                .map(|x| x.1)
                .unwrap_or(f64::NAN);
            log::info!(
                "step {s:>7}  day {:>9.3}  max Courant {courant:.3}",
                model.time() / 86400.0
            );
        }
    }
    let wall = t0.elapsed().as_secs_f64();
    let days = (model.step_count() - start_step) as f64 * model.dt() / 86400.0;
    log::info!(
        "done: {days:.2} model days in {wall:.2} s wall ({:.2} model days / s)",
        days / wall.max(1e-9)
    );
    dump_restart(model.as_ref(), config, &out.join("restart"))?;
    Ok(())
}

fn bench(config: &Config) -> Result<()> {
    let mut model = build_model(config).map_err(anyhow::Error::msg)?;
    let total = config.total_steps();
    let t0 = Instant::now();
    for _ in 0..total {
        model.step();
    }
    let wall = t0.elapsed().as_secs_f64();
    let days = total as f64 * model.dt() / 86400.0;
    println!(
        "{} T{}: {total} steps, {days:.3} model days in {wall:.3} s -> {:.2} model days/s, {:.3} ms/step (threads: {})",
        model.kind(),
        config.truncation,
        days / wall,
        wall * 1e3 / total as f64,
        rayon_threads()
    );
    if !model.is_finite() {
        bail!("state became non-finite");
    }
    Ok(())
}

fn rayon_threads() -> String {
    std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "default".into())
}
