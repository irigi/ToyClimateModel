//! M6: determinism across thread counts and bitwise restarts for the primitive-equation
//! model with physics, stations and climatology.

use std::path::{Path, PathBuf};
use std::process::Command;

const CFG: &str = r#"
model = "primitive"
truncation = 21
levels = 10
dt = "30min"
length = "10day"
output_every = "2day"
output_fields = ["u", "t", "ps", "t850"]
initial_condition = { case = "rest", temperature = 300.0, noise = 0.1, seed = 7 }
station_every = "3h"
station_fields = ["ps", "t850", "u"]
[[stations]]
name = "a"
lat = 45.0
lon = 10.0
[[stations]]
name = "b"
lat = -20.0
lon = 200.0
[planet]
preset = "earth"
[forcing]
type = "held_suarez"
[climatology]
start = "4day"
every = "6h"
maps = ["ps", "t850"]
"#;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("toyclim_pe_{}_{name}", std::process::id()))
}

fn run(cfg: &Path, out: &Path, threads: &str, extra: &[&str]) {
    let st = Command::new(env!("CARGO_BIN_EXE_toyclim"))
        .env("RUST_LOG", "warn")
        .env("RAYON_NUM_THREADS", threads)
        .args(["run", "--config"])
        .arg(cfg)
        .arg("--out")
        .arg(out)
        .args(extra)
        .status()
        .unwrap();
    assert!(st.success());
}

/// All files under `dir` except meta.json (it echoes the run options) and restart dirs.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for sub in ["fields", "stations", "climatology"] {
        let d = dir.join(sub);
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        let mut fs: Vec<PathBuf> = rd.map(|e| e.unwrap().path()).collect();
        fs.sort();
        v.extend(fs);
    }
    v.push(dir.join("diagnostics.csv"));
    v
}

fn assert_same(a: &Path, b: &Path) {
    let fa = files(a);
    let fb = files(b);
    assert_eq!(fa.len(), fb.len(), "different file sets");
    for (x, y) in fa.iter().zip(&fb) {
        assert_eq!(x.file_name(), y.file_name());
        assert!(
            std::fs::read(x).unwrap() == std::fs::read(y).unwrap(),
            "{} differs",
            x.display()
        );
    }
}

