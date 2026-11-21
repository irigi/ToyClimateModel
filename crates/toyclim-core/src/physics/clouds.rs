//! Diagnostic clouds (SPEC.md §9.4.7, step 6 of §9.3.3): fraction and in-cloud water per
//! layer from the state after convection and condensation, stored in
//! [`SurfaceExchange::cloud`] for the next radiation call. No prognostic cloud water; the
//! scheme changes neither temperature nor humidity.
//!
//! - Stratiform fraction (Sundqvist type): c_s = 1 − √((1 − RH)/(1 − RH_c)) for RH > RH_c,
//!   with RH_c(σ) = RH_u + (RH_s − RH_u) exp(1 − σ^(−n)): RH_s near the surface, RH_u
//!   aloft (the shape of the ECHAM profile, Lohmann & Roeckner 1996; not checked against
//!   the paper, the values are knobs).
//! - Convective fraction from the convective precipitation P of this step, after Slingo
//!   (1987): c_c = min(c_max, b ln(1 + P / 1 mm day⁻¹)), between the top of the convecting
//!   layer and the second level from the bottom. (Slingo's form, a + b ln P, is singular
//!   at small P; the coefficients are knobs.)
//! - Optional "freeze-dry" reduction of the stratiform fraction in cold, dry low levels
//!   (Vavrus & Waliser 2008, J. Climate 21, 5673–5687; the paper could not be fetched, the
//!   form and constants are transcribed from the NCAR CAM source, `cloud_fraction.F90`,
//!   subroutine `cldfrc`, branch `cam_cesm2_2_rel`): for σ ≥ σ_fd,
//!   c_s ← c_s · max(f_min, min(1, q / q_fd)), with q_fd = 3e-3 kg kg⁻¹ and f_min = 0.15.
//!   CAM applies it to its low-cloud RH cloud below the pressure bound `premib`, whose value
//!   is not in that file; σ_fd = 0.75 (750 hPa) is unverified. On by default (P4 frozen knobs); `freeze_dry_q = 0` switches it off.
//! - Combined at each level as independent covers: c = 1 − (1 − c_s)(1 − c_c).
//! - In-cloud water mixing ratio r = min(β q_sat(T, p), r_max), split into ice and liquid
//!   with f_ice(T) of the condensable; path r Δp / g.
//! - No clouds above σ_min.
//!
//! Diagnostic: `clwvi`, the grid-mean condensed water path \[kg m⁻²\] (the total cover
//! `clt` comes from the radiation, which uses these clouds in the next step).

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::condensation::Condensable;

#[derive(Debug, Clone)]
pub struct DiagnosticClouds {
    pub cond: Condensable,
    /// Critical relative humidity near the surface and aloft, and the profile exponent.
    pub rh_crit_surface: f64,
    pub rh_crit_upper: f64,
    pub rh_crit_exponent: f64,
    /// Convective cover coefficient b and cap c_max.
    pub conv_coef: f64,
    pub conv_max: f64,
    /// In-cloud water as a fraction β of q_sat, capped at `water_max` \[kg kg⁻¹\].
    pub water_ratio: f64,
    pub water_max: f64,
    /// No clouds at σ below this.
    pub sigma_min: f64,
    /// Freeze-dry humidity scale q_fd \[kg kg⁻¹\] (0 = off), minimum factor and the σ at and
    /// below which it applies (low clouds).
    pub freeze_dry_q: f64,
    pub freeze_dry_min: f64,
    pub freeze_dry_sigma: f64,
}

impl DiagnosticClouds {
    pub fn new(cond: Condensable) -> Self {
        DiagnosticClouds {
            cond,
            rh_crit_surface: 0.95,
            rh_crit_upper: 0.9,
            rh_crit_exponent: 4.0,
            conv_coef: 0.125,
            conv_max: 0.8,
            water_ratio: 0.03,
            water_max: 5e-4,
            sigma_min: 0.1,
            freeze_dry_q: 3e-3,
            freeze_dry_min: 0.15,
            freeze_dry_sigma: 0.75,
        }
    }

    /// Critical relative humidity at σ.
    pub fn rh_crit(&self, sigma: f64) -> f64 {
        let x = (1.0 - sigma.max(1e-6).powf(-self.rh_crit_exponent)).exp();
        self.rh_crit_upper + (self.rh_crit_surface - self.rh_crit_upper) * x
    }

    /// Stratiform fraction for relative humidity `rh` with critical value `rc`.
    pub fn stratiform(rh: f64, rc: f64) -> f64 {
        if rh <= rc {
            0.0
        } else if rh >= 1.0 {
            1.0
        } else {
            1.0 - ((1.0 - rh) / (1.0 - rc)).sqrt()
        }
    }

    /// Freeze-dry factor for specific humidity `q` at σ (1 when off or above σ_fd).
    pub fn freeze_dry(&self, q: f64, sigma: f64) -> f64 {
        if self.freeze_dry_q <= 0.0 || sigma < self.freeze_dry_sigma {
            1.0
        } else {
            (q / self.freeze_dry_q).clamp(self.freeze_dry_min, 1.0)
        }
    }

    /// Convective fraction for precipitation `p` \[kg m⁻² s⁻¹\].
    pub fn convective(&self, p: f64) -> f64 {
        if p <= 0.0 {
            return 0.0;
        }
        (self.conv_coef * (1.0 + p * 86400.0).ln()).min(self.conv_max)
    }
}

