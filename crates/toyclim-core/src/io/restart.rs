//! Restart files: each spectral array as `<name>.npy` (float64, shape `[n, 2]` = re, im)
//! plus `restart.json` with scalar counters and the config echo.

use crate::model::RestartArray;
use crate::spectral::Complex;
use crate::Real;
use serde_json::{json, Value};
use std::path::Path;

/// Write a restart directory.
pub fn write(
    dir: &Path,
    arrays: &[RestartArray],
    scalars: &[(String, f64)],
    config: Value,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut names = Vec::new();
    for (name, data) in arrays {
        let flat: Vec<f64> = data
            .iter()
            .flat_map(|c| [c.re as f64, c.im as f64])
            .collect();
        super::npy::write(&dir.join(format!("{name}.npy")), &flat, &[data.len(), 2])
            .map_err(std::io::Error::other)?;
        names.push(name.clone());
    }
    let sc: serde_json::Map<String, Value> =
        scalars.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
    let meta = json!({ "arrays": names, "scalars": sc, "config": config });
    std::fs::write(
        dir.join("restart.json"),
        serde_json::to_string_pretty(&meta)?,
    )
}

/// Contents of a restart directory.
pub struct Restart {
    pub arrays: Vec<RestartArray>,
    pub scalars: Vec<(String, f64)>,
    pub config: Value,
}

/// Read a restart directory.
pub fn read(dir: &Path) -> std::io::Result<Restart> {
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("restart.json"))?)
        .map_err(std::io::Error::other)?;
    let bad = |m: &str| std::io::Error::other(m.to_string());
    let mut arrays = Vec::new();
    for n in meta["arrays"]
        .as_array()
        .ok_or_else(|| bad("restart.json: arrays"))?
    {
        let name = n.as_str().ok_or_else(|| bad("array name"))?;
        let (flat, shape) = super::npy::read::<f64>(&dir.join(format!("{name}.npy")))
            .map_err(std::io::Error::other)?;
        if shape.len() != 2 || shape[1] != 2 {
            return Err(bad("restart array shape"));
        }
        let data = flat
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| Complex::new(c[0] as Real, c[1] as Real))
            .collect();
        arrays.push((name.to_string(), data));
    }
    let scalars = meta["scalars"]
        .as_object()
        .ok_or_else(|| bad("restart.json: scalars"))?
        .iter()
        .map(|(k, v)| (k.clone(), v.as_f64().unwrap_or(f64::NAN)))
        .collect();
    Ok(Restart {
        arrays,
        scalars,
        config: meta["config"].clone(),
    })
}
