//! `[clouds] scheme = "diagnostic"`: knobs and their validation (docs/tuning.md).

use super::{ConvectiveCloud, DiagnosticClouds, LowCloud, LowRh};
use crate::physics::condensation::Condensable;
use crate::physics::radiation::cloud_optics::CloudOpticsParams;
use crate::physics::{boolean, check_keys, num, RadiationEnv};

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
    "cre_split",
    "cre_split_low_sigma",
    "convective",
    "core_share",
    "deep_sigma",
    "shallow_sigma",
    "anvil_water",
    "conv_water_path",
    "low_cloud",
    "low_sigma",
    "low_rh",
    "low_rh_crit",
    "low_omega",
    "inv_pressure",
    "inv_slope",
    "inv_offset",
];

/// Knobs that only `low_cloud = "slingo"` reads.
const SLINGO_LOW_KEYS: &[&str] = &[
    "low_sigma",
    "low_rh",
    "low_rh_crit",
    "low_omega",
    "inv_pressure",
    "inv_slope",
    "inv_offset",
];

/// A string option of `[clouds]` from `choices` (name, value); the first is the default.
fn choice<T: Copy>(p: &toml::Table, key: &str, choices: &[(&str, T)]) -> Result<T, String> {
    match p.get(key) {
        None => Ok(choices[0].1),
        Some(v) => v
            .as_str()
            .and_then(|s| choices.iter().find(|c| c.0 == s))
            .map(|c| c.1)
            .ok_or_else(|| {
                let names: Vec<String> = choices.iter().map(|c| format!("\"{}\"", c.0)).collect();
                format!("[clouds] {key} = {v}: expected {}", names.join(" or "))
            }),
    }
}

pub(crate) fn diagnostic_clouds(
    p: &toml::Table,
    moisture: Option<&Condensable>,
    env: &RadiationEnv,
) -> Result<(DiagnosticClouds, CloudOpticsParams), String> {
    check_keys("[clouds]", p, CLOUD_KEYS)?;
    let cond = moisture.ok_or("[clouds] needs a [moisture] block")?;
    let d = DiagnosticClouds::new(cond.clone());
    let c = DiagnosticClouds {
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
        cre_split: boolean(p, "cre_split", false)?,
        cre_split_low_sigma: num(p, "cre_split_low_sigma", 0.0)?,
        convective: match p.get("convective") {
            None => ConvectiveCloud::Column,
            Some(v) => match v.as_str() {
                Some("column") => ConvectiveCloud::Column,
                Some("anvil") => ConvectiveCloud::Anvil,
                _ => {
                    return Err(format!(
                        "[clouds] convective = {v}: expected \"column\" or \"anvil\""
                    ))
                }
            },
        },
        core_share: num(p, "core_share", d.core_share)?,
        deep_sigma: num(p, "deep_sigma", d.deep_sigma)?,
        shallow_sigma: num(p, "shallow_sigma", d.shallow_sigma)?,
        anvil_water: num(p, "anvil_water", d.anvil_water)?,
        conv_water_path: num(p, "conv_water_path", d.conv_water_path)?,
        low_cloud: choice(
            p,
            "low_cloud",
            &[("rh", LowCloud::Rh), ("slingo", LowCloud::Slingo)],
        )?,
        low_sigma: num(p, "low_sigma", d.low_sigma)?,
        low_rh: choice(
            p,
            "low_rh",
            &[("slingo", LowRh::Slingo), ("sundqvist", LowRh::Sundqvist)],
        )?,
        low_rh_crit: num(p, "low_rh_crit", d.low_rh_crit)?,
        low_omega: num(p, "low_omega", d.low_omega)?,
        inv_pressure: num(p, "inv_pressure", d.inv_pressure)?,
        inv_slope: num(p, "inv_slope", d.inv_slope)?,
        inv_offset: num(p, "inv_offset", d.inv_offset)?,
        ..d
    };
    if c.low_cloud != LowCloud::Slingo {
        if let Some(k) = SLINGO_LOW_KEYS.iter().find(|k| p.contains_key(**k)) {
            return Err(format!("[clouds] {k} needs low_cloud = \"slingo\""));
        }
    }
    if !(0.0..=1.0).contains(&c.low_sigma) || !(0.0..1.0).contains(&c.low_rh_crit) {
        return Err("[clouds] low_sigma must be in [0, 1] and low_rh_crit in [0, 1)".into());
    }
    for (k, v) in [
        ("low_omega", c.low_omega),
        ("inv_pressure", c.inv_pressure),
        ("inv_slope", c.inv_slope),
    ] {
        if v.is_nan() || v <= 0.0 {
            return Err(format!("[clouds] {k} must be > 0"));
        }
    }
    if c.inv_offset.is_nan() {
        return Err("[clouds] inv_offset must be a number".into());
    }
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
    for (k, v) in [
        ("core_share", c.core_share),
        ("deep_sigma", c.deep_sigma),
        ("shallow_sigma", c.shallow_sigma),
    ] {
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("[clouds] {k} must be in [0, 1]"));
        }
    }
    for (k, v) in [
        ("anvil_water", c.anvil_water),
        ("conv_water_path", c.conv_water_path),
    ] {
        if v.is_nan() || v < 0.0 {
            return Err(format!("[clouds] {k} must be ≥ 0"));
        }
    }
    if !(0.0..=1.0).contains(&c.cre_split_low_sigma) {
        return Err("[clouds] cre_split_low_sigma must be in [0, 1]".into());
    }
    if c.cre_split_low_sigma > 0.0 && !c.cre_split {
        return Err("[clouds] cre_split_low_sigma needs cre_split = true".into());
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
