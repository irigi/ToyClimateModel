//! The host star (SPEC.md §9.3.2). A blackbody spectrum; band fractions arrive with the
//! radiation scheme in P3.

use serde::{Deserialize, Serialize};

/// Stellar parameters, in solar units where noted.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Star {
    /// Effective temperature \[K\].
    pub t_eff: f64,
    /// Bolometric luminosity \[L☉\].
    pub luminosity: f64,
    /// Mass \[M☉\].
    pub mass: f64,
}

impl Star {
    /// The Sun: T_eff = 5772 K (IAU 2015 Resolution B3 nominal value), L = M = 1.
    pub fn sun() -> Self {
        Star {
            t_eff: 5772.0,
            luminosity: 1.0,
            mass: 1.0,
        }
    }

    /// TRAPPIST-1 after Agol et al. (2021), Planet. Sci. J. 2, 1, Table 7:
    /// M = 0.0898 M☉ (Mann et al. 2019), L = 5.53e-4 L☉ (Ducrot et al. 2020),
    /// T_eff = 2566 K.
    pub fn trappist1() -> Self {
        Star {
            t_eff: 2566.0,
            luminosity: 5.53e-4,
            mass: 0.0898,
        }
    }

    /// Check that all values are positive and finite.
    pub fn validate(&self) -> Result<(), String> {
        for (what, v) in [
            ("t_eff", self.t_eff),
            ("luminosity", self.luminosity),
            ("mass", self.mass),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("star {what} must be > 0"));
            }
        }
        Ok(())
    }
}
