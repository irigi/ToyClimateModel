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
