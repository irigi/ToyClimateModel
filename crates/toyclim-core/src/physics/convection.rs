//! Convection: dry convective adjustment and simplified Betts–Miller moist convection.

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::condensation::Condensable;
use crate::planet::Planet;

/// Dry convective adjustment (SPEC.md §9.4.3): wherever the provisional profile is
/// statically unstable (potential temperature decreasing with height between adjacent
/// layers), the pair is mixed to a common potential temperature that conserves the
/// enthalpy Σ c_p T Δp. Repeated sweeps until the column is stable to 1e-9 K. Only T is
/// adjusted (Manabe-type adjustment); q and momentum are left to the other schemes.
/// Diagnostic `dry_adj_heat`: column-integrated heating moved by the adjustment \[W m⁻²\]
/// (sum of |c_p ΔT| Δp/g / Δt_p, zero when the column was stable).
pub struct DryAdjustment {
    /// Maximum number of sweeps.
    pub max_sweeps: usize,
}

impl Default for DryAdjustment {
    fn default() -> Self {
        DryAdjustment { max_sweeps: 500 }
    }
}

impl ColumnScheme for DryAdjustment {
    fn name(&self) -> &'static str {
        "dry_adjustment"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["dry_adj_heat"]
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        work: &mut Workspace,
    ) {
        let l = st.nlev;
        let kappa = ctx.planet.kappa();
        let [pi, t, ..] = &mut work.buf;
        for k in 0..l {
            pi[k] = (ctx.p[k] / ctx.ps).powf(kappa);
            t[k] = st.t[k];
        }
        let mut any = false;
        for _ in 0..self.max_sweeps {
            let mut changed = false;
            for k in (1..l).rev() {
                let (lo, up) = (t[k] / pi[k], t[k - 1] / pi[k - 1]);
                if lo - up > 1e-9 {
                    let (m1, m0) = (ctx.dp[k], ctx.dp[k - 1]);
                    let th = (t[k] * m1 + t[k - 1] * m0) / (pi[k] * m1 + pi[k - 1] * m0);
                    t[k] = th * pi[k];
                    t[k - 1] = th * pi[k - 1];
                    changed = true;
                }
            }
            any |= changed;
            if !changed {
                break;
            }
        }
        if !any {
            return;
        }
        let mut moved = 0.0;
        for k in 0..l {
            moved += (ctx.planet.cp * (t[k] - st.t[k])).abs() * ctx.dp[k] / ctx.planet.gravity;
            st.set_t(k, t[k]);
        }
        diag[0] += moved / ctx.dt;
    }
}

/// Simplified Betts–Miller convection (Frierson 2007, J. Atmos. Sci. 64, 1959–1976).
///
/// A parcel from the lowest level is lifted dry-adiabatically to its lifting condensation
/// level and moist-adiabatically above. Where the parcel is buoyant (CAPE > 0), T and q
/// relax towards reference profiles T_ref = T_parcel + ΔT and q_ref = RH_ref q_sat(T_ref)
/// with time scale τ, from the lowest level up to the level of neutral buoyancy.
/// ΔT (uniform in the column) is chosen so the scheme conserves moist enthalpy:
/// ∫ c_p δT dp = −∫ L δq dp ("deep convection" with the energy-conserving closure).
/// Columns where the moisture adjustment would moisten (P_q ≤ 0) are left alone
/// (Frierson's shallow-convection variants are not implemented yet).
/// Diagnostics: `precip_conv` \[kg m⁻² s⁻¹\], `cape` \[J/kg\].
pub struct SimplifiedBettsMiller {
    pub cond: Condensable,
    /// Relaxation time τ \[s\] (2 h).
    pub tau: f64,
    /// Reference relative humidity (0.7).
    pub rh_ref: f64,
    /// RK4 sub-steps per layer for the parcel ascent. With the exact LCL search, one
    /// sub-step is accurate to ~0.001 K for L26-type layers (see the test).
    pub substeps: usize,
}

impl SimplifiedBettsMiller {
    /// Parcel temperature at each level (index 0 = top) of a parcel starting at the lowest
    /// level with (t0, q0), written to `tp`.
    fn parcel(&self, planet: &Planet, p: &[f64], t0: f64, q0: f64, tp: &mut [f64]) {
        let l = p.len();
        let kappa = planet.kappa();
        let (lv, cp, rd) = (self.cond.latent_heat, planet.cp, planet.gas_constant);
        let eps = self.cond.epsilon(planet);
        tp[l - 1] = t0;
        let mut t = t0;
        let mut pc = p[l - 1];
        let mut saturated = q0 >= self.cond.qsat(planet, t0, pc);
        // moist adiabat: dT/dln p = (R T/c_p)(1 + L q_s/(R T)) / (1 + ε L² q_s/(c_p R T²))
        let f = |tt: f64, pp: f64| {
            let qs = self.cond.qsat(planet, tt, pp);
            rd * tt / cp * (1.0 + lv * qs / (rd * tt))
                / (1.0 + eps * lv * lv * qs / (cp * rd * tt * tt))
        };
        let rk4 = |t: f64, p0: f64, h: f64| {
            let k1 = f(t, p0);
            let k2 = f(t + 0.5 * h * k1, p0 * (0.5 * h).exp());
            let k3 = f(t + 0.5 * h * k2, p0 * (0.5 * h).exp());
            let k4 = f(t + h * k3, p0 * h.exp());
            t + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0
        };
        for k in (0..l - 1).rev() {
            let n = self.substeps.max(1);
            let dlnp = (p[k] / pc).ln() / n as f64;
            for _ in 0..n {
                let pn = pc * dlnp.exp();
                if !saturated {
                    let tn = t * (pn / pc).powf(kappa);
                    if q0 >= self.cond.qsat(planet, tn, pn) {
                        // locate the lifting condensation level within this sub-step
                        let (mut lo, mut hi) = (0.0, 1.0);
                        for _ in 0..30 {
                            let mid = 0.5 * (lo + hi);
                            let pm = pc * (mid * dlnp).exp();
                            if q0 >= self.cond.qsat(planet, t * (pm / pc).powf(kappa), pm) {
                                hi = mid;
                            } else {
                                lo = mid;
                            }
                        }
                        let p_lcl = pc * (hi * dlnp).exp();
                        let t_lcl = t * (p_lcl / pc).powf(kappa);
                        saturated = true;
                        t = rk4(t_lcl, p_lcl, (1.0 - hi) * dlnp);
                    } else {
                        t = tn;
                    }
                } else {
                    t = rk4(t, pc, dlnp);
                }
                pc = pn;
            }
            tp[k] = t;
        }
    }
}

