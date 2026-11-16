//! Configuration blocks shared by both schemas (and the internal representation).

use super::{ConfigError, Duration};
use crate::planet::Planet;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which model to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    /// Writes analytic fields only; exercises the I/O pipeline (M0).
    Dummy,
    ShallowWater,
    Primitive,
    /// Single-column model (`toyclim scm`, SPEC.md §9.5.1).
    Scm,
}

impl ModelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelKind::Dummy => "dummy",
            ModelKind::ShallowWater => "shallow_water",
            ModelKind::Scm => "scm",
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
    pub fn resolve(&self) -> Result<crate::physics::Condensable, ConfigError> {
        let mut c = crate::physics::Condensable::preset(&self.condensable).ok_or_else(|| {
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

/// `[orbit]` block. Angles in degrees.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbitConfig {
    /// Orbital period (default 365.25 days).
    #[serde(default = "default_year")]
    pub year: Duration,
    #[serde(default)]
    pub eccentricity: f64,
    #[serde(default = "default_obliquity")]
    pub obliquity: f64,
    #[serde(default = "default_perihelion")]
    pub perihelion_longitude: f64,
    #[serde(default)]
    pub start_solar_longitude: f64,
    /// Stellar flux at the semi-major axis \[W m⁻²\].
    #[serde(default = "default_instellation")]
    pub instellation: f64,
    #[serde(default)]
    pub tidally_locked: bool,
    #[serde(default = "default_substellar")]
    pub substellar_lon: f64,
    /// Resolve the diurnal cycle (default true).
    #[serde(default = "default_true")]
    pub diurnal: bool,
    /// Calendar month names for statistics (default: 12 Earth months, equal length).
    /// Months start at t = 0, which is the vernal equinox unless `start_solar_longitude`
    /// is set (Earth's 1 January is ≈ 280°).
    #[serde(default)]
    pub months: Vec<String>,
}

fn default_year() -> Duration {
    Duration(365.25 * 86400.0)
}
fn default_obliquity() -> f64 {
    23.44
}
fn default_perihelion() -> f64 {
    282.9
}
fn default_instellation() -> f64 {
    1361.0
}
fn default_substellar() -> f64 {
    180.0
}
fn default_true() -> bool {
    true
}

impl OrbitConfig {
    pub fn resolve(&self, rotation_rate: f64) -> crate::system::orbit::Orbit {
        crate::system::orbit::Orbit {
            year: self.year.0,
            eccentricity: self.eccentricity,
            obliquity: self.obliquity.to_radians(),
            perihelion_longitude: self.perihelion_longitude.to_radians(),
            start_solar_longitude: self.start_solar_longitude.to_radians(),
            instellation: self.instellation,
            tidally_locked: self.tidally_locked,
            substellar_lon: self.substellar_lon.to_radians(),
            diurnal: self.diurnal,
            rotation_rate,
        }
    }
}

/// `[surface]` block: slab ocean (mixed layer).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceConfig {
    /// Mixed-layer depth \[m\] (default 20).
    #[serde(default = "default_mld")]
    pub mixed_layer_depth: f64,
    /// Initial surface temperature: "tj16" (default) or a number in K.
    pub initial: Option<toml::Value>,
    /// Keep T_s fixed (prescribed SST).
    #[serde(default)]
    pub fixed: bool,
    /// Equirectangular heightmap (.npy, float32/64, `[lat][lon]`, row 0 north, metres; ≤ 0 is
    /// ocean). Path relative to the working directory. Absent → aquaplanet.
    pub heightmap: Option<String>,
    /// Land heat capacity \[J m⁻² K⁻¹\] (default 2e5 ≈ 5 cm of water).
    #[serde(default = "default_land_c")]
    pub land_heat_capacity: f64,
    /// Land albedo (default 0.3).
    #[serde(default = "default_land_albedo")]
    pub land_albedo: f64,
    /// Ocean albedo (default: the radiation scheme's own value).
    pub ocean_albedo: Option<f64>,
    /// Bucket capacity \[kg m⁻²\] (default 150).
    #[serde(default = "default_bucket")]
    pub bucket_capacity: f64,
    /// Initial bucket fill fraction (default 0.75 = fully wet).
    #[serde(default = "default_bucket_init")]
    pub initial_bucket: f64,
    /// Spectral orography filter: damping factor at ℓ = M (default 0.1).
    #[serde(default = "default_oro_filter")]
    pub orography_filter: f64,
    /// Albedo of sea ice / snow (default 0.6; negative disables the ice–albedo switch).
    #[serde(default = "default_ice_albedo")]
    pub ice_albedo: f64,
    /// Ocean freezing temperature \[K\] (default 271.15).
    #[serde(default = "default_ocean_freeze")]
    pub ocean_freeze: f64,
    /// Land snow threshold \[K\] (default 273.15).
    #[serde(default = "default_land_freeze")]
    pub land_freeze: f64,
    /// Amplitude Q0 \[W m⁻²\] of the prescribed ocean heat transport (q-flux)
    /// Q0 (1 − 2φ²/φ0²) exp(−φ²/φ0²)/cos φ with φ0 = 16° (Merlis et al. 2013). Typical
    /// Earth-like value 50; default 0 (off).
    #[serde(default)]
    pub ocean_heat_transport: f64,
}

fn default_ice_albedo() -> f64 {
    0.6
}
fn default_ocean_freeze() -> f64 {
    271.15
}
fn default_land_freeze() -> f64 {
    273.15
}

fn default_land_c() -> f64 {
    2.0e5
}
fn default_land_albedo() -> f64 {
    0.3
}
fn default_bucket() -> f64 {
    150.0
}
fn default_bucket_init() -> f64 {
    0.75
}
fn default_oro_filter() -> f64 {
    0.1
}

fn default_mld() -> f64 {
    20.0
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
    /// Also keep per-calendar-month means of the maps (needs an `[orbit]`).
    #[serde(default)]
    pub monthly: bool,
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

/// A typed physics block: `scheme` plus that scheme's parameters (checked when the
/// physics is built).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeConfig {
    pub scheme: String,
    #[serde(flatten)]
    pub params: toml::Table,
}

impl SchemeConfig {
    pub fn new(scheme: &str, params: toml::Table) -> Self {
        SchemeConfig {
            scheme: scheme.into(),
            params,
        }
    }
}

/// World physics from the typed blocks of config schema 2 (SPEC.md §9.3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsConfig {
    pub radiation: SchemeConfig,
    pub boundary_layer: SchemeConfig,
    pub convection: Option<SchemeConfig>,
    pub condensation: Option<toml::Table>,
    pub clouds: Option<SchemeConfig>,
}

/// Dynamical-core options of Phase 2 (`[dynamics]`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicsConfig {
    /// Return the kinetic energy removed by hyperdiffusion as a uniform heating
    /// (SPEC.md §9.4.3; off for Part I cases).
    #[serde(default)]
    pub energy_fixer: bool,
    /// `[dynamics.sponge]`: `enabled`, `sigma_top` (0.02), `tau` ("1day").
    pub sponge: Option<toml::Table>,
    /// Tracer (humidity) transport: `"spectral"` (default; Part I and the idealized
    /// comparators) or `"semi_lagrangian"` (grid-point, SPEC.md §9.8).
    pub tracer_transport: Option<String>,
}
