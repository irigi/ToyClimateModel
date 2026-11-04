//! TOML run configuration (SPEC.md §6).
//!
//! Durations may be written as strings with a unit (`"30min"`, `"1day"`, `"2.4h"`,
//! `"1200s"`) or as bare numbers in seconds.

use crate::planet::Planet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
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

/// A span of model time, stored in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Duration(pub f64);

impl Duration {
    /// Length in seconds.
    pub fn seconds(self) -> f64 {
        self.0
    }
}

impl FromStr for Duration {
    type Err = ConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        // The unit is the trailing alphabetic suffix, so exponents like "1e3s" work.
        let num = t.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let unit = &t[num.len()..];
        let value: f64 = num
            .trim()
            .parse()
            .map_err(|_| ConfigError::Duration(s.to_string()))?;
        let scale = match unit.trim().to_ascii_lowercase().as_str() {
            "" | "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
            "min" | "mins" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
            "d" | "day" | "days" => 86400.0,
            _ => return Err(ConfigError::Duration(s.to_string())),
        };
        if !value.is_finite() || value < 0.0 {
            return Err(ConfigError::Duration(s.to_string()));
        }
        Ok(Duration(value * scale))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f64),
            Str(String),
        }
        match Raw::deserialize(d)? {
            Raw::Num(x) if x.is_finite() && x >= 0.0 => Ok(Duration(x)),
            Raw::Num(x) => Err(serde::de::Error::custom(format!("invalid duration {x}"))),
            Raw::Str(s) => s.parse().map_err(serde::de::Error::custom),
        }
    }
}

impl Serialize for Duration {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.0)
    }
}

/// Which model to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    /// Writes analytic fields only; exercises the I/O pipeline (M0).
    Dummy,
    ShallowWater,
    Primitive,
}

impl ModelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelKind::Dummy => "dummy",
            ModelKind::ShallowWater => "shallow_water",
            ModelKind::Primitive => "primitive",
        }
    }
}

/// Initial condition: a named case plus free-form parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InitialCondition {
    pub case: String,
    #[serde(flatten)]
    pub params: BTreeMap<String, toml::Value>,
}

/// Planet block: a preset with optional explicit overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanetConfig {
    pub preset: Option<String>,
    pub name: Option<String>,
    /// \[m\]
    pub radius: Option<f64>,
    /// \[s⁻¹\]
    pub rotation_rate: Option<f64>,
    /// Alternative to `rotation_rate`: Ω as a multiple of Earth's.
    pub rotation_factor: Option<f64>,
    /// \[m s⁻²\]
    pub gravity: Option<f64>,
    /// \[J kg⁻¹ K⁻¹\]
    pub gas_constant: Option<f64>,
    /// \[J kg⁻¹ K⁻¹\]
    pub cp: Option<f64>,
    /// \[Pa\]
    pub p0: Option<f64>,
    /// Semi-implicit reference temperature T̄ \[K\].
    pub t_ref: Option<f64>,
}

impl PlanetConfig {
    /// Resolve to concrete constants: preset (default Earth), then overrides.
    pub fn resolve(&self) -> Result<Planet, ConfigError> {
        let mut p = match &self.preset {
            Some(name) => Planet::preset(name).ok_or_else(|| {
                ConfigError::Invalid(format!(
                    "unknown planet preset {name:?} (known: {:?})",
                    Planet::preset_names()
                ))
            })?,
            None => Planet::earth(),
        };
        if self.rotation_rate.is_some() && self.rotation_factor.is_some() {
            return Err(ConfigError::Invalid(
                "give either rotation_rate or rotation_factor, not both".into(),
            ));
        }
        if let Some(v) = &self.name {
            p.name = v.clone();
        }
        if let Some(v) = self.radius {
            p.radius = v;
        }
        if let Some(v) = self.rotation_rate {
            p.rotation_rate = v;
        }
        if let Some(v) = self.rotation_factor {
            p.rotation_rate = v * Planet::earth().rotation_rate;
        }
        if let Some(v) = self.gravity {
            p.gravity = v;
        }
        if let Some(v) = self.gas_constant {
            p.gas_constant = v;
        }
        if let Some(v) = self.cp {
            p.cp = v;
        }
        if let Some(v) = self.p0 {
            p.p0 = v;
        }
        if let Some(v) = self.t_ref {
            p.t_ref = v;
        }
        for (what, v) in [
            ("radius", p.radius),
            ("gravity", p.gravity),
            ("gas_constant", p.gas_constant),
            ("cp", p.cp),
            ("p0", p.p0),
            ("t_ref", p.t_ref),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(ConfigError::Invalid(format!("planet {what} must be > 0")));
            }
        }
        if !p.rotation_rate.is_finite() {
            return Err(ConfigError::Invalid(
                "planet rotation_rate must be finite".into(),
            ));
        }
        Ok(p)
    }
}