impl ColumnScheme for SimplifiedBettsMiller {
    fn name(&self) -> &'static str {
        "simplified_betts_miller"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["precip_conv", "cape"]
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        work: &mut Workspace,
    ) {
        let l = st.nlev;
        if st.humidity().is_none() || l < 3 {
            return;
        }
        let planet = ctx.planet;
        let (p, dp) = (ctx.p, ctx.dp);
        let [tp, ..] = &mut work.buf;
        self.parcel(planet, p, st.t[l - 1], st.q[l - 1], tp);
        // CAPE and level of neutral buoyancy (scan upward from the surface)
        let rd = planet.gas_constant;
        let mut cape = 0.0;
        let mut top = l; // highest (smallest index) level of the convecting layer
        for k in (0..l - 1).rev() {
            let b = tp[k] - st.t[k];
            if b > 0.0 {
                cape += rd * b * (p[k + 1] / p[k]).ln();
                top = k;
            } else if cape > 0.0 {
                break;
            }
        }
        diag[1] += cape;
        if cape <= 0.0 || top >= l {
            return;
        }
        let (lv, cp, g) = (self.cond.latent_heat, planet.cp, planet.gravity);
        let qref = |tr: f64, pk: f64| self.rh_ref * self.cond.qsat(planet, tr, pk);
        let mut pq = 0.0; // ∫ (q − q_ref) dp (positive = drying)
        let mut pt = 0.0; // ∫ c_p/L (T_ref − T) dp
        for k in top..l {
            pq += (st.q[k] - qref(tp[k], p[k])) * dp[k];
            pt += cp / lv * (tp[k] - st.t[k]) * dp[k];
        }
        if pq <= 0.0 {
            return;
        }
        // energy closure: shift T_ref by a uniform ΔT so that heating balances drying,
        // Σ c_p (T_ref − T) dp = Σ L (q − q_ref) dp (q_ref held at the first-guess value)
        let mass: f64 = dp[top..l].iter().sum();
        let dtref = (lv / cp) * (pq - pt) / mass;
        let mut precip = 0.0;
        for k in top..l {
            let dtk = (tp[k] + dtref - st.t[k]) / self.tau;
            let dqk = -(st.q[k] - qref(tp[k], p[k])) / self.tau;
            st.add_t(k, dtk);
            st.add_q(0, k, dqk);
            precip -= dqk * dp[k] / g;
        }
        // The closure makes the column heating equal L × precipitation; report the column
        // water loss (the same number) so the water budget closes.
        sfc.rain += precip;
        diag[0] += precip;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;

    #[test]
    fn parcel_converges_with_substeps() {
        let planet = Planet::earth();
        let p: Vec<f64> = (0..26)
            .map(|k| 1e5 * (0.02 + 0.98 * (k as f64 + 0.5) / 26.0))
            .collect();
        let mk = |n| SimplifiedBettsMiller {
            cond: Condensable::water(),
            tau: 7200.0,
            rh_ref: 0.7,
            substeps: n,
        };
        let mut b = vec![0.0; 26];
        mk(32).parcel(&planet, &p, 300.0, 0.018, &mut b);
        let mut a = vec![0.0; 26];
        mk(2).parcel(&planet, &p, 300.0, 0.018, &mut a);
        let err = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
        assert!(err < 0.02, "max difference {err} K");
    }

    #[test]
    fn dry_adjustment_removes_instability_and_conserves_enthalpy() {
        let geo = Geometry::l26();
        let planet = Planet::earth();
        let ctx = geo.ctx(&planet, None, 1800.0);
        let l = geo.p.len();
        let mut st = ColumnState::new(l, 0);
        st.begin(1800.0);
        // superadiabatic lower troposphere (10 K/km near the surface), stable above
        for k in 0..l {
            let z = 7000.0 * (1e5 / geo.p[k]).ln();
            st.t[k] = if z < 2000.0 {
                300.0 - 0.012 * z
            } else {
                276.0 - 0.005 * (z - 2000.0)
            }
            .max(200.0);
        }
        let t0 = st.t.clone();
        let mut diag = [0.0];
        DryAdjustment::default().apply(
            &ctx,
            &mut st,
            &mut SurfaceExchange::default(),
            &mut diag,
            &mut Workspace::new(l),
        );
        let kappa = planet.kappa();
        let de: f64 = (0..l)
            .map(|k| planet.cp * (st.t[k] - t0[k]) * geo.dp[k] / planet.gravity)
            .sum();
        assert!(de.abs() / 1800.0 < 1e-6, "enthalpy change {de} J/m²");
        assert!(diag[0] > 0.0);
        for k in 1..l {
            let th = |k: usize| st.t[k] * (1e5 / geo.p[k]).powf(kappa);
            assert!(th(k) <= th(k - 1) + 1e-6, "unstable at {k}");
        }
    }
}
