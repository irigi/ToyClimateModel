//! Write station series: `stations/<field>.npy` (float32 `[time][station]` or
//! `[time][station][lev]`), `stations/time.npy` (float64 seconds) and `stations/stations.json`.

use crate::stations::StationRecorder;
use serde_json::json;
use std::path::Path;

pub fn write(rec: &StationRecorder, dir: &Path) -> std::io::Result<()> {
    if rec.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    let nt = rec.times.len();
    let ns = rec.names.len();
    super::npy::write(&dir.join("time.npy"), &rec.times, &[nt]).map_err(std::io::Error::other)?;
    let mut levels = serde_json::Map::new();
    for (name, (nlev, data)) in rec.fields.iter().zip(&rec.data) {
        let shape = if *nlev <= 1 {
            vec![nt, ns]
        } else {
            vec![nt, ns, *nlev]
        };
        super::npy::write(&dir.join(format!("{name}.npy")), data, &shape)
            .map_err(std::io::Error::other)?;
        levels.insert(name.clone(), json!(nlev));
    }
    let meta = json!({
        "stations": rec.names.iter().zip(&rec.lat_deg).zip(&rec.lon_deg)
            .map(|((n, la), lo)| json!({"name": n, "lat": la, "lon": lo})).collect::<Vec<_>>(),
        "fields": rec.fields,
        "levels": levels,
        "interpolation": "bilinear from the Gaussian grid",
    });
    std::fs::write(
        dir.join("stations.json"),
        serde_json::to_string_pretty(&meta)?,
    )
}

/// Load series written earlier into `rec` (used when continuing from a restart into the same
/// output directory). Samples after `t_resume` (from a run that went further than the
/// restart point) are dropped, so a resumed run
/// does not duplicate them. Missing files are not an error.
pub fn read_into(rec: &mut StationRecorder, dir: &Path, t_resume: f64) -> std::io::Result<()> {
    let tpath = dir.join("time.npy");
    if rec.is_empty() || !tpath.exists() {
        return Ok(());
    }
    let (times, _) = super::npy::read::<f64>(&tpath).map_err(std::io::Error::other)?;
    let keep = times.iter().take_while(|&&t| t <= t_resume + 1e-6).count();
    let ns = rec.names.len();
    for (name, (nlev, data)) in rec.fields.iter().zip(rec.data.iter_mut()) {
        let p = dir.join(format!("{name}.npy"));
        if !p.exists() {
            return Err(std::io::Error::other(format!(
                "station series {name} missing"
            )));
        }
        let (old, shape) = super::npy::read::<f32>(&p).map_err(std::io::Error::other)?;
        let l = if shape.len() == 3 { shape[2] } else { 1 };
        if shape.first() != Some(&times.len()) || shape.get(1) != Some(&ns) {
            return Err(std::io::Error::other(format!(
                "station series {name}: shape mismatch"
            )));
        }
        *nlev = l;
        data.splice(0..0, old[..keep * ns * l].iter().copied());
    }
    rec.times.splice(0..0, times[..keep].iter().copied());
    Ok(())
}
