//! Planet constants and derived quantities.

use serde::{Deserialize, Serialize};

/// Physical constants of a planet and its atmosphere. All SI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Planet {
    /// Human-readable name.
    pub name: String,
    /// Mean radius a \[m\].
    pub radius: f64,
    /// Rotation rate Ω \[s⁻¹\]. May be zero or negative (retrograde).
    pub rotation_rate: f64,
    /// Surface gravity g \[m s⁻²\].
    pub gravity: f64,
    /// Specific gas constant of dry air R_d \[J kg⁻¹ K⁻¹\].
    pub gas_constant: f64,
    /// Specific heat at constant pressure c_p \[J kg⁻¹ K⁻¹\].
    pub cp: f64,
    /// Reference surface pressure p₀ \[Pa\].
    pub p0: f64,
    /// Isothermal reference temperature T̄ of the semi-implicit scheme \[K\]. Must be at
    /// least as warm as typical temperatures for stability (SPEC.md §5.2).
    #[serde(default = "default_tref")]
    pub t_ref: f64,
}

fn default_tref() -> f64 {
    300.0
}

impl Planet {
    /// Earth with the Williamson et al. (1992) values.
    pub fn earth() -> Self {
        Planet {
            name: "earth".into(),
            radius: 6.37122e6,
            rotation_rate: 7.292e-5,
            gravity: 9.80616,
            gas_constant: 287.04,
            cp: 1004.64,
            p0: 1.0e5,
            t_ref: 300.0,
        }
    }

    /// Look up a named preset.
    pub fn preset(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "earth" => Some(Self::earth()),
            _ => None,
        }
    }

    /// Names of all presets.
    pub fn preset_names() -> &'static [&'static str] {
        &["earth"]
    }

    /// κ = R_d / c_p (dimensionless).
    pub fn kappa(&self) -> f64 {
        self.gas_constant / self.cp
    }

    /// Coriolis parameter f = 2Ω sin φ \[s⁻¹\] at latitude `lat` \[rad\].
    pub fn coriolis(&self, lat: f64) -> f64 {
        2.0 * self.rotation_rate * lat.sin()
    }

    /// Length of the sidereal day 2π/|Ω| \[s\]; infinite for Ω = 0.
    pub fn sidereal_day(&self) -> f64 {
        2.0 * std::f64::consts::PI / self.rotation_rate.abs()
    }
}

impl Default for Planet {
    fn default() -> Self {
        Self::earth()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_kappa() {
        let e = Planet::earth();
        assert!((e.kappa() - 287.04 / 1004.64).abs() < 1e-15);
        assert!(Planet::preset("Earth").is_some());
        assert!(Planet::preset("vulcan").is_none());
    }
}
