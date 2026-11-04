//! Write/read climatology accumulators: `climatology/zonal_<name>.npy` (time means,
//! float64 `[lev][lat]`), `zonal_ps.npy` (`[lat]`), `mean_<field>.npy` (float32 maps),
//! `sums.json` (count, start) and `sums_*.npy` (raw f64 sums, for continuing after a restart).

use crate::climatology::{Climatology, ZONAL};
use serde_json::json;
use std::path::Path;

pub fn write(c: &Climatology, dir: &Path, nlon: usize) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let n = c.count.max(1) as f64;
    let shape = [c.nlev, c.nlat];
    for (name, sums) in ZONAL.iter().zip(&c.zonal) {
        let mean: Vec<f64> = sums.iter().map(|s| s / n).collect();
        super::npy::write(&dir.join(format!("zonal_{name}.npy")), &mean, &shape)
            .map_err(std::io::Error::other)?;
        super::npy::write(&dir.join(format!("sums_zonal_{name}.npy")), sums, &shape)
            .map_err(std::io::Error::other)?;
    }
    let psm: Vec<f64> = c.ps.iter().map(|s| s / n).collect();
    super::npy::write(&dir.join("zonal_ps.npy"), &psm, &[c.nlat]).map_err(std::io::Error::other)?;
    super::npy::write(&dir.join("sums_zonal_ps.npy"), &c.ps, &[c.nlat])
        .map_err(std::io::Error::other)?;
    for (name, nlev, sums) in &c.maps {
        let mean: Vec<f32> = sums.iter().map(|s| (s / n) as f32).collect();
        let shape: Vec<usize> = if *nlev == 1 {
            vec![c.nlat, nlon]
        } else {
            vec![*nlev, c.nlat, nlon]
        };
        super::npy::write(&dir.join(format!("mean_{name}.npy")), &mean, &shape)
            .map_err(std::io::Error::other)?;
        super::npy::write(
            &dir.join(format!("sums_mean_{name}.npy")),
            sums,
            &[sums.len()],
        )
        .map_err(std::io::Error::other)?;
    }
    if let Some(cal) = &c.calendar {
        let nm = cal.months.len();
        for (fi, (name, nlev, _)) in c.maps.iter().enumerate() {
            let per = c.monthly[fi][0].len();
            let mut mean = Vec::with_capacity(nm * per);
            for m in 0..nm {
                // months without samples are NaN, so incomplete years are visible
                let n = c.monthly_count[m] as f64;
                mean.extend(c.monthly[fi][m].iter().map(|s| if n > 0.0 { (s / n) as f32 } else { f32::NAN }));
            }
            let shape: Vec<usize> = if *nlev == 1 {
                vec![nm, c.nlat, nlon]
            } else {
                vec![nm, *nlev, c.nlat, nlon]
            };
            super::npy::write(&dir.join(format!("monthly_{name}.npy")), &mean, &shape)
                .map_err(std::io::Error::other)?;
            let flat: Vec<f64> = c.monthly[fi].iter().flatten().copied().collect();
            super::npy::write(
                &dir.join(format!("sums_monthly_{name}.npy")),
                &flat,
                &[flat.len()],
            )
            .map_err(std::io::Error::other)?;
        }
    }
    let meta = json!({"count": c.count, "start_s": c.start, "every_steps": c.every_steps,
                      "monthly_count": c.monthly_count,
                      "months": c.calendar.as_ref().map(|cal| cal.months.clone()),
                      "zonal": ZONAL, "maps": c.maps.iter().map(|m| &m.0).collect::<Vec<_>>()});
    std::fs::write(dir.join("sums.json"), serde_json::to_string_pretty(&meta)?)
}

/// Load sums written by [`write`] into an accumulator with the same layout (restart).
pub fn read_into(c: &mut Climatology, dir: &Path) -> std::io::Result<()> {
    let p = dir.join("sums.json");
    if !p.exists() {
        return Ok(());
    }
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(p)?).map_err(std::io::Error::other)?;
    c.count = meta["count"].as_u64().unwrap_or(0);
    for (name, sums) in ZONAL.iter().zip(c.zonal.iter_mut()) {
        let (d, _) = super::npy::read::<f64>(&dir.join(format!("sums_zonal_{name}.npy")))
            .map_err(std::io::Error::other)?;
        if d.len() != sums.len() {
            return Err(std::io::Error::other("climatology shape mismatch"));
        }
        *sums = d;
    }
    if let Some(mc) = meta["monthly_count"].as_array() {
        if c.calendar.is_some() && mc.len() == c.monthly_count.len() {
            for (dst, v) in c.monthly_count.iter_mut().zip(mc) {
                *dst = v.as_u64().unwrap_or(0);
            }
            for (fi, (name, _, _)) in c.maps.iter().enumerate() {
                let (d, _) = super::npy::read::<f64>(&dir.join(format!("sums_monthly_{name}.npy")))
                    .map_err(std::io::Error::other)?;
                let per = c.monthly[fi][0].len();
                if d.len() != per * c.monthly[fi].len() {
                    return Err(std::io::Error::other("monthly climatology shape mismatch"));
                }
                for (m, chunk) in d.chunks(per).enumerate() {
                    c.monthly[fi][m].copy_from_slice(chunk);
                }
            }
        }
    }
    c.ps = super::npy::read::<f64>(&dir.join("sums_zonal_ps.npy"))
        .map_err(std::io::Error::other)?
        .0;
    for (name, _, sums) in c.maps.iter_mut() {
        let (d, _) = super::npy::read::<f64>(&dir.join(format!("sums_mean_{name}.npy")))
            .map_err(std::io::Error::other)?;
        if d.len() != sums.len() {
            return Err(std::io::Error::other("climatology map shape mismatch"));
        }
        *sums = d;
    }
    Ok(())
}