impl ColumnScheme for DiagnosticClouds {
    fn name(&self) -> &'static str {
        "diagnostic_clouds"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["clwvi"]
    }
    fn memory(&self, nlev: usize) -> usize {
        3 * nlev
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let l = st.nlev;
        let m = &mut sfc.cloud;
        m.clear();
        m.resize(3 * l, 0.0);
        let Some(q) = st.humidity() else {
            return;
        };
        let (planet, g) = (ctx.planet, ctx.planet.gravity);
        let cc = self.convective(st.conv_precip);
        let mut path = 0.0;
        for k in 0..l {
            if ctx.sigma[k] < self.sigma_min {
                continue;
            }
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            let cs = Self::stratiform(q[k] / qs, self.rh_crit(ctx.sigma[k]))
                * self.freeze_dry(q[k], ctx.sigma[k]);
            let cv = if k >= st.conv_top && k + 1 < l {
                cc
            } else {
                0.0
            };
            let c = 1.0 - (1.0 - cs) * (1.0 - cv);
            if c <= 0.0 {
                continue;
            }
            let w = (self.water_ratio * qs).min(self.water_max) * ctx.dp[k] / g;
            let fi = self.cond.ice_fraction(st.t[k]);
            m[k] = c;
            m[l + k] = (1.0 - fi) * w;
            m[2 * l + k] = fi * w;
            path += c * w;
        }
        diag[0] += path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;
    use crate::physics::radiation::solver::cloud_cover;

    fn scheme() -> DiagnosticClouds {
        DiagnosticClouds::new(Condensable::water())
    }

    #[test]
    fn fraction_limits_and_profile() {
        assert_eq!(DiagnosticClouds::stratiform(0.5, 0.8), 0.0);
        assert_eq!(DiagnosticClouds::stratiform(1.01, 0.8), 1.0);
        let c = DiagnosticClouds::stratiform(0.95, 0.8);
        assert!((c - (1.0 - 0.25f64.sqrt())).abs() < 1e-12);
        let s = scheme();
        assert!((s.rh_crit(1.0) - 0.95).abs() < 1e-12);
        assert!((s.rh_crit(0.4) - 0.9).abs() < 1e-6);
        assert_eq!(s.convective(0.0), 0.0);
        let ten = 10.0 / 86400.0;
        assert!((s.convective(ten) - 0.125 * 11f64.ln()).abs() < 1e-12);
        assert_eq!(s.convective(1.0), 0.8);
        // freeze-dry: factor q / 3 g/kg between 0.15 and 1, low levels only; off with q_fd = 0
        let off = DiagnosticClouds {
            freeze_dry_q: 0.0,
            ..scheme()
        };
        assert_eq!(off.freeze_dry(1e-4, 0.95), 1.0);
        let f = scheme();
        assert!((f.freeze_dry(1.5e-3, 0.9) - 0.5).abs() < 1e-12);
        assert_eq!(f.freeze_dry(1e-4, 0.9), 0.15);
        assert_eq!(f.freeze_dry(5e-3, 0.9), 1.0);
        assert_eq!(f.freeze_dry(1e-4, 0.5), 1.0);
    }

    /// Freeze-dry in a cold, dry saturated column: low cloud reduced, still no tendencies,
    /// fractions in [0, 1]; a warm moist column is unchanged.
    #[test]
    fn freeze_dry_column() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let run = |s: &DiagnosticClouds, t_sfc: f64| {
            let mut st = ColumnState::new(26, 1);
            st.begin(1200.0);
            for k in 0..26 {
                st.t[k] = t_sfc - 50.0 * (1.0 - ctx.sigma[k]);
                st.q[k] = 0.97 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
            }
            let q0 = st.q.clone();
            let mut sfc = SurfaceExchange::default();
            s.apply(&ctx, &mut st, &mut sfc, &mut [0.0], &mut Workspace::new(26));
            assert_eq!(st.q, q0);
            assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
            assert!(sfc.cloud[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
            sfc.cloud
        };
        let off = DiagnosticClouds {
            freeze_dry_q: 0.0,
            ..scheme()
        };
        let on = scheme();
        let (a, b) = (run(&off, 245.0), run(&on, 245.0));
        assert!(b[25] < 0.5 * a[25] && b[25] > 0.0, "{} {}", a[25], b[25]);
        assert_eq!(a[5], b[5]); // aloft unchanged
        assert_eq!(run(&off, 300.0), run(&on, 300.0));
    }

    /// The scheme leaves T and q alone, stores fractions in [0, 1] and positive paths, and
    /// a saturated layer is overcast.
    #[test]
    fn column_memory_and_no_tendencies() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let s = scheme();
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = 290.0 - 60.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.5 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        st.q[20] = 1.05 * s.cond.qsat(ctx.planet, st.t[20], ctx.p[20]);
        st.conv_precip = 20.0 / 86400.0;
        st.conv_top = 10;
        let (t0, q0) = (st.t.clone(), st.q.clone());
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0; 1];
        s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
        assert_eq!(st.t, t0);
        assert_eq!(st.q, q0);
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        let m = &sfc.cloud;
        assert_eq!(m.len(), 78);
        assert_eq!(m[20], 1.0);
        let cv = s.convective(st.conv_precip);
        assert!((m[12] - cv).abs() < 1e-12, "{}", m[12]);
        assert_eq!(m[5], 0.0); // above the convection, dry
        assert_eq!(m[25], 0.0); // lowest level: no convective cloud
        assert!(m.iter().all(|&x| x >= 0.0));
        assert!(m[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
        assert!(cloud_cover(&m[..26]) >= 1.0 - 1e-12 && diag[0] > 0.0);
        // warm layer liquid, cold layer ice
        assert!(m[26 + 20] > 0.0);
    }
}
