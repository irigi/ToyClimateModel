//! TOML run configuration (SPEC.md §6).
//!
//! Durations may be written as strings with a unit (`"30min"`, `"1day"`, `"2.4h"`,
//! `"1200s"`) or as bare numbers in seconds.

mod blocks;
mod duration;
mod envelope;
pub mod v2;

pub use blocks::*;
pub use duration::Duration;

#[cfg(test)]
use crate::planet::Planet;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid duration {0:?}: expected a number with unit s, min, h or day")]
    Duration(String),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// A full run configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Free-text description, shown by `toyclim list-cases`.
    #[serde(default)]
    pub description: String,
    pub model: ModelKind,
    /// Triangular truncation M.
    pub truncation: usize,
    /// Number of σ levels (primitive equations; equally spaced unless `sigma_half` given).
    pub levels: Option<usize>,
    /// Explicit half-level σ values from 0 (top) to 1 (surface), length levels+1.
    pub sigma_half: Option<Vec<f64>>,
    pub dt: Duration,
    pub length: Duration,
    pub output_every: Duration,
    #[serde(default)]
    pub output_fields: Vec<String>,
    #[serde(default)]
    pub initial_condition: InitialCondition,
    #[serde(default)]
    pub planet: PlanetConfig,
    /// Idealized physics bundle (`type = "held_suarez" | "tidally_locked" |
    /// "moist_held_suarez"`, or the schema-1 alias `"gray_aquaplanet"`).
    pub forcing: Option<toml::Table>,
    /// World physics from the typed blocks of schema 2.
    pub physics: Option<PhysicsConfig>,
    /// Dynamical-core options (energy fixer, sponge).
    pub dynamics: Option<DynamicsConfig>,
    /// Single-column settings (`model = "scm"`), see `crate::scm`.
    pub scm: Option<toml::Table>,
    #[serde(default)]
    pub diffusion: DiffusionConfig,
    #[serde(default)]
    pub time_filter: TimeFilterConfig,
    /// Orbit and star (M8). Absent → no insolation (forcings that need it fail).
    pub orbit: Option<OrbitConfig>,
    /// Surface model (M8/M9): slab ocean. Absent → schemes use prescribed values.
    pub surface: Option<SurfaceConfig>,
    /// Moisture (M7): a condensable tracer q. Absent → dry model.
    pub moisture: Option<MoistureConfig>,
    /// Online climatology (time means after a spin-up); see `climatology.rs`.
    pub climatology: Option<ClimatologyConfig>,
    /// Global dry-mass fixer for the primitive-equation model (default true).
    pub mass_fixer: Option<bool>,
    /// Point time series ("weather stations"), see docs/design-decisions.md.
    #[serde(default)]
    pub stations: Vec<StationConfig>,
    /// Sampling interval of the station series (default: every step).
    pub station_every: Option<Duration>,
    /// Fields sampled at stations (default: all 2D fields of the model / lowest level).
    #[serde(default)]
    pub station_fields: Vec<String>,
    /// Star, orbit and atmosphere inputs with their derived values (schema 2 only;
    /// written to `meta.json` with the config echo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<v2::SystemInfo>,
}

