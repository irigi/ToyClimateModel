//! Grid-point (column) physics (SPEC.md §5.4).

pub mod column;
pub mod convection;
pub mod held_suarez;
pub mod moist;
pub mod radiation;

pub use column::{ColumnIn, ColumnOut, ColumnPhysics};

use crate::config::Duration;
use crate::planet::Planet;
use convection::SimplifiedBettsMiller;
use held_suarez::{Damping, HeldSuarez, TidallyLocked};
use moist::{
    BoundaryLayer, Condensable, LargeScaleCondensation, SurfaceFluxes, SurfaceTemperature,
};
use radiation::{GrayRadiation, RayleighFriction};

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
    "c_h",
    "c_e",
    "sst_delta",
    "sst_min",
    "sst_width",
    "sst_uniform",
    "p_pbl",
    "p_strato",
    "convection",
    "tau_convection",
    "rh_convection",
    "tau_eq",
    "tau_pole",
    "f_linear",
    "tau_q",
    "tau_sw",
    "albedo",
    "clouds",
    "cloud_rh_crit",
    "cloud_sw_reflect",
    "cloud_lw_tau",
];

/// Build the column physics named in the `[forcing]` block (none → adiabatic).
///
/// `type = "held_suarez"`: optional `delta_ty`, `delta_theta_z`, `t_surface`, `t_min`,
/// `p0` and damping times `tau_friction` (1 day), `tau_free` (40 days), `tau_surface`
/// (4 days), `sigma_b` (0.7).
/// `type = "tidally_locked"`: as above plus `t_night` (250 K), `delta_td` (70 K),
/// `substellar_lon` (degrees, default 180).
///
/// `type = "moist_held_suarez"` (Thatcher & Jablonowski 2016): Held–Suarez with
/// `t_surface` = 294 K and `delta_ty` = 65 K by default, plus large-scale condensation,
/// surface fluxes (`c_h`, `c_e` = 0.0044; SST `sst_delta` 29 K, `sst_min` 271 K,
/// `sst_width` 26° or `sst_uniform`) and boundary-layer mixing (`p_pbl` 850 hPa,
/// `p_strato` 100 hPa). Needs a `[moisture]` block. Optional `convection = "sbm"`
/// (simplified Betts–Miller, `tau_convection` 2 h, `rh_convection` 0.7; not in TJ16).
///
/// `type = "gray_aquaplanet"` (Frierson et al. 2006 style, M8): gray radiation (`tau_eq` 6,
/// `tau_pole` 1.5, `f_linear` 0.1, `tau_q` 0, `tau_sw` 0.22, `albedo` 0.3) driven by the
/// `[orbit]` insolation, moist physics as above with SBM convection by default
/// (`convection = "none"` to disable), boundary-layer Rayleigh drag. Use with a
/// `[surface]` slab ocean; without one the surface is held at `sst_uniform` (288 K) for
/// radiation and at the TJ16 SST for the surface fluxes. Experimental diagnostic clouds:
/// `clouds = true` (`cloud_rh_crit` 0.8, `cloud_sw_reflect` 0.25, `cloud_lw_tau` 3); then
/// use an ocean-like `albedo` (≈ 0.1) since clouds provide the planetary albedo.
pub fn build(
    forcing: Option<&toml::Table>,
    planet: &Planet,
    moisture: Option<&Condensable>,
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
        "moist_held_suarez" => {
            let cond = moisture
                .ok_or("forcing moist_held_suarez needs a [moisture] block")?
                .clone();
            let d = HeldSuarez::default();
            let c_e = num(f, "c_e", 0.0044)?;
            let sst = match f.get("sst_uniform") {
                Some(_) => SurfaceTemperature::Uniform(num(f, "sst_uniform", 300.0)?),
                None => SurfaceTemperature::Gaussian {
                    delta_t: num(f, "sst_delta", 29.0)?,
                    t_min: num(f, "sst_min", 271.0)?,
                    width: num(f, "sst_width", 26.0)?.to_radians(),
                },
            };
            let convection = f
                .get("convection")
                .and_then(|v| v.as_str())
                .unwrap_or("none");
            let mut schemes: Vec<Box<dyn ColumnPhysics>> = vec![
                Box::new(HeldSuarez {
                    damping: damping(f)?,
                    delta_ty: num(f, "delta_ty", 65.0)?,
                    delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                    t_surface: num(f, "t_surface", 294.0)?,
                    t_min: num(f, "t_min", d.t_min)?,
                    p0: num(f, "p0", planet.p0)?,
                }),
                Box::new(LargeScaleCondensation { cond: cond.clone() }),
                Box::new(SurfaceFluxes {
                    cond,
                    sst,
                    c_h: num(f, "c_h", 0.0044)?,
                    c_e,
                }),
                Box::new(BoundaryLayer {
                    c_e,
                    p_pbl: num(f, "p_pbl", 85000.0)?,
                    p_strato: num(f, "p_strato", 10000.0)?,
                    p0: num(f, "p0", planet.p0)?,
                }),
            ];
            match convection {
                "none" => {}
                "sbm" | "simplified_betts_miller" => {
                    schemes.push(Box::new(SimplifiedBettsMiller {
                        cond: moisture.unwrap().clone(),
                        tau: 1.0 / rate(f, "tau_convection", 1.0 / 7200.0)?,
                        rh_ref: num(f, "rh_convection", 0.7)?,
                    substeps: 1,
                    }))
                }
                other => return Err(format!("unknown convection {other:?} (none, sbm)")),
            }
            Ok(schemes)
        }
        "gray_aquaplanet" => {
            let cond = moisture
                .ok_or("forcing gray_aquaplanet needs a [moisture] block")?
                .clone();
            let c_e = num(f, "c_e", 0.0044)?;
            let rd = GrayRadiation::default();
            let mut schemes: Vec<Box<dyn ColumnPhysics>> = vec![
                Box::new(GrayRadiation {
                    tau_eq: num(f, "tau_eq", rd.tau_eq)?,
                    tau_pole: num(f, "tau_pole", rd.tau_pole)?,
                    f_linear: num(f, "f_linear", rd.f_linear)?,
                    tau_q: num(f, "tau_q", rd.tau_q)?,
                    tau_sw: num(f, "tau_sw", rd.tau_sw)?,
                    albedo: num(f, "albedo", rd.albedo)?,
                    t_surface_default: num(f, "sst_uniform", rd.t_surface_default)?,
                    clouds: match f.get("clouds").and_then(|v| v.as_bool()).unwrap_or(false) {
                        false => None,
                        true => Some(radiation::Clouds {
                            cond: cond.clone(),
                            rh_crit: num(f, "cloud_rh_crit", 0.8)?,
                            sw_reflect: num(f, "cloud_sw_reflect", 0.25)?,
                            lw_tau: num(f, "cloud_lw_tau", 3.0)?,
                        }),
                    },
                }),
                Box::new(LargeScaleCondensation { cond: cond.clone() }),
                Box::new(SurfaceFluxes {
                    cond: cond.clone(),
                    sst: SurfaceTemperature::tj16(),
                    c_h: num(f, "c_h", 0.0044)?,
                    c_e,
                }),
                Box::new(BoundaryLayer {
                    c_e,
                    p_pbl: num(f, "p_pbl", 85000.0)?,
                    p_strato: num(f, "p_strato", 10000.0)?,
                    p0: num(f, "p0", planet.p0)?,
                }),
                Box::new(RayleighFriction {
                    kf: rate(f, "tau_friction", 1.0 / 86400.0)?,
                    sigma_b: num(f, "sigma_b", 0.7)?,
                }),
            ];
            match f.get("convection").and_then(|v| v.as_str()).unwrap_or("sbm") {
                "none" => {}
                "sbm" | "simplified_betts_miller" => schemes.push(Box::new(SimplifiedBettsMiller {
                    cond,
                    tau: 1.0 / rate(f, "tau_convection", 1.0 / 7200.0)?,
                    rh_ref: num(f, "rh_convection", 0.7)?,
                    substeps: 1,
                })),
                other => return Err(format!("unknown convection {other:?} (none, sbm)")),
            }
            Ok(schemes)
        }
        other => Err(format!(
            "unknown forcing type {other:?} (none, held_suarez, tidally_locked, moist_held_suarez, gray_aquaplanet)"
        )),
    }
}
