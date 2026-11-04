//! Grid-point (column) physics (SPEC.md §5.4).

pub mod column;
pub mod held_suarez;

pub use column::{ColumnIn, ColumnOut, ColumnPhysics};

use crate::config::Duration;
use crate::planet::Planet;
use held_suarez::{Damping, HeldSuarez, TidallyLocked};

fn num(t: &toml::Table, key: &str, default: f64) -> Result<f64, String> {
    match t.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_float()
            .or_else(|| v.as_integer().map(|i| i as f64))
            .ok_or_else(|| format!("forcing.{key} must be a number")),
    }
}

/// A rate given as an e-folding time (`"40day"` or seconds) → s⁻¹.
fn rate(t: &toml::Table, key: &str, default_rate: f64) -> Result<f64, String> {
    match t.get(key) {
        None => Ok(default_rate),
        Some(toml::Value::String(s)) => {
            let d: Duration = s
                .parse()
                .map_err(|e: crate::config::ConfigError| e.to_string())?;
            Ok(if d.0 > 0.0 { 1.0 / d.0 } else { 0.0 })
        }
        Some(v) => {
            let x = v
                .as_float()
                .or_else(|| v.as_integer().map(|i| i as f64))
                .ok_or_else(|| format!("forcing.{key}"))?;
            Ok(if x > 0.0 { 1.0 / x } else { 0.0 })
        }
    }
}

fn damping(t: &toml::Table) -> Result<Damping, String> {
    let d = Damping::default();
    Ok(Damping {
        kf: rate(t, "tau_friction", d.kf)?,
        ka: rate(t, "tau_free", d.ka)?,
        ks: rate(t, "tau_surface", d.ks)?,
        sigma_b: num(t, "sigma_b", d.sigma_b)?,
    })
}

const KNOWN: &[&str] = &[
    "type",
    "tau_friction",
    "tau_free",
    "tau_surface",
    "sigma_b",
    "delta_ty",
    "delta_theta_z",
    "t_surface",
    "t_min",
    "p0",
    "t_night",
    "delta_td",
    "substellar_lon",
];

/// Build the column physics named in the `[forcing]` block (none → adiabatic).
///
/// `type = "held_suarez"`: optional `delta_ty`, `delta_theta_z`, `t_surface`, `t_min`,
/// `p0` and damping times `tau_friction` (1 day), `tau_free` (40 days), `tau_surface`
/// (4 days), `sigma_b` (0.7).
/// `type = "tidally_locked"`: as above plus `t_night` (250 K), `delta_td` (70 K),
/// `substellar_lon` (degrees, default 180).
pub fn build(
    forcing: Option<&toml::Table>,
    planet: &Planet,
) -> Result<Vec<Box<dyn ColumnPhysics>>, String> {
    let Some(f) = forcing else {
        return Ok(Vec::new());
    };
    if let Some(k) = f.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Err(format!("unknown forcing key {k:?}"));
    }
    let kind = f.get("type").and_then(|v| v.as_str()).unwrap_or("none");
    match kind {
        "none" => Ok(Vec::new()),
        "held_suarez" => {
            let d = HeldSuarez::default();
            Ok(vec![Box::new(HeldSuarez {
                damping: damping(f)?,
                delta_ty: num(f, "delta_ty", d.delta_ty)?,
                delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                t_surface: num(f, "t_surface", d.t_surface)?,
                t_min: num(f, "t_min", d.t_min)?,
                p0: num(f, "p0", planet.p0)?,
            })])
        }
        "tidally_locked" => {
            let d = TidallyLocked::default();
            Ok(vec![Box::new(TidallyLocked {
                damping: damping(f)?,
                t_night: num(f, "t_night", d.t_night)?,
                delta_td: num(f, "delta_td", d.delta_td)?,
                delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                t_min: num(f, "t_min", d.t_min)?,
                p0: num(f, "p0", planet.p0)?,
                substellar_lon: num(f, "substellar_lon", 180.0)?.to_radians(),
            })])
        }
        other => Err(format!(
            "unknown forcing type {other:?} (none, held_suarez, tidally_locked)"
        )),
    }
}
