//! Config schema 2 (SPEC.md §9.3.2): star, orbit and atmosphere as data with derived
//! instellation, year and rotation; world physics as typed blocks.
//!
//! A schema-2 file is parsed into [`ConfigV2`] (every block rejects unknown keys) and then
//! *lowered* to the internal representation ([`super::Config`], the schema-1 layout), so
//! that the model code is shared and migrated cases give identical results. Derived
//! quantities and the inputs they came from are kept in [`SystemInfo`] (`config.system`
//! in `meta.json`).
//!
//! The typed physics blocks are passed to [`crate::physics::build_world`], which checks
//! the scheme names and parameters; schemes of later milestones (`semigray`, `diagnostic`
//! clouds, re-evaporation) are rejected there with a message until they are implemented.

use super::{ConfigError, Duration, ModelKind, SchemeConfig};
use crate::primitive::vertical::L26_HALF;
use crate::system::derive::{self, Spin};
use crate::system::star::Star;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use toml::{Table, Value};

/// `levels = 20` or a named set `"L26"`, `"L20"`, `"L10"`.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LevelsSpec {
    Count(usize),
    Named(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetV2 {
    pub preset: Option<String>,
    pub name: Option<String>,
    pub radius: Option<f64>,
    pub gravity: Option<f64>,
    /// Sidereal rotation period, e.g. "23.9345h"; a leading "-" means retrograde.
    pub rotation_period: Option<String>,
    pub rotation_rate: Option<f64>,
    pub rotation_factor: Option<f64>,
    pub gas_constant: Option<f64>,
    pub cp: Option<f64>,
    pub p0: Option<f64>,
    pub t_ref: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbitV2 {
    /// \[AU\]
    pub semi_major_axis: f64,
    #[serde(default)]
    pub eccentricity: f64,
    /// \[degrees\]
    #[serde(default)]
    pub obliquity: f64,
    #[serde(default)]
    pub perihelion_longitude: f64,
    #[serde(default)]
    pub start_solar_longitude: f64,
    /// "free" | "synchronous" | "resonant"
    #[serde(default = "default_spin")]
    pub spin: String,
    /// [p, q]: p rotations per q orbits (spin = "resonant").
    pub resonance: Option<[u32; 2]>,
    /// Substellar longitude at t = 0 (fixed for a synchronous circular orbit) \[degrees\].
    pub substellar_lon: Option<f64>,
    pub diurnal: Option<bool>,
    #[serde(default)]
    pub months: Vec<String>,
    /// Explicit overrides, checked against the derived values.
    pub instellation: Option<f64>,
    pub year: Option<Duration>,
    #[serde(default)]
    pub allow_inconsistent: bool,
}

fn default_spin() -> String {
    "free".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtmosphereV2 {
    /// Global-mean dry surface pressure \[Pa\].
    pub surface_pressure: f64,
    /// Volume fractions by gas.
    pub composition: BTreeMap<String, f64>,
    pub gas_constant: Option<f64>,
    pub cp: Option<f64>,
    #[serde(default)]
    pub allow_inconsistent: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceV2 {
    /// "aquaplanet" or a heightmap path.
    #[serde(default = "default_geography")]
    pub geography: String,
    pub orography_filter: Option<f64>,
    pub ocean: Option<Table>,
    pub land: Option<Table>,
    pub sea_ice: Option<Table>,
}

fn default_geography() -> String {
    "aquaplanet".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicsV2 {
    #[serde(default)]
    pub energy_fixer: bool,
    pub sponge: Option<Table>,
    pub tracer_transport: Option<String>,
}

/// A schema-2 configuration as written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV2 {
    pub schema: u32,
    pub description: Option<String>,
    pub model: ModelKind,
    pub truncation: usize,
    pub levels: Option<LevelsSpec>,
    pub sigma_half: Option<Vec<f64>>,
    pub dt: Option<Duration>,
    pub length: Duration,
    pub output_every: Duration,
    pub diagnostics_every: Option<Value>,
    pub output_fields: Option<Value>,
    pub initial_condition: Option<Value>,
    pub planet: Option<PlanetV2>,
    pub star: Option<Star>,
    pub orbit: Option<OrbitV2>,
    pub atmosphere: Option<AtmosphereV2>,
    pub moisture: Option<Value>,
    pub forcing: Option<Table>,
    pub radiation: Option<SchemeConfig>,
    pub boundary_layer: Option<SchemeConfig>,
    pub convection: Option<SchemeConfig>,
    pub condensation: Option<Table>,
    pub clouds: Option<SchemeConfig>,
    pub surface: Option<SurfaceV2>,
    pub dynamics: Option<DynamicsV2>,
    pub diffusion: Option<Value>,
    pub time_filter: Option<Value>,
    pub climatology: Option<Value>,
    pub mass_fixer: Option<bool>,
    pub stations: Option<Value>,
    pub station_every: Option<Value>,
    pub station_fields: Option<Value>,
    pub scm: Option<Value>,
}

/// Derived system quantities and the inputs they came from (echoed in `meta.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemInfo {
    pub star: Star,
    /// \[AU\]
    pub semi_major_axis: f64,
    pub spin: String,
    /// Instellation used \[W m⁻²\] and the value derived from star and orbit.
    pub instellation: f64,
    pub instellation_derived: f64,
    /// Orbital period used \[s\] and the derived value.
    pub year: f64,
    pub year_derived: f64,
    /// Rotation rate used \[s⁻¹\]; the derived value for synchronous or resonant spin.
    pub rotation_rate: f64,
    pub rotation_rate_derived: Option<f64>,
    /// Atmosphere: volume fractions and the mixture's R_d, c_p (before overrides).
    pub composition: Option<BTreeMap<String, f64>>,
    pub gas_constant_derived: Option<f64>,
    pub cp_derived: Option<f64>,
}

fn invalid(m: impl Into<String>) -> ConfigError {
    ConfigError::Invalid(m.into())
}

/// Parse a signed duration ("-23.9345h" → −86164.2 s).
fn signed_duration(s: &str) -> Result<f64, ConfigError> {
    let t = s.trim();
    match t.strip_prefix('-') {
        Some(rest) => Ok(-rest.parse::<Duration>()?.0),
        None => Ok(t.parse::<Duration>()?.0),
    }
}

fn put(t: &mut Table, key: &str, v: impl Into<Value>) {
    t.insert(key.to_string(), v.into());
}

/// Copy the parameters of a typed block into the forcing table, renamed per `map`
/// (block key → forcing key). Unknown parameters are an error.
fn copy_params(
    block: &str,
    src: &Table,
    map: &[(&str, &str)],
    dst: &mut Table,
) -> Result<(), ConfigError> {
    for (k, v) in src {
        let (_, to) = map
            .iter()
            .find(|(from, _)| from == k)
            .ok_or_else(|| invalid(format!("unknown key {k:?} in [{block}]")))?;
        dst.insert(to.to_string(), v.clone());
    }
    Ok(())
}

impl ConfigV2 {
    /// Lower to the internal (schema-1) table and the derived system information.
    pub fn lower(&self, raw: &Table) -> Result<(Table, Option<SystemInfo>), ConfigError> {
        if self.schema != 2 {
            return Err(invalid(format!("unsupported schema {}", self.schema)));
        }
        let mut t = Table::new();
        for key in [
            "description",
            "model",
            "truncation",
            "sigma_half",
            "length",
            "output_every",
            "diagnostics_every",
            "output_fields",
            "initial_condition",
            "moisture",
            "diffusion",
            "time_filter",
            "climatology",
            "mass_fixer",
            "stations",
            "station_every",
            "station_fields",
            "scm",
        ] {
            if let Some(v) = raw.get(key) {
                t.insert(key.into(), v.clone());
            }
        }
        self.lower_levels(&mut t)?;
        match (&self.dt, raw.get("dt")) {
            (Some(_), Some(v)) => put(&mut t, "dt", v.clone()),
            _ => match self.truncation {
                42 => put(&mut t, "dt", "15min"),
                21 => put(&mut t, "dt", "30min"),
                m => return Err(invalid(format!("dt is required at T{m}"))),
            },
        }
        let (planet, system) = self.lower_planet_orbit(&mut t)?;
        t.insert("planet".into(), Value::Table(planet));
        physics::lower_physics(self, &mut t)?;
        if let Some(s) = &self.surface {
            t.insert("surface".into(), Value::Table(physics::lower_surface(s)?));
        }
        if let Some(d) = &self.dynamics {
            let mut dt = Table::new();
            put(&mut dt, "energy_fixer", d.energy_fixer);
            if let Some(tt) = &d.tracer_transport {
                put(&mut dt, "tracer_transport", tt.as_str());
            }
            if let Some(sp) = &d.sponge {
                put(&mut dt, "sponge", Value::Table(sp.clone()));
            }
            t.insert("dynamics".into(), Value::Table(dt));
        }
        Ok((t, system))
    }

    fn lower_levels(&self, t: &mut Table) -> Result<(), ConfigError> {
        match &self.levels {
            None => {}
            Some(LevelsSpec::Count(n)) => put(t, "levels", *n as i64),
            Some(LevelsSpec::Named(name)) => match name.as_str() {
                "L26" => {
                    if self.sigma_half.is_some() {
                        return Err(invalid("give either levels = \"L26\" or sigma_half"));
                    }
                    let v: Vec<Value> = L26_HALF.iter().map(|&x| Value::Float(x)).collect();
                    put(t, "sigma_half", Value::Array(v));
                }
                "L20" => put(t, "levels", 20),
                "L10" => put(t, "levels", 10),
                other => {
                    return Err(invalid(format!(
                        "unknown level set {other:?} (L26, L20, L10)"
                    )))
                }
            },
        }
        Ok(())
    }

    fn lower_planet_orbit(
        &self,
        t: &mut Table,
    ) -> Result<(Table, Option<SystemInfo>), ConfigError> {
        let mut planet = Table::new();
        let p = self.planet.clone().unwrap_or(PlanetV2 {
            preset: None,
            name: None,
            radius: None,
            gravity: None,
            rotation_period: None,
            rotation_rate: None,
            rotation_factor: None,
            gas_constant: None,
            cp: None,
            p0: None,
            t_ref: None,
        });
        for (k, v) in [
            ("radius", p.radius),
            ("gravity", p.gravity),
            ("rotation_factor", p.rotation_factor),
            ("gas_constant", p.gas_constant),
            ("cp", p.cp),
            ("p0", p.p0),
            ("t_ref", p.t_ref),
        ] {
            if let Some(x) = v {
                put(&mut planet, k, x);
            }
        }
        if let Some(s) = &p.preset {
            put(&mut planet, "preset", s.as_str());
        }
        if let Some(s) = &p.name {
            put(&mut planet, "name", s.as_str());
        }
        let explicit_rate = match (&p.rotation_period, p.rotation_rate) {
            (Some(_), Some(_)) => {
                return Err(invalid("give either rotation_period or rotation_rate"))
            }
            (Some(s), None) => {
                let period = signed_duration(s)?;
                if period == 0.0 {
                    return Err(invalid("rotation_period must not be zero"));
                }
                Some(TAU / period)
            }
            (None, r) => r,
        };
        // Atmosphere → R_d, c_p, p0.
        let mut composition = None;
        let (mut r_derived, mut cp_derived) = (None, None);
        if let Some(a) = &self.atmosphere {
            if p.gas_constant.is_some() || p.cp.is_some() || p.p0.is_some() {
                return Err(invalid(
                    "with [atmosphere], give gas_constant, cp and the pressure there, not in [planet]",
                ));
            }
            let mix = crate::atmosphere::mixture(&a.composition).map_err(invalid)?;
            let r = derive::checked_override(
                "atmosphere.gas_constant",
                mix.gas_constant,
                a.gas_constant,
                a.allow_inconsistent,
            )
            .map_err(invalid)?;
            let cp = derive::checked_override("atmosphere.cp", mix.cp, a.cp, a.allow_inconsistent)
                .map_err(invalid)?;
            put(&mut planet, "gas_constant", r);
            put(&mut planet, "cp", cp);
            put(&mut planet, "p0", a.surface_pressure);
            composition = Some(a.composition.clone());
            (r_derived, cp_derived) = (Some(mix.gas_constant), Some(mix.cp));
        }
        let Some(o) = &self.orbit else {
            if self.star.is_some() {
                return Err(invalid("[star] needs an [orbit] block"));
            }
            if let Some(r) = explicit_rate {
                put(&mut planet, "rotation_rate", r);
            }
            return Ok((planet, None));
        };
        let star = self.star.unwrap_or_else(Star::sun);
        star.validate().map_err(invalid)?;
        if !(o.semi_major_axis.is_finite() && o.semi_major_axis > 0.0) {
            return Err(invalid("orbit.semi_major_axis must be > 0"));
        }
        let s_derived = derive::instellation(&star, o.semi_major_axis);
        let p_derived = derive::orbital_period(&star, o.semi_major_axis);
        let ai = o.allow_inconsistent;
        let instellation =
            derive::checked_override("orbit.instellation", s_derived, o.instellation, ai)
                .map_err(invalid)?;
        let year = derive::checked_override("orbit.year", p_derived, o.year.map(|d| d.0), ai)
            .map_err(invalid)?;
        let spin = match (o.spin.as_str(), o.resonance) {
            ("free", None) => Spin::Free,
            ("synchronous", None) => Spin::Synchronous,
            ("resonant", Some([p, q])) if p > 0 && q > 0 => Spin::Resonant { p, q },
            ("resonant", _) => return Err(invalid("spin = \"resonant\" needs resonance = [p, q]")),
            (_, Some(_)) => return Err(invalid("resonance is only used with spin = \"resonant\"")),
            (other, None) => {
                return Err(invalid(format!(
                    "unknown spin {other:?} (free, synchronous, resonant)"
                )))
            }
        };
        let rate_derived = spin.rotation_rate(year);
        let rotation_rate = match rate_derived {
            None => explicit_rate,
            Some(d) => {
                if p.rotation_factor.is_some() {
                    return Err(invalid("rotation_factor cannot be used with a locked spin"));
                }
                Some(
                    derive::checked_override("planet rotation", d, explicit_rate, ai)
                        .map_err(invalid)?,
                )
            }
        };
        if let Some(r) = rotation_rate {
            put(&mut planet, "rotation_rate", r);
        }
        // Circular synchronous orbit: the substellar point is fixed. Otherwise it moves
        // at the synodic rate (a synchronous rotator on an eccentric orbit librates).
        let locked = spin == Spin::Synchronous && o.eccentricity == 0.0;
        let mut orbit = Table::new();
        put(&mut orbit, "year", year);
        put(&mut orbit, "eccentricity", o.eccentricity);
        put(&mut orbit, "obliquity", o.obliquity);
        put(&mut orbit, "perihelion_longitude", o.perihelion_longitude);
        put(&mut orbit, "start_solar_longitude", o.start_solar_longitude);
        put(&mut orbit, "instellation", instellation);
        put(&mut orbit, "tidally_locked", locked);
        if let Some(x) = o.substellar_lon {
            put(&mut orbit, "substellar_lon", x);
        }
        if let Some(d) = o.diurnal {
            put(&mut orbit, "diurnal", d);
        }
        if !o.months.is_empty() {
            let m: Vec<Value> = o.months.iter().map(|s| Value::from(s.as_str())).collect();
            put(&mut orbit, "months", Value::Array(m));
        }
        t.insert("orbit".into(), Value::Table(orbit));
        let resolved = super::PlanetConfig::deserialize(Value::Table(planet.clone()))
            .map_err(|e| invalid(format!("[planet]: {e}")))?
            .resolve()?;
        let info = SystemInfo {
            star,
            semi_major_axis: o.semi_major_axis,
            spin: o.spin.clone(),
            instellation,
            instellation_derived: s_derived,
            year,
            year_derived: p_derived,
            rotation_rate: resolved.rotation_rate,
            rotation_rate_derived: rate_derived,
            composition,
            gas_constant_derived: r_derived,
            cp_derived,
        };
        Ok((planet, Some(info)))
    }
}

mod physics;
#[cfg(test)]
mod tests;