/// Implicit hyperdiffusion (SPEC.md §4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffusionConfig {
    /// Power p of the Laplacian (∇^{2p}); default 4.
    #[serde(default = "default_diff_order")]
    pub order: u32,
    /// e-folding time τ_h at the truncation wavenumber; default 2.4 h.
    #[serde(default = "default_diff_tau")]
    pub tau: Duration,
}

fn default_diff_order() -> u32 {
    4
}
fn default_diff_tau() -> Duration {
    Duration(2.4 * 3600.0)
}

impl Default for DiffusionConfig {
    fn default() -> Self {
        DiffusionConfig {
            order: default_diff_order(),
            tau: default_diff_tau(),
        }
    }
}

/// Robert–Asselin–Williams filter parameters (SPEC.md §4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeFilterConfig {
    #[serde(default = "default_nu")]
    pub nu: f64,
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_nu() -> f64 {
    0.1
}
fn default_alpha() -> f64 {
    0.53
}

impl Default for TimeFilterConfig {
    fn default() -> Self {
        TimeFilterConfig {
            nu: default_nu(),
            alpha: default_alpha(),
        }
    }
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
    /// Forcing block (free-form until M5 defines it).
    pub forcing: Option<toml::Table>,
    #[serde(default)]
    pub diffusion: DiffusionConfig,
    #[serde(default)]
    pub time_filter: TimeFilterConfig,
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
}

/// `[moisture]` block: the condensing species (preset plus optional overrides) and the
/// initial humidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoistureConfig {
    /// Preset name ("water").
    #[serde(default = "default_condensable")]
    pub condensable: String,
    pub latent_heat: Option<f64>,
    pub r_vapor: Option<f64>,
    pub e0: Option<f64>,
    pub t0: Option<f64>,
    /// Initial humidity: "tj16" (Thatcher & Jablonowski 2016 eq. A1, default), "dry".
    #[serde(default = "default_initial_q")]
    pub initial: String,
}

fn default_condensable() -> String {
    "water".into()
}
fn default_initial_q() -> String {
    "tj16".into()
}

impl MoistureConfig {
    pub fn resolve(&self) -> Result<crate::physics::moist::Condensable, ConfigError> {
        let mut c =
            crate::physics::moist::Condensable::preset(&self.condensable).ok_or_else(|| {
                ConfigError::Invalid(format!("unknown condensable {:?}", self.condensable))
            })?;
        if let Some(v) = self.latent_heat {
            c.latent_heat = v;
        }
        if let Some(v) = self.r_vapor {
            c.r_vapor = v;
        }
        if let Some(v) = self.e0 {
            c.e0 = v;
        }
        if let Some(v) = self.t0 {
            c.t0 = v;
        }
        Ok(c)
    }
}

/// `[climatology]` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClimatologyConfig {
    /// Start accumulating at this model time (spin-up discarded).
    pub start: Duration,
    /// Sampling interval (multiple of dt).
    pub every: Duration,
    /// Fields whose full time-mean maps are kept.
    #[serde(default)]
    pub maps: Vec<String>,
}

/// A point at which time series are recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationConfig {
    pub name: String,
    /// Latitude \[degrees north\].
    pub lat: f64,
    /// Longitude \[degrees east\].
    pub lon: f64,
}

impl Config {
    /// Parse and validate a TOML string.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let c: Config = toml::from_str(text)?;
        c.validate()?;
        Ok(c)
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
        if self.model == ModelKind::Primitive && self.levels.is_none() && self.sigma_half.is_none()
        {
            return bad("primitive model needs `levels` or `sigma_half`");
        }
        self.planet.resolve()?;
        Ok(())
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
