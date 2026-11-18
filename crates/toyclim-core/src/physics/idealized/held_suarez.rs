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

use crate::physics::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};

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
    /// (cos⁴φ for HS) towards `teq(p)`. Rates from the provisional state (forward Euler).
    fn apply(&self, ctx: &ColumnCtx, st: &mut ColumnState, w4: f64, teq: impl Fn(f64) -> f64) {
        for k in 0..st.nlev {
            let sigma = ctx.sigma[k];
            let bl = self.bl(sigma);
            let kv = self.kf * bl;
            let kt = self.ka + (self.ks - self.ka) * bl * w4;
            let p = sigma * ctx.ps;
            let (du, dv, dt) = (-(kv * st.u[k]), -(kv * st.v[k]), -(kt * (st.t[k] - teq(p))));
            st.add_u(k, du);
            st.add_v(k, dv);
            st.add_t(k, dt);
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
        self.t_eq_sc(kappa, lat.sin().powi(2), lat.cos().powi(2), p)
    }
    /// T_eq from sin²φ and cos²φ (computed once per column).
    fn t_eq_sc(&self, kappa: f64, s2: f64, c2: f64, p: f64) -> f64 {
        let pr = p / self.p0;
        ((self.t_surface - self.delta_ty * s2 - self.delta_theta_z * pr.ln() * c2) * pr.powf(kappa))
            .max(self.t_min)
    }
}

impl ColumnScheme for HeldSuarez {
    fn name(&self) -> &'static str {
        "held_suarez"
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        _diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let kappa = ctx.planet.kappa();
        let lat = ctx.lat;
        let w4 = lat.cos().powi(4);
        let (s2, c2) = (lat.sin().powi(2), lat.cos().powi(2));
        self.damping
            .apply(ctx, st, w4, |p| self.t_eq_sc(kappa, s2, c2, p));
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
        self.t_eq_mu(kappa, self.mu_star(lat, lon), p)
    }
    /// T_eq from μ* (computed once per column).
    fn t_eq_mu(&self, kappa: f64, mu: f64, p: f64) -> f64 {
        let pr = p / self.p0;
        ((self.t_night + self.delta_td * mu - self.delta_theta_z * pr.ln()) * pr.powf(kappa))
            .max(self.t_min)
    }
}

impl ColumnScheme for TidallyLocked {
    fn name(&self) -> &'static str {
        "tidally_locked"
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        _diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let kappa = ctx.planet.kappa();
        let mu = self.mu_star(ctx.lat, ctx.lon);
        let w4 = mu.powi(4);
        self.damping
            .apply(ctx, st, w4, |p| self.t_eq_mu(kappa, mu, p));
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
