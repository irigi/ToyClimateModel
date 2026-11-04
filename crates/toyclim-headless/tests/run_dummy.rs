//! End-to-end: `toyclim run` on the dummy case writes meta.json and readable fields.

use std::path::PathBuf;
use std::process::Command;
use toyclim_core::io::npy;

#[test]
fn dummy_run_writes_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("toyclim_dummy_{}", std::process::id()));
    let status = Command::new(env!("CARGO_BIN_EXE_toyclim"))
        .args(["run", "--config"])
        .arg(root.join("cases/dummy.toml"))
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());

    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("meta.json")).unwrap()).unwrap();
    assert_eq!(meta["truncation"], 42);
    assert_eq!(meta["grid"]["nlat"], 64);
    assert_eq!(meta["planet"]["name"], "earth");

    let (data, shape) = npy::read::<f32>(&out.join("fields/analytic_000000.npy")).unwrap();
    assert_eq!(shape, vec![64, 128]);
    // f = sin φ + cos²φ cos 3λ at the first grid point (λ = 0).
    let mu0 = meta["grid"]["mu"][0].as_f64().unwrap();
    let expect = mu0 + (1.0 - mu0 * mu0);
    assert!((data[0] as f64 - expect).abs() < 1e-6);
    assert!(out.join("fields/analytic_000048.npy").exists());
    std::fs::remove_dir_all(&out).ok();
}
