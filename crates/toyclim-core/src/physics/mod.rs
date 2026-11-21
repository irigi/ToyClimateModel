//! Grid-point (column) physics (SPEC.md §9.3.3, §9.4).
//!
//! Every scheme implements [`column::ColumnScheme`] and updates the provisional column
//! state in order (sequential split). Two kinds of scheme lists exist:
//! - **idealized bundles** from `[forcing] type = ...` (Held–Suarez, tidally locked, TJ16
//!   moist Held–Suarez): frozen regression physics;
//! - **world physics** from the typed blocks of config schema 2 (`[radiation]`,
//!   `[boundary_layer]`, `[convection]`, `[condensation]`, `[clouds]`, `[dynamics.sponge]`),
//!   in the order of §9.3.3: radiation, surface fluxes + boundary layer + surface energy
//!   balance, dry adjustment, moist convection, large-scale condensation, sponge.

pub mod boundary_layer;
pub mod clouds;
pub mod column;
pub mod condensation;
pub mod convection;
pub mod damping;
pub mod driver;
pub mod idealized;
pub mod radiation;
pub mod surface_flux;

pub use column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
pub use condensation::Condensable;

use crate::config::{Duration, PhysicsConfig, SchemeConfig};
use crate::planet::Planet;
use boundary_layer::KProfile;
use condensation::{LargeScaleCondensation, PrecipitationFall};
use convection::{DryAdjustment, SimplifiedBettsMiller};
use damping::Sponge;
use idealized::held_suarez::{Damping, HeldSuarez, TidallyLocked};
use idealized::tj16::{SurfaceTemperature, Tj16BoundaryLayer, Tj16SurfaceFluxes};
use radiation::cloud_optics::CloudOpticsParams;
use radiation::GrayRadiation;
use surface_flux::ExchangeParams;

/// A list of schemes in call order.
pub type Schemes = Vec<Box<dyn ColumnScheme>>;

/// What the radiation needs from the star and the atmosphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiationEnv {
    /// Effective temperature of the star \[K\].
    pub t_eff: f64,
    /// CO₂ volume mixing ratio \[ppm\].
    pub co2_ppm: f64,
}

impl Default for RadiationEnv {
    /// The Sun and 400 ppm (cases without `[star]` / `[atmosphere]` blocks).
    fn default() -> Self {
        RadiationEnv {
            t_eff: 5772.0,
            co2_ppm: 400.0,
        }
    }
}

impl RadiationEnv {
    /// From the derived system information of a schema-2 config: the star's T_eff and the
    /// CO₂ of the composition (0 if the composition has none).
    pub fn from_system(sys: Option<&crate::config::v2::SystemInfo>) -> Self {
        let d = RadiationEnv::default();
        let Some(s) = sys else { return d };
        RadiationEnv {
            t_eff: s.star.t_eff,
            co2_ppm: s
                .composition
                .as_ref()
                .map_or(d.co2_ppm, |c| c.get("CO2").copied().unwrap_or(0.0) * 1e6),
        }
    }
}

fn array<const N: usize>(
    t: &toml::Table,
    key: &str,
    default: [f64; N],
) -> Result<[f64; N], String> {
    match t.get(key) {
        None => Ok(default),
        Some(toml::Value::Array(a)) if a.len() == N => {
            let mut out = [0.0; N];
            for (o, v) in out.iter_mut().zip(a) {
                *o = v
                    .as_float()
                    .or_else(|| v.as_integer().map(|i| i as f64))
                    .ok_or_else(|| format!("{key} must be an array of {N} numbers"))?;
            }
            Ok(out)
        }
        Some(_) => Err(format!("{key} must be an array of {N} numbers")),
    }
}

fn num(t: &toml::Table, key: &str, default: f64) -> Result<f64, String> {
    match t.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_float()
            .or_else(|| v.as_integer().map(|i| i as f64))
            .ok_or_else(|| format!("{key} must be a number")),
    }
}

fn boolean(t: &toml::Table, key: &str, default: bool) -> Result<bool, String> {
    match t.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_bool()
            .ok_or_else(|| format!("{key} must be true or false")),
    }
}

