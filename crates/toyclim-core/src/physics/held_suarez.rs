//! Held & Suarez (1994) forcing and a tidally locked day/night variant (SPEC.md §8, M5).
//!
//! Newtonian relaxation of temperature towards T_eq and Rayleigh friction in the boundary
//! layer:
//!   T_eq = max{T_min, [T_s − ΔT_y sin²φ − Δθ_z ln(p/p₀) cos²φ] (p/p₀)^κ}
//!   k_T  = k_a + (k_s − k_a) max(0, (σ−σ_b)/(1−σ_b)) cos⁴φ
//!   k_v  = k_f max(0, (σ−σ_b)/(1−σ_b))
//! Tidally locked variant (Merlis & Schneider 2010, Heng et al. 2011 in spirit):
//!   T_eq = max{T_min, [T_n + ΔT_d max(0, cos φ cos(λ−λ_ss)) − Δθ_z ln(p/p₀)] (p/p₀)^κ}
//!   with cos⁴φ in k_T replaced by max(0, cos φ cos(λ−λ_ss))⁴ (zero on the night side).

use super::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::Real;

const DAY: f64 = 86400.0;

/// Shared damping parameters.
#[derive(Debug, Clone)]
pub struct Damping {
    /// Rayleigh friction rate k_f at the surface \[s⁻¹\].
    pub kf: f64,
    /// Free-atmosphere radiative relaxation rate k_a \[s⁻¹\].
    pub ka: f64,
    /// Surface radiative relaxation rate k_s \[s⁻¹\].
    pub ks: f64,
    /// Boundary-layer top σ_b.
    pub sigma_b: f64,
}

impl Default for Damping {
    fn default() -> Self {
        Damping {
            kf: 1.0 / DAY,
            ka: 1.0 / (40.0 * DAY),
            ks: 1.0 / (4.0 * DAY),
            sigma_b: 0.7,
        }
    }
}

impl Damping {
    fn bl(&self, sigma: f64) -> f64 {
        ((sigma - self.sigma_b) / (1.0 - self.sigma_b)).max(0.0)
    }
    /// Rayleigh friction on u, v and Newtonian cooling of T with latitude weight `w4`
    /// (cos⁴φ for HS) towards `teq(p)`.
    fn apply(&self, col: &ColumnIn, out: &mut ColumnOut, w4: f64, teq: impl Fn(f64) -> f64) {
        for k in 0..col.t.len() {
            let sigma = col.sigma[k] as f64;
            let bl = self.bl(sigma);
            let kv = self.kf * bl;
            let kt = self.ka + (self.ks - self.ka) * bl * w4;
            let p = sigma * col.ps as f64;
            out.du[k] -= (kv * col.u[k] as f64) as Real;
            out.dv[k] -= (kv * col.v[k] as f64) as Real;
            out.dt[k] -= (kt * (col.t[k] as f64 - teq(p))) as Real;
        }
    }
}

/// Held–Suarez forcing.
#[derive(Debug, Clone)]
pub struct HeldSuarez {
    pub damping: Damping,
    /// Equator-to-pole temperature difference ΔT_y \[K\].
    pub delta_ty: f64,
    /// Static stability parameter Δθ_z \[K\].
    pub delta_theta_z: f64,
    /// Surface equatorial T_eq \[K\].
    pub t_surface: f64,
    /// Lower bound of T_eq \[K\].
    pub t_min: f64,
    /// Reference pressure p₀ \[Pa\].
    pub p0: f64,
}

impl Default for HeldSuarez {
    fn default() -> Self {
        HeldSuarez {
            damping: Damping::default(),
            delta_ty: 60.0,
            delta_theta_z: 10.0,
            t_surface: 315.0,
            t_min: 200.0,
            p0: 1.0e5,
        }
    }
}

impl HeldSuarez {
    /// T_eq at latitude `lat` and pressure `p`.
    pub fn t_eq(&self, kappa: f64, lat: f64, p: f64) -> f64 {
        let (s2, c2) = (lat.sin().powi(2), lat.cos().powi(2));
        let pr = p / self.p0;
        ((self.t_surface - self.delta_ty * s2 - self.delta_theta_z * pr.ln() * c2) * pr.powf(kappa))
            .max(self.t_min)
    }
}

impl ColumnPhysics for HeldSuarez {
    fn name(&self) -> &'static str {
        "held_suarez"
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let kappa = planet.kappa();
        let lat = col.lat as f64;
        let w4 = lat.cos().powi(4);
        self.damping
            .apply(col, out, w4, |p| self.t_eq(kappa, lat, p));
    }
}

/// Day/night Newtonian forcing for a tidally locked planet.
#[derive(Debug, Clone)]
pub struct TidallyLocked {
    pub damping: Damping,
    /// Night-side (and terminator) surface T_eq \[K\].
    pub t_night: f64,
    /// Substellar excess ΔT_d \[K\].
    pub delta_td: f64,
    pub delta_theta_z: f64,
    pub t_min: f64,
    pub p0: f64,
    /// Substellar longitude λ_ss \[rad\] (latitude 0).
    pub substellar_lon: f64,
}

impl Default for TidallyLocked {
    fn default() -> Self {
        TidallyLocked {
            damping: Damping::default(),
            t_night: 250.0,
            delta_td: 70.0,
            delta_theta_z: 10.0,
            t_min: 200.0,
            p0: 1.0e5,
            substellar_lon: std::f64::consts::PI,
        }
    }
}

impl TidallyLocked {
    /// cos of the stellar zenith angle, clipped at 0 (night).
    pub fn mu_star(&self, lat: f64, lon: f64) -> f64 {
        (lat.cos() * (lon - self.substellar_lon).cos()).max(0.0)
    }
    pub fn t_eq(&self, kappa: f64, lat: f64, lon: f64, p: f64) -> f64 {
        let pr = p / self.p0;
        ((self.t_night + self.delta_td * self.mu_star(lat, lon) - self.delta_theta_z * pr.ln())
            * pr.powf(kappa))
        .max(self.t_min)
    }
}

impl ColumnPhysics for TidallyLocked {
    fn name(&self) -> &'static str {
        "tidally_locked"
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let kappa = planet.kappa();
        let (lat, lon) = (col.lat as f64, col.lon as f64);
        let w4 = self.mu_star(lat, lon).powi(4);
        self.damping
            .apply(col, out, w4, |p| self.t_eq(kappa, lat, lon, p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hs_teq_values() {
        let hs = HeldSuarez::default();
        let kappa = 287.04 / 1004.64;
        // surface equator: 315 K; surface pole: 255 K; stratosphere floor 200 K
        assert!((hs.t_eq(kappa, 0.0, 1e5) - 315.0).abs() < 1e-12);
        assert!((hs.t_eq(kappa, std::f64::consts::FRAC_PI_2, 1e5) - 255.0).abs() < 1e-9);
        assert_eq!(hs.t_eq(kappa, 0.3, 5e3), 200.0);
    }

    #[test]
    fn tidally_locked_day_night() {
        let tl = TidallyLocked::default();
        let kappa = 2.0 / 7.0;
        assert!((tl.t_eq(kappa, 0.0, std::f64::consts::PI, 1e5) - 320.0).abs() < 1e-12);
        assert!((tl.t_eq(kappa, 0.0, 0.0, 1e5) - 250.0).abs() < 1e-12);
    }
}
