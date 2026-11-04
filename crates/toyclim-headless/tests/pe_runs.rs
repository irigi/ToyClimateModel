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
        let mut fs: Vec<PathBuf> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
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
    let cfg = tmp("moist.toml");
    std::fs::write(&cfg, MOIST).unwrap();
    let (a, b, c) = (tmp("ma"), tmp("mb"), tmp("mc"));
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

/// Land, orography, bucket, slab ocean, gray radiation and orbit together: the model runs,
/// stays finite and keeps the bucket within its bounds; restart is bitwise.
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
output_fields = ["ts", "bucket", "precip", "orog", "land"]
initial_condition = {{ case = "rest", temperature = 280.0 }}
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
    let (a, c) = (tmp("la"), tmp("lc"));
    run(&cfg, &a, "2", &[]);
    run(&cfg, &c, "2", &["--length", "2day"]);
    let r = c.join("restart");
    run(&cfg, &c, "2", &["--restart", r.to_str().unwrap()]);
    for f in ["ts_000192.npy", "bucket_000192.npy", "precip_000192.npy"] {
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
    for d in [&a, &c] {
        std::fs::remove_dir_all(d).ok();
    }
    std::fs::remove_file(&cfg).ok();
}