/// A time given as a duration string (`"40day"`) or seconds.
fn seconds(t: &toml::Table, key: &str, default: f64) -> Result<f64, String> {
    match t.get(key) {
        None => Ok(default),
        Some(toml::Value::String(s)) => s
            .parse::<Duration>()
            .map(|d| d.0)
            .map_err(|e| format!("{key}: {e}")),
        Some(_) => num(t, key, default),
    }
}

/// A rate given as an e-folding time (`"40day"` or seconds) → s⁻¹.
fn rate(t: &toml::Table, key: &str, default_rate: f64) -> Result<f64, String> {
    if t.get(key).is_none() {
        return Ok(default_rate);
    }
    let x = seconds(t, key, 0.0)?;
    Ok(if x > 0.0 { 1.0 / x } else { 0.0 })
}

fn check_keys(block: &str, t: &toml::Table, known: &[&str]) -> Result<(), String> {
    match t.keys().find(|k| !known.contains(&k.as_str())) {
        Some(k) => Err(format!("unknown key {k:?} in {block} (known: {known:?})")),
        None => Ok(()),
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

const HS_KEYS: &[&str] = &[
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
];
const TL_KEYS: &[&str] = &["t_night", "delta_td", "substellar_lon"];
const TJ16_KEYS: &[&str] = &[
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
];
const GRAY_FORCING_KEYS: &[&str] = &[
    "type",
    "tau_eq",
    "tau_pole",
    "latitude_dependence",
    "f_linear",
    "tau_q",
    "tau_sw",
    "albedo",
    "sst_uniform",
    "convection",
    "tau_convection",
    "rh_convection",
    "clouds",
    "cloud_rh_crit",
    "cloud_sw_reflect",
    "cloud_lw_tau",
];

/// `shallow_default`: Frierson's "shallower" shallow convection unless the table says
/// `shallow = "none"` (on for `[convection] scheme = "sbm"`, SPEC.md §9.4.4; off for the
/// idealized forcing bundles, which keep the prototype behaviour).
fn sbm(
    cond: &Condensable,
    t: &toml::Table,
    tau_key: &str,
    rh_key: &str,
    shallow_default: bool,
) -> Result<SimplifiedBettsMiller, String> {
    let shallow = match t.get("shallow") {
        None => shallow_default,
        Some(v) => match v.as_str() {
            Some("shallower") => true,
            Some("none") => false,
            _ => {
                return Err(format!(
                    "[convection] shallow = {v}: expected \"shallower\" or \"none\""
                ))
            }
        },
    };
    Ok(SimplifiedBettsMiller {
        cond: cond.clone(),
        tau: 1.0 / rate(t, tau_key, 1.0 / 7200.0)?,
        rh_ref: num(t, rh_key, 0.7)?,
        shallow,
        substeps: 1,
    })
}

/// Build the idealized bundle named in the `[forcing]` block (none → adiabatic).
///
/// `type = "held_suarez"`: optional `delta_ty`, `delta_theta_z`, `t_surface`, `t_min`,
/// `p0` and damping times `tau_friction` (1 day), `tau_free` (40 days), `tau_surface`
/// (4 days), `sigma_b` (0.7).
/// `type = "tidally_locked"`: as above plus `t_night` (250 K), `delta_td` (70 K),
/// `substellar_lon` (degrees, default 180).
///
/// `type = "moist_held_suarez"` (Thatcher & Jablonowski 2016): Held–Suarez with
/// `t_surface` = 294 K and `delta_ty` = 65 K by default, large-scale condensation,
/// surface fluxes (`c_h`, `c_e` = 0.0044; SST `sst_delta` 29 K, `sst_min` 271 K,
/// `sst_width` 26° or `sst_uniform`) and boundary-layer mixing (`p_pbl` 850 hPa,
/// `p_strato` 100 hPa), sequential in the paper's order. Needs a `[moisture]` block.
/// Optional `convection = "sbm"` (simplified Betts–Miller, `tau_convection` 2 h,
/// `rh_convection` 0.7; before the condensation, TJ16 Sect. 6.1).
///
/// `type = "gray_aquaplanet"` (schema 1): an alias for the world physics with gray
/// radiation (`tau_eq`, `tau_pole`, `f_linear`, `tau_q`, `tau_sw`, `albedo`, `sst_uniform`),
/// the K-profile boundary layer, SBM (`convection`, `tau_convection`, `rh_convection`) and the
/// experimental clouds (`clouds = true`, `cloud_rh_crit`, `cloud_sw_reflect`,
/// `cloud_lw_tau`). Needs a `[surface]`.
pub fn build(
    forcing: Option<&toml::Table>,
    planet: &Planet,
    moisture: Option<&Condensable>,
) -> Result<Schemes, String> {
    let Some(f) = forcing else {
        return Ok(Vec::new());
    };
    let kind = f.get("type").and_then(|v| v.as_str()).unwrap_or("none");
    let p0 = |f: &toml::Table| num(f, "p0", planet.p0);
    match kind {
        "none" => {
            check_keys("[forcing]", f, &["type"])?;
            Ok(Vec::new())
        }
        "held_suarez" => {
            check_keys("[forcing]", f, HS_KEYS)?;
            let d = HeldSuarez::default();
            Ok(vec![Box::new(HeldSuarez {
                damping: damping(f)?,
                delta_ty: num(f, "delta_ty", d.delta_ty)?,
                delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                t_surface: num(f, "t_surface", d.t_surface)?,
                t_min: num(f, "t_min", d.t_min)?,
                p0: p0(f)?,
            })])
        }
        "tidally_locked" => {
            check_keys("[forcing]", f, &[HS_KEYS, TL_KEYS].concat())?;
            let d = TidallyLocked::default();
            Ok(vec![Box::new(TidallyLocked {
                damping: damping(f)?,
                t_night: num(f, "t_night", d.t_night)?,
                delta_td: num(f, "delta_td", d.delta_td)?,
                delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                t_min: num(f, "t_min", d.t_min)?,
                p0: p0(f)?,
                substellar_lon: num(f, "substellar_lon", 180.0)?.to_radians(),
            })])
        }
        "moist_held_suarez" => {
            check_keys("[forcing]", f, &[HS_KEYS, TJ16_KEYS].concat())?;
            let cond = &moisture
                .ok_or("forcing moist_held_suarez needs a [moisture] block")?
                .without_fusion();
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
            let mut schemes: Schemes = Vec::new();
            match f.get("convection").and_then(|v| v.as_str()).unwrap_or("none") {
                "none" => {}
                "sbm" | "simplified_betts_miller" => {
                    schemes.push(Box::new(sbm(cond, f, "tau_convection", "rh_convection", false)?))
                }
                other => return Err(format!("unknown convection {other:?} (none, sbm)")),
            }
            schemes.push(Box::new(LargeScaleCondensation { cond: cond.clone() }));
            schemes.push(Box::new(Tj16SurfaceFluxes {
                cond: cond.clone(),
                sst,
                c_h: num(f, "c_h", 0.0044)?,
                c_e,
            }));
            schemes.push(Box::new(Tj16BoundaryLayer {
                c_e,
                p_pbl: num(f, "p_pbl", 85000.0)?,
                p_strato: num(f, "p_strato", 10000.0)?,
                p0: p0(f)?,
            }));
            schemes.push(Box::new(HeldSuarez {
                damping: damping(f)?,
                delta_ty: num(f, "delta_ty", 65.0)?,
                delta_theta_z: num(f, "delta_theta_z", d.delta_theta_z)?,
                t_surface: num(f, "t_surface", 294.0)?,
                t_min: num(f, "t_min", d.t_min)?,
                p0: p0(f)?,
            }));
            Ok(schemes)
        }
        "gray_aquaplanet" => {
            for k in f.keys() {
                if !GRAY_FORCING_KEYS.contains(&k.as_str()) {
                    return Err(format!(
                        "forcing key {k:?} is not used by gray_aquaplanet any more: the \
                         prototype TJ16 boundary layer and Rayleigh drag were replaced by \
                         the K-profile boundary layer in P1 (known keys: {GRAY_FORCING_KEYS:?})"
                    ));
                }
            }
            let cond = moisture.ok_or("forcing gray_aquaplanet needs a [moisture] block")?;
            let mut rad = toml::Table::new();
            // an Earth-like idealized bundle: the Earth-shaped τ₀(φ) unless switched off
            rad.insert(
                "latitude_dependence".into(),
                f.get("latitude_dependence")
                    .cloned()
                    .unwrap_or(toml::Value::Boolean(true)),
            );
            for (from, to) in [
                ("tau_eq", "tau_eq"),
                ("tau_pole", "tau_pole"),
                ("f_linear", "f_linear"),
                ("tau_q", "tau_q"),
                ("tau_sw", "tau_sw"),
                ("albedo", "albedo"),
                ("sst_uniform", "t_surface_default"),
            ] {
                if let Some(v) = f.get(from) {
                    rad.insert(to.into(), v.clone());
                }
            }
            let mut conv = toml::Table::new();
            for (from, to) in [("tau_convection", "tau"), ("rh_convection", "rh_ref")] {
                if let Some(v) = f.get(from) {
                    conv.insert(to.into(), v.clone());
                }
            }
            let mut clouds = toml::Table::new();
            for (from, to) in [
                ("cloud_rh_crit", "rh_crit"),
                ("cloud_sw_reflect", "sw_reflect"),
                ("cloud_lw_tau", "lw_tau"),
            ] {
                if let Some(v) = f.get(from) {
                    clouds.insert(to.into(), v.clone());
                }
            }
            let pc = PhysicsConfig {
                radiation: SchemeConfig::new("gray", rad),
                boundary_layer: SchemeConfig::new("kprofile", toml::Table::new()),
                convection: Some(SchemeConfig::new(
                    f.get("convection").and_then(|v| v.as_str()).unwrap_or("sbm"),
                    conv,
                )),
                condensation: None,
                clouds: boolean(f, "clouds", false)?
                    .then(|| SchemeConfig::new("rh_prototype", clouds)),
            };
            build_world(&pc, None, Some(cond), &RadiationEnv::default())
        }
        other => Err(format!(
            "unknown forcing type {other:?} (none, held_suarez, tidally_locked, moist_held_suarez, gray_aquaplanet)"
        )),
    }
}

const GRAY_KEYS: &[&str] = &[
    "reflectance",
    "tau_eq",
    "tau_pole",
    "latitude_dependence",
    "f_linear",
    "tau_q",
    "tau_sw",
    "albedo",
    "t_surface_default",
];
const KPROFILE_KEYS: &[&str] = &[
    "ri_crit",
    "surface_layer_fraction",
    "background_k",
    "frictional_heating",
    "gustiness",
    "charnock",
    "z0_ocean_min",
    "z0_land",
    "z0_ice",
    "z0h_ratio",
    "gust_unstable",
    "gust_stable",
];

/// The K-profile boundary layer of a `[boundary_layer] scheme = "kprofile"` block.
pub fn kprofile(t: &toml::Table) -> Result<KProfile, String> {
    check_keys("[boundary_layer]", t, KPROFILE_KEYS)?;
    let d = KProfile::default();
    let e = ExchangeParams::default();
    Ok(KProfile {
        exchange: ExchangeParams {
            gustiness: num(t, "gustiness", e.gustiness)?,
            charnock: num(t, "charnock", e.charnock)?,
            z0_ocean_min: num(t, "z0_ocean_min", e.z0_ocean_min)?,
            z0_land: num(t, "z0_land", e.z0_land)?,
            z0_ice: num(t, "z0_ice", e.z0_ice)?,
            z0h_ratio: num(t, "z0h_ratio", e.z0h_ratio)?,
            gust_unstable: num(t, "gust_unstable", e.gust_unstable)?,
            gust_stable: num(t, "gust_stable", e.gust_stable)?,
        },
        ri_crit: num(t, "ri_crit", d.ri_crit)?,
        surface_layer_fraction: num(t, "surface_layer_fraction", d.surface_layer_fraction)?,
        background_k: num(t, "background_k", d.background_k)?,
        frictional_heating: boolean(t, "frictional_heating", d.frictional_heating)?,
    })
}

/// The semi-gray opacities of a `[radiation] scheme = "semigray"` block.
pub fn semigray(p: &toml::Table, co2_ppm: f64) -> Result<radiation::semigray::SemiGray, String> {
    check_keys("[radiation]", p, radiation::semigray::KEYS)?;
    let d = radiation::semigray::SemiGray::default();
    let sg = radiation::semigray::SemiGray {
        vis_edge: num(p, "vis_edge", d.vis_edge)?,
        nir_weak_share: num(p, "nir_weak_share", d.nir_weak_share)?,
        rayleigh: array(p, "rayleigh", d.rayleigh)?,
        k_wv: array(p, "k_wv", d.k_wv)?,
        dry_abs: array(p, "dry_abs", d.dry_abs)?,
        wv_p_ref: num(p, "wv_p_ref", d.wv_p_ref)?,
        wv_n: num(p, "wv_n", d.wv_n)?,
        ozone_tau: num(p, "ozone_tau", d.ozone_tau)?,
        ozone_p: num(p, "ozone_p", d.ozone_p)?,
        ozone_width: num(p, "ozone_width", d.ozone_width)?,
        lw_rest: array(p, "lw_rest", d.lw_rest)?,
        lw_window: array(p, "lw_window", d.lw_window)?,
        lw_rest_strong: array(p, "lw_rest_strong", d.lw_rest_strong)?,
        lw_strong_share: num(p, "lw_strong_share", d.lw_strong_share)?,
        window: d.window,
        lw_n: num(p, "lw_n", d.lw_n)?,
        lw_p_ref: num(p, "lw_p_ref", d.lw_p_ref)?,
        lw_n_co2: num(p, "lw_n_co2", d.lw_n_co2)?,
        lw_co2_curvature: num(p, "lw_co2_curvature", d.lw_co2_curvature)?,
        co2_ppm,
        ..d
    };
    if !(0.0..=1.0).contains(&sg.lw_strong_share) {
        return Err("[radiation] lw_strong_share must be in [0, 1]".into());
    }
    if !(0.0..=1.0).contains(&sg.nir_weak_share) {
        return Err("[radiation] nir_weak_share must be in [0, 1]".into());
    }
    Ok(sg)
}

/// Build the world physics of the typed blocks (SPEC.md §9.3.2) in the order of §9.3.3.
/// `sponge`: the `[dynamics.sponge]` table if enabled.
pub fn build_world(
    pc: &PhysicsConfig,
    sponge: Option<&toml::Table>,
    moisture: Option<&Condensable>,
    env: &RadiationEnv,
) -> Result<Schemes, String> {
    let mut schemes: Schemes = Vec::new();
    // 1. radiation (with the experimental clouds inside the gray scheme)
    let mut diagnostic: Option<(clouds::DiagnosticClouds, CloudOpticsParams)> = None;
    let clouds = match &pc.clouds {
        None => None,
        Some(c) => match c.scheme.as_str() {
            "none" => None,
            "rh_prototype" => {
                check_keys("[clouds]", &c.params, &["rh_crit", "sw_reflect", "lw_tau"])?;
                let cond = moisture.ok_or("[clouds] needs a [moisture] block")?;
                Some(radiation::Clouds {
                    cond: cond.clone(),
                    rh_crit: num(&c.params, "rh_crit", 0.8)?,
                    sw_reflect: num(&c.params, "sw_reflect", 0.25)?,
                    lw_tau: num(&c.params, "lw_tau", 3.0)?,
                })
            }
            "diagnostic" => {
                diagnostic = Some(diagnostic_clouds(&c.params, moisture, env)?);
                None
            }
            other => {
                return Err(format!(
                    "unknown clouds scheme {other:?} (none, diagnostic, rh_prototype)"
                ))
            }
        },
    };
    let r = &pc.radiation;
    match r.scheme.as_str() {
        "gray" => {
            check_keys("[radiation]", &r.params, GRAY_KEYS)?;
            if diagnostic.is_some() {
                return Err(
                    "[clouds] scheme \"diagnostic\" needs [radiation] scheme \"semigray\"".into(),
                );
            }
            let d = GrayRadiation::default();
            schemes.push(Box::new(GrayRadiation {
                tau_eq: num(&r.params, "tau_eq", d.tau_eq)?,
                tau_pole: num(&r.params, "tau_pole", d.tau_pole)?,
                latitude_dependence: boolean(
                    &r.params,
                    "latitude_dependence",
                    d.latitude_dependence,
                )?,
                f_linear: num(&r.params, "f_linear", d.f_linear)?,
                tau_q: num(&r.params, "tau_q", d.tau_q)?,
                tau_sw: num(&r.params, "tau_sw", d.tau_sw)?,
                reflectance: num(&r.params, "reflectance", d.reflectance)?,
                albedo: num(&r.params, "albedo", d.albedo)?,
                t_surface_default: num(&r.params, "t_surface_default", d.t_surface_default)?,
                clouds,
            }))
        }
        "none" => {
            if clouds.is_some() || diagnostic.is_some() {
                return Err("[clouds] needs radiation".into());
            }
        }
        "semigray" => {
            if clouds.is_some() {
                return Err("the rh_prototype clouds work only inside the gray scheme".into());
            }
            let sg = semigray(&r.params, env.co2_ppm)?;
            let p = &r.params;
            let mut ts = radiation::TwoStream::new(Box::new(sg), env.t_eff);
            let a = num(p, "albedo", ts.albedo[0])?;
            ts.albedo = [a, a];
            ts.t_surface_default = num(p, "t_surface_default", ts.t_surface_default)?;
            ts.reflectance = num(p, "reflectance", 0.0)?;
            if let Some(a) = p.get("planetary_albedo") {
                let a = a
                    .as_float()
                    .ok_or("[radiation] planetary_albedo must be a number")?;
                if !(0.0..1.0).contains(&a) {
                    return Err("[radiation] planetary_albedo must be in [0, 1)".into());
                }
                ts.planetary_albedo = Some(a);
            }
            if !(0.0..1.0).contains(&ts.reflectance) {
                return Err("[radiation] reflectance must be in [0, 1)".into());
            }
            ts.clouds = diagnostic.as_ref().map(|d| d.1.clone());
            schemes.push(Box::new(ts));
        }
        other => {
            return Err(format!(
                "unknown radiation scheme {other:?} (gray, semigray, none)"
            ))
        }
    }
    // 2. surface fluxes + boundary layer + surface energy balance
    let b = &pc.boundary_layer;
    match b.scheme.as_str() {
        "kprofile" => schemes.push(Box::new(kprofile(&b.params)?)),
        "tj16" => {
            return Err(
                "boundary_layer scheme \"tj16\" (with Rayleigh drag) was replaced by \"kprofile\" \
                 in P1; the TJ16 scheme remains inside [forcing] type = \"moist_held_suarez\""
                    .into(),
            )
        }
        other => {
            return Err(format!(
                "unknown boundary_layer scheme {other:?} (kprofile)"
            ))
        }
    }
    // 3.–4. convection
    let conv = pc
        .convection
        .clone()
        .unwrap_or_else(|| SchemeConfig::new("dry", toml::Table::new()));
    match conv.scheme.as_str() {
        "none" => check_keys("[convection]", &conv.params, &[])?,
        "dry" => {
            check_keys("[convection]", &conv.params, &[])?;
            schemes.push(Box::new(DryAdjustment::default()));
        }
        "sbm" | "simplified_betts_miller" => {
            check_keys("[convection]", &conv.params, &["tau", "rh_ref", "shallow"])?;
            let cond = moisture.ok_or("convection \"sbm\" needs a [moisture] block")?;
            schemes.push(Box::new(DryAdjustment::default()));
            schemes.push(Box::new(sbm(cond, &conv.params, "tau", "rh_ref", true)?));
        }
        other => {
            return Err(format!(
                "unknown convection scheme {other:?} (sbm, dry, none)"
            ))
        }
    }
    // 5. large-scale condensation
    if let Some(c) = &pc.condensation {
        check_keys("[condensation]", c, &["reevaporation"])?;
        if boolean(c, "reevaporation", false)? {
            return Err("condensation.reevaporation is not implemented yet (P2)".into());
        }
    }
    if let Some(cond) = moisture {
        schemes.push(Box::new(LargeScaleCondensation { cond: cond.clone() }));
        schemes.push(Box::new(PrecipitationFall { cond: cond.clone() }));
    }
    // 6. cloud diagnosis (for the next radiation call)
    if let Some((d, _)) = diagnostic {
        schemes.push(Box::new(d));
    }
    // 7. sponge
    if let Some(s) = sponge {
        check_keys("[dynamics.sponge]", s, &["enabled", "sigma_top", "tau"])?;
        if boolean(s, "enabled", false)? {
            let d = Sponge::default();
            schemes.push(Box::new(Sponge {
                sigma_top: num(s, "sigma_top", d.sigma_top)?,
                tau: seconds(s, "tau", d.tau)?,
            }));
        }
    }
    Ok(schemes)
}

/// Knobs of `[clouds] scheme = "diagnostic"` (SPEC.md §9.4.7; docs/tuning.md).
pub const CLOUD_KEYS: &[&str] = &[
    "rh_crit_surface",
    "rh_crit_upper",
    "rh_crit_exponent",
    "conv_coef",
    "conv_max",
    "water_ratio",
    "water_max",
    "sigma_min",
    "freeze_dry_q",
    "freeze_dry_min",
    "freeze_dry_sigma",
    "r_liquid",
    "r_ice",
    "k_lw_liquid",
];

fn diagnostic_clouds(
    p: &toml::Table,
    moisture: Option<&Condensable>,
    env: &RadiationEnv,
) -> Result<(clouds::DiagnosticClouds, CloudOpticsParams), String> {
    check_keys("[clouds]", p, CLOUD_KEYS)?;
    let cond = moisture.ok_or("[clouds] needs a [moisture] block")?;
    let d = clouds::DiagnosticClouds::new(cond.clone());
    let c = clouds::DiagnosticClouds {
        rh_crit_surface: num(p, "rh_crit_surface", d.rh_crit_surface)?,
        rh_crit_upper: num(p, "rh_crit_upper", d.rh_crit_upper)?,
        rh_crit_exponent: num(p, "rh_crit_exponent", d.rh_crit_exponent)?,
        conv_coef: num(p, "conv_coef", d.conv_coef)?,
        conv_max: num(p, "conv_max", d.conv_max)?,
        water_ratio: num(p, "water_ratio", d.water_ratio)?,
        water_max: num(p, "water_max", d.water_max)?,
        sigma_min: num(p, "sigma_min", d.sigma_min)?,
        freeze_dry_q: num(p, "freeze_dry_q", d.freeze_dry_q)?,
        freeze_dry_min: num(p, "freeze_dry_min", d.freeze_dry_min)?,
        freeze_dry_sigma: num(p, "freeze_dry_sigma", d.freeze_dry_sigma)?,
        ..d
    };
    for (k, v) in [
        ("rh_crit_surface", c.rh_crit_surface),
        ("rh_crit_upper", c.rh_crit_upper),
    ] {
        if !(0.0..1.0).contains(&v) {
            return Err(format!("[clouds] {k} must be in [0, 1)"));
        }
    }
    for (k, v) in [
        ("conv_coef", c.conv_coef),
        ("conv_max", c.conv_max),
        ("water_ratio", c.water_ratio),
        ("water_max", c.water_max),
        ("freeze_dry_q", c.freeze_dry_q),
        ("freeze_dry_min", c.freeze_dry_min),
    ] {
        if v.is_nan() || v < 0.0 {
            return Err(format!("[clouds] {k} must be ≥ 0"));
        }
    }
    if c.conv_max > 1.0 {
        return Err("[clouds] conv_max must be ≤ 1".into());
    }
    let optics = CloudOpticsParams::new(
        num(p, "r_liquid", 10.0)?,
        num(p, "r_ice", 30.0)?,
        num(p, "k_lw_liquid", 0.090361)?,
        env.t_eff,
    );
    Ok((c, optics))
}

/// Whether `schemes` couple to a surface model (the K-profile surface solve).
pub fn needs_surface(schemes: &Schemes) -> bool {
    schemes
        .iter()
        .any(|s| s.name() == "kprofile_boundary_layer")
}