#[test]
fn pe_bitwise_across_threads_and_restart() {
    let cfg = tmp("cfg.toml");
    std::fs::write(&cfg, CFG).unwrap();
    let (a, b, c) = (tmp("a"), tmp("b"), tmp("c"));
    run(&cfg, &a, "1", &[]);
    run(&cfg, &b, "4", &[]);
    assert_same(&a, &b);
    // restart: 6 days, then continue to 10
    run(&cfg, &c, "3", &["--length", "6day"]);
    let r = c.join("restart");
    run(&cfg, &c, "2", &["--restart", r.to_str().unwrap()]);
    assert_same(&a, &c);
    for d in [&a, &b, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    std::fs::remove_file(&cfg).ok();
}

const MOIST: &str = r#"
model = "primitive"
truncation = 21
levels = 12
dt = "30min"
length = "6day"
output_every = "2day"
output_fields = ["q", "t", "precip", "pw"]
initial_condition = { case = "jw06", perturbed = true }
station_every = "1h"
station_fields = ["precip", "pw"]
[[stations]]
name = "eq"
lat = 0.0
lon = 30.0
[moisture]
condensable = "water"
[planet]
preset = "earth"
[forcing]
type = "moist_held_suarez"
convection = "sbm"
[climatology]
start = "2day"
every = "6h"
maps = ["precip", "evap"]
"#;

/// Moist model (tracer, fixer, SBM): bitwise across thread counts and restarts; physical
/// sanity (it rains, precipitable water stays finite and positive).
#[test]
fn moist_bitwise_and_sane() {
    moist_case(MOIST, "m");
}

/// The same with grid-point semi-Lagrangian humidity transport (SPEC.md §9.8): its grid
/// tracers are restart state, and the departure-point search and the vertical step do not
/// depend on the thread count.
#[test]
fn moist_semi_lagrangian_bitwise_and_sane() {
    let text = format!("{MOIST}[dynamics]\ntracer_transport = \"semi_lagrangian\"\n");
    moist_case(&text, "s");
}

fn moist_case(text: &str, tag: &str) {
    let cfg = tmp(&format!("moist_{tag}.toml"));
    std::fs::write(&cfg, text).unwrap();
    let (a, b, c) = (
        tmp(&format!("{tag}a")),
        tmp(&format!("{tag}b")),
        tmp(&format!("{tag}c")),
    );
    run(&cfg, &a, "1", &[]);
    run(&cfg, &b, "4", &[]);
    assert_same(&a, &b);
    run(&cfg, &c, "2", &["--length", "4day"]);
    let r = c.join("restart");
    run(&cfg, &c, "3", &["--restart", r.to_str().unwrap()]);
    assert_same(&a, &c);
    let (p, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/precip_000288.npy")).unwrap();
    let (pw, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/pw_000288.npy")).unwrap();
    assert!(p.iter().all(|x| x.is_finite() && *x >= 0.0));
    assert!(p.iter().any(|x| *x > 0.0), "no precipitation at all");
    assert!(pw.iter().all(|x| x.is_finite() && *x > 0.0 && *x < 200.0));
    for d in [&a, &b, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    std::fs::remove_file(&cfg).ok();
}

/// Land, orography, soil, bucket, snow, slab ocean and sea ice, gray radiation, the K-profile
/// boundary layer with the coupled surface solve (carried state: u*) and orbit together: the model runs, stays
/// finite and keeps the bucket within its bounds; restart is bitwise and the output does
/// not depend on the thread count.
#[test]
fn land_world_runs_and_restarts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let hm = root.join("worlds/testworld.npy");
    let cfg_text = format!(
        r#"
model = "primitive"
truncation = 21
levels = 10
dt = "30min"
length = "4day"
output_every = "2day"
output_fields = ["ts", "bucket", "precip", "orog", "land", "snw", "sic", "sit", "tos", "mrro"]
initial_condition = {{ case = "rest", temperature = 265.0 }}
[moisture]
condensable = "water"
[orbit]
diurnal = true
[surface]
heightmap = "{}"
[planet]
preset = "earth"
[forcing]
type = "gray_aquaplanet"
"#,
        hm.display()
    );
    let cfg = tmp("land.toml");
    std::fs::write(&cfg, cfg_text).unwrap();
    let (a, a5, c) = (tmp("la"), tmp("lb"), tmp("lc"));
    run(&cfg, &a, "2", &[]);
    run(&cfg, &a5, "5", &[]);
    assert_same(&a, &a5);
    run(&cfg, &c, "2", &["--length", "2day"]);
    let r = c.join("restart");
    run(&cfg, &c, "2", &["--restart", r.to_str().unwrap()]);
    for f in [
        "ts_000192.npy",
        "bucket_000192.npy",
        "precip_000192.npy",
        "snw_000192.npy",
        "sic_000192.npy",
        "sit_000192.npy",
        "tos_000192.npy",
        "mrro_000192.npy",
    ] {
        let x = std::fs::read(a.join("fields").join(f)).unwrap();
        let y = std::fs::read(c.join("fields").join(f)).unwrap();
        assert!(x == y, "{f} differs after restart");
    }
    let (b, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/bucket_000192.npy")).unwrap();
    let (land, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/land_000000.npy")).unwrap();
    let (ts, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/ts_000192.npy")).unwrap();
    assert!(b.iter().all(|&w| (0.0..=150.0).contains(&w)));
    assert!(land.iter().any(|&f| f >= 0.5) && land.iter().any(|&f| f < 0.5));
    assert!(ts.iter().all(|&t| t.is_finite() && t > 150.0 && t < 350.0));
    // a cold start makes snow and sea ice somewhere: the new surface state is exercised
    let (snw, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/snw_000192.npy")).unwrap();
    let (sic, _) = toyclim_core::io::npy::read::<f32>(&a.join("fields/sic_000192.npy")).unwrap();
    assert!(snw.iter().any(|&s| s > 0.0), "no snow anywhere");
    assert!(sic.iter().any(|&s| s > 0.0), "no sea ice anywhere");
    for d in [&a, &a5, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    std::fs::remove_file(&cfg).ok();
}

/// Global energy budget of the coupled moist/radiative model: the change of atmospheric
/// energy (c_p T + Φ_s + E + L q) over 30 days equals the time integral of
/// (absorbed solar − OLR − net flux into the surface) within a few W m⁻². Catches sign and
/// accounting errors between radiation, surface fluxes, condensation and the slab.
#[test]
fn coupled_energy_budget_closes() {
    energy_budget("", "plain");
}

/// Same with the experimental diagnostic clouds.
#[test]
fn coupled_energy_budget_closes_with_clouds() {
    energy_budget("clouds = true\nalbedo = 0.1\n", "clouds");
}

fn energy_budget(extra_forcing: &str, tag: &str) {
    let cfg_text = r#"
model = "primitive"
truncation = 21
levels = 10
dt = "30min"
length = "30day"
output_every = "30min"
output_fields = []
initial_condition = { case = "rest", temperature = 270.0, noise = 0.1 }
[moisture]
condensable = "water"
[orbit]
diurnal = true
[surface]
mixed_layer_depth = 20.0
[planet]
preset = "earth"
[forcing]
type = "gray_aquaplanet"
"#;
    let cfg = tmp(&format!("energy_{tag}.toml"));
    let text = cfg_text.replace("output_fields = []", "output_fields = [\"ts\"]") + extra_forcing;
    std::fs::write(&cfg, text).unwrap();
    let out = tmp(&format!("energy_{tag}"));
    run(&cfg, &out, "4", &[]);
    let text = std::fs::read_to_string(out.join("diagnostics.csv")).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |n: &str| header.iter().position(|h| *h == n).unwrap();
    let rows: Vec<Vec<f64>> = lines
        .map(|l| l.split(',').map(|x| x.parse().unwrap()).collect())
        .collect();
    let (it, ie, il) = (col("time_s"), col("energy"), col("latent_energy"));
    let (ia, io, is) = (col("asr"), col("olr"), col("surface_heat"));
    let area = 4.0 * std::f64::consts::PI * 6.37122e6f64.powi(2);
    // skip the first 5 days (adjustment from rest; physics diagnostics lag one step)
    let start = rows.iter().position(|r| r[it] >= 5.0 * 86400.0).unwrap();
    let (r0, r1) = (&rows[start], rows.last().unwrap());
    let dt = r1[it] - r0[it];
    let de = ((r1[ie] + r1[il]) - (r0[ie] + r0[il])) / dt / area; // W m⁻²
    let forcing: f64 = rows[start + 1..]
        .iter()
        .map(|r| r[ia] - r[io] - r[is])
        .sum::<f64>()
        / (rows.len() - start - 1) as f64;
    eprintln!("[{tag}] atmospheric energy tendency {de:.2} W/m², net forcing {forcing:.2} W/m²");
    assert!(
        (de - forcing).abs() < 3.0,
        "energy budget residual {:.2} W/m²",
        de - forcing
    );
    std::fs::remove_dir_all(&out).ok();
    std::fs::remove_file(&cfg).ok();
}

/// Prescribed monthly SST and sea ice (AMIP-style ocean) with land: bitwise across thread
/// counts and restarts, and the output concentration and SST are the files' values
/// interpolated linearly between mid-months.
/// P4: semi-gray radiation with the diagnostic clouds on a slab aquaplanet with sea ice.
/// The clouds are carried between physics steps (driver memory, restart array
/// `physics_memory`): output identical across thread counts and through a restart; the
/// clouds exist and stay within bounds.
#[test]
fn clouds_run_and_restart() {
    use toyclim_core::io::npy;
    let cfg_text = r#"
schema = 2
model = "primitive"
truncation = 21
levels = 10
dt = "30min"
length = "4day"
output_every = "2day"
output_fields = ["ts", "precip", "clt", "clwvi", "olr", "olr_cs", "asr", "asr_cs"]
initial_condition = { case = "jw06", perturbed = true }
[star]
t_eff = 5772.0
luminosity = 1.0
mass = 1.0
[orbit]
semi_major_axis = 1.0
eccentricity = 0.0167
obliquity = 23.44
perihelion_longitude = 282.9
spin = "free"
diurnal = true
year = "365.25day"
instellation = 1361.0
[planet]
preset = "earth"
[atmosphere]
surface_pressure = 100000.0
composition = { N2 = 0.7808, O2 = 0.2095, Ar = 0.0093, CO2 = 0.0004 }
[moisture]
condensable = "water"
initial = "tj16"
[radiation]
scheme = "semigray"
[clouds]
scheme = "diagnostic"
[boundary_layer]
scheme = "kprofile"
[convection]
scheme = "sbm"
[dynamics]
tracer_transport = "semi_lagrangian"
[surface]
geography = "aquaplanet"
[surface.ocean]
model = "slab"
mixed_layer_depth = 20.0
initial = "tj16"
[surface.sea_ice]
enabled = true
"#;
    let cfg = tmp("clouds.toml");
    std::fs::write(&cfg, cfg_text).unwrap();
    let (a, a5, c) = (tmp("ca"), tmp("cb"), tmp("cc"));
    run(&cfg, &a, "2", &[]);
    run(&cfg, &a5, "5", &[]);
    assert_same(&a, &a5);
    run(&cfg, &c, "2", &["--length", "2day"]);
    let r = c.join("restart");
    run(&cfg, &c, "2", &["--restart", r.to_str().unwrap()]);
    for f in ["ts", "precip", "clt", "clwvi", "olr"] {
        let name = format!("{f}_000192.npy");
        let x = std::fs::read(a.join("fields").join(&name)).unwrap();
        let y = std::fs::read(c.join("fields").join(&name)).unwrap();
        assert!(x == y, "{name} differs after restart");
    }
    let rd = |f: &str| {
        npy::read::<f32>(&a.join("fields").join(format!("{f}_000192.npy")))
            .unwrap()
            .0
    };
    let (clt, clwvi) = (rd("clt"), rd("clwvi"));
    assert!(clt.iter().all(|&x| (0.0..=1.0 + 1e-6).contains(&x)));
    assert!(clwvi.iter().all(|&x| x >= 0.0));
    let mean = |v: &[f32]| v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64;
    assert!(mean(&clt) > 0.05, "cloud cover {}", mean(&clt));
    assert!(mean(&clwvi) > 1e-3, "water path {}", mean(&clwvi));
    // clouds trap longwave: all-sky OLR below clear-sky on average
    assert!(mean(&rd("olr")) < mean(&rd("olr_cs")));
    for d in [&a, &a5, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    std::fs::remove_file(&cfg).ok();
}

#[test]
fn prescribed_ocean_runs_and_restarts() {
    use toyclim_core::io::npy;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let hm = root.join("worlds/testworld.npy");
    let (nlat, nlon) = (32usize, 64usize);
    let (x, _) = gauss_mu(nlat);
    // SST: 300 K at the equator falling to 271 K at the poles, +2 K × month index in the
    // north; sea ice: full poleward of 65° in month 0, none in month 6 (linear in between)
    let mut sst = Vec::new();
    let mut sic = Vec::new();
    for m in 0..12 {
        for mu in &x {
            let lat = mu.asin();
            let s = 271.0 + 29.0 * lat.cos().powi(2) + if lat > 0.0 { 2.0 * m as f64 } else { 0.0 };
            let ph = (m as f64 / 6.0 * std::f64::consts::PI).cos() * 0.5 + 0.5;
            let a = if lat.abs() > 65f64.to_radians() {
                ph
            } else {
                0.0
            };
            for _ in 0..nlon {
                sst.push(s as f32);
                sic.push(a as f32);
            }
        }
    }
    let (fs, fi) = (tmp("p_sst.npy"), tmp("p_sic.npy"));
    npy::write(&fs, &sst, &[12, nlat, nlon]).unwrap();
    npy::write(&fi, &sic, &[12, nlat, nlon]).unwrap();
    let cfg_text = format!(
        r#"
model = "primitive"
truncation = 21
levels = 10
dt = "30min"
length = "4day"
output_every = "2day"
output_fields = ["ts", "tos", "sic", "sit", "precip", "snw", "land"]
initial_condition = {{ case = "rest", temperature = 280.0 }}
[moisture]
condensable = "water"
[orbit]
diurnal = true
[surface]
heightmap = "{}"
fixed = true
prescribed_sst = "{}"
prescribed_sic = "{}"
[planet]
preset = "earth"
[forcing]
type = "gray_aquaplanet"
"#,
        hm.display(),
        fs.display(),
        fi.display()
    );
    let cfg = tmp("presc.toml");
    std::fs::write(&cfg, cfg_text).unwrap();
    let (a, b, c) = (tmp("pa"), tmp("pb"), tmp("pc"));
    run(&cfg, &a, "1", &[]);
    run(&cfg, &b, "4", &[]);
    assert_same(&a, &b);
    run(&cfg, &c, "2", &["--length", "2day"]);
    let r = c.join("restart");
    run(&cfg, &c, "3", &["--restart", r.to_str().unwrap()]);
    assert_same(&a, &c);
    // the fields of step 192 (4 days) were set by the last physics step, at step 191
    let t: f64 = 191.0 * 1800.0;
    let xm = (t / 31_557_600.0) * 12.0 - 0.5;
    let w = xm - xm.floor(); // weight of January (m1 = 0, m0 = 11)
    let (out_sic, _) = npy::read::<f32>(&a.join("fields/sic_000192.npy")).unwrap();
    let (out_tos, _) = npy::read::<f32>(&a.join("fields/tos_000192.npy")).unwrap();
    let (land, _) = npy::read::<f32>(&a.join("fields/land_000000.npy")).unwrap();
    for p in 0..nlat * nlon {
        if land[p] >= 1.0 {
            continue; // tos is the skin temperature there
        }
        let e_sic = (1.0 - w) * sic[11 * nlat * nlon + p] as f64 + w * sic[p] as f64;
        let e_sst = (1.0 - w) * sst[11 * nlat * nlon + p] as f64 + w * sst[p] as f64;
        assert!((out_sic[p] as f64 - e_sic).abs() < 1e-5, "sic at {p}");
        assert!(
            (out_tos[p] as f64 - e_sst).abs() < 1e-3,
            "tos at {p}: {} {e_sst}",
            out_tos[p]
        );
    }
    assert!(out_sic.iter().any(|&s| s > 0.5), "no prescribed ice");
    let (ts, _) = npy::read::<f32>(&a.join("fields/ts_000192.npy")).unwrap();
    assert!(ts.iter().all(|&t| t.is_finite() && t > 150.0 && t < 350.0));
    for d in [&a, &b, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    for f in [&cfg, &fs, &fi] {
        std::fs::remove_file(f).ok();
    }
}

/// Gaussian latitudes μ, north to south, and weights.
fn gauss_mu(n: usize) -> (Vec<f64>, Vec<f64>) {
    let g = toyclim_core::grid::GaussianGrid::new(n, 2 * n);
    (g.lat.iter().map(|l| l.sin()).collect(), vec![])
}