impl Config {
    /// Parse and validate a TOML string.
    ///
    /// `schema = 2` files (SPEC.md §9.3.2) are lowered to this representation; files
    /// without `schema` (or with `schema = 1`) are read as they are.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut raw: toml::Table = toml::from_str(text)?;
        let schema = match raw.get("schema") {
            None => 1,
            Some(v) => v
                .as_integer()
                .ok_or_else(|| ConfigError::Invalid("schema must be an integer".into()))?,
        };
        let c = match schema {
            1 => {
                raw.remove("schema");
                Config::deserialize(toml::Value::Table(raw))?
            }
            2 => {
                let v = v2::ConfigV2::deserialize(toml::Value::Table(raw.clone()))?;
                let (lowered, system) = v.lower(&raw)?;
                let mut c = Config::deserialize(toml::Value::Table(lowered))?;
                c.system = system;
                c
            }
            n => return Err(ConfigError::Invalid(format!("unknown schema {n} (1, 2)"))),
        };
        c.validate()?;
        Ok(c)
    }

    /// Build the physics once to check scheme names and parameters.
    fn validate_physics(&self) -> Result<(), ConfigError> {
        let inv = |e: String| ConfigError::Invalid(e);
        let cond = self.moisture.as_ref().map(|m| m.resolve()).transpose()?;
        let sponge = self.dynamics.as_ref().and_then(|d| d.sponge.as_ref());
        if let Some(t) = self
            .dynamics
            .as_ref()
            .and_then(|d| d.tracer_transport.as_deref())
        {
            if !matches!(t, "spectral" | "semi_lagrangian") {
                return Err(inv(format!(
                    "[dynamics] tracer_transport = \"{t}\": expected \"spectral\" or \"semi_lagrangian\""
                )));
            }
        }
        match &self.physics {
            Some(pc) => {
                crate::physics::build_world(pc, sponge, cond.as_ref()).map_err(inv)?;
            }
            None => {
                if sponge.is_some() {
                    return Err(inv("[dynamics.sponge] needs the typed world physics".into()));
                }
                let planet = self.planet.resolve()?;
                crate::physics::build(self.forcing.as_ref(), &planet, cond.as_ref())
                    .map_err(inv)?;
            }
        }
        Ok(())
    }

    /// Read, parse and validate a TOML file.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Check internal consistency.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let bad = |m: &str| Err(ConfigError::Invalid(m.to_string()));
        if self.truncation < 1 {
            return bad("truncation must be >= 1");
        }
        if self.dt.0 <= 0.0 {
            return bad("dt must be > 0");
        }
        if self.output_every.0 <= 0.0 {
            return bad("output_every must be > 0");
        }
        let ratio = self.output_every.0 / self.dt.0;
        if (ratio - ratio.round()).abs() > 1e-9 * ratio.max(1.0) {
            return bad("output_every must be a whole multiple of dt");
        }
        if let Some(s) = &self.sigma_half {
            if s.len() < 2 || s[0] != 0.0 || *s.last().unwrap() != 1.0 {
                return bad("sigma_half must start at 0 and end at 1");
            }
            if !s.windows(2).all(|w| w[1] > w[0]) {
                return bad("sigma_half must be strictly increasing");
            }
            if let Some(l) = self.levels {
                if l + 1 != s.len() {
                    return bad("sigma_half must have levels+1 entries");
                }
            }
        }
        if self.forcing.is_some() && self.physics.is_some() {
            return bad("use either [forcing] (an idealized bundle) or the typed physics blocks");
        }
        self.validate_physics()?;
        if let Some(se) = self.station_every {
            let r = se.0 / self.dt.0;
            if se.0 <= 0.0 || (r - r.round()).abs() > 1e-9 * r.max(1.0) {
                return bad("station_every must be a positive whole multiple of dt");
            }
        }
        for st in &self.stations {
            if !(-90.0..=90.0).contains(&st.lat) || !st.lon.is_finite() {
                return bad("station latitude must be within ±90°");
            }
            if st.name.is_empty() || st.name.contains(['/', '\\']) {
                return bad("station names must be non-empty and contain no slashes");
            }
        }
        if matches!(self.model, ModelKind::Primitive | ModelKind::Scm)
            && self.levels.is_none()
            && self.sigma_half.is_none()
        {
            return bad("primitive model needs `levels` or `sigma_half`");
        }
        if (self.model == ModelKind::Scm) != self.scm.is_some() {
            return bad("an [scm] block goes with model = \"scm\" (and only there)");
        }
        if let Some(s) = &self.scm {
            crate::scm::ScmSettings::parse(s).map_err(ConfigError::Invalid)?;
        }
        self.planet.resolve()?;
        Ok(())
    }

    /// Warnings for settings outside the supported envelope (SPEC.md §9.2 P-5).
    pub fn envelope_warnings(&self) -> Vec<String> {
        if self.model == ModelKind::Primitive && (self.orbit.is_some() || self.system.is_some()) {
            envelope::envelope_warnings(self)
        } else {
            Vec::new()
        }
    }

    /// Number of time steps between outputs.
    pub fn steps_per_output(&self) -> u64 {
        (self.output_every.0 / self.dt.0).round() as u64
    }

    /// Steps between station samples (1 if not set).
    pub fn steps_per_station_sample(&self) -> u64 {
        self.station_every
            .map(|d| (d.0 / self.dt.0).round() as u64)
            .unwrap_or(1)
            .max(1)
    }

    /// Total number of time steps (rounded up).
    pub fn total_steps(&self) -> u64 {
        (self.length.0 / self.dt.0 - 1e-9).ceil().max(0.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        let d = |s: &str| s.parse::<Duration>().unwrap().0;
        assert_eq!(d("30min"), 1800.0);
        assert_eq!(d("1day"), 86400.0);
        assert_eq!(d("5 days"), 432000.0);
        assert_eq!(d("6h"), 21600.0);
        assert!((d("2.4h") - 8640.0).abs() < 1e-9);
        assert_eq!(d("1200s"), 1200.0);
        assert_eq!(d("1200"), 1200.0);
        assert_eq!(d("1e3s"), 1000.0);
        assert_eq!(d("1d"), 86400.0);
        assert!("3 fortnights".parse::<Duration>().is_err());
        assert!("-1h".parse::<Duration>().is_err());
        assert!("h".parse::<Duration>().is_err());
    }

    const EXAMPLE: &str = r#"
model = "shallow_water"
truncation = 42
dt = "30min"
length = "5day"
output_every = "6h"
output_fields = ["h", "vor", "u", "v"]
initial_condition = { case = "williamson2", alpha = 0.0 }
[planet]
preset = "earth"
[diffusion]
order = 4
tau = "2.4h"
"#;

    #[test]
    fn spec_example_parses() {
        let c = Config::parse(EXAMPLE).unwrap();
        assert_eq!(c.model, ModelKind::ShallowWater);
        assert_eq!(c.truncation, 42);
        assert_eq!(c.dt.0, 1800.0);
        assert_eq!(c.steps_per_output(), 12);
        assert_eq!(c.total_steps(), 240);
        assert_eq!(c.initial_condition.case, "williamson2");
        assert_eq!(c.initial_condition.params["alpha"].as_float(), Some(0.0));
        assert_eq!(c.diffusion.tau.0, 8640.0);
        assert_eq!(c.planet.resolve().unwrap(), Planet::earth());
        assert_eq!(c.time_filter.alpha, 0.53);
    }

    #[test]
    fn planet_overrides_and_errors() {
        let c = Config::parse(&EXAMPLE.replace(
            "preset = \"earth\"",
            "preset = \"earth\"\nrotation_factor = -0.5\nradius = 3.4e6",
        ))
        .unwrap();
        let p = c.planet.resolve().unwrap();
        assert_eq!(p.radius, 3.4e6);
        assert!((p.rotation_rate + 0.5 * 7.292e-5).abs() < 1e-20);

        assert!(Config::parse(&EXAMPLE.replace("\"earth\"", "\"krypton\"")).is_err());
        assert!(Config::parse(&EXAMPLE.replace("\"6h\"", "\"45min\"")).is_err());
        assert!(Config::parse(&format!("{EXAMPLE}\n[bogus]\nx=1")).is_err());
    }
}
