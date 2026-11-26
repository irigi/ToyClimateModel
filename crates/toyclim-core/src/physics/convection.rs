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

/// Shallow-convection variant of [`SimplifiedBettsMiller`] (Frierson 2007, §2c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shallow {
    /// Columns with P_q ≤ 0 are left alone.
    None,
    /// Lowered depth p_shall (eqs. 7–9), the paper's control.
    Shallower,
    /// Full depth, humidity reference scaled (eqs. 10–15).
    Qref,
}

/// Simplified Betts–Miller convection (Frierson 2007, J. Atmos. Sci. 64, 1959–1976).
///
/// A parcel from the lowest level is lifted dry-adiabatically to its lifting condensation
/// level and moist-adiabatically above. Where the parcel is buoyant (CAPE > 0), T and q
/// relax towards reference profiles T_ref = T_parcel + ΔT and q_ref = RH_ref q_sat(T_ref)
/// with time scale τ, from the lowest level up to the level of neutral buoyancy.
/// ΔT (uniform in the column) is chosen so the scheme conserves moist enthalpy:
/// ∫ c_p δT dp = −∫ L δq dp ("deep convection" with the energy-conserving closure).
/// Columns where the moisture adjustment would moisten (P_q ≤ 0) get Frierson's (2007,
/// J. Atmos. Sci. 64, 1959–1976, §2c, eqs. 7–9) "shallower" shallow convection when
/// `shallow` is on (his control) and P_T > 0: the top is lowered to the deepest level
/// p_shall where ∫_{p0}^{p_shall} (q − q_ref) dp = 0 (eq. 7; the crossing layer enters with
/// the fraction that makes the integral exactly zero, so the precipitation is identically
/// zero), and T, q relax within that layer with T_ref2 = T_ref − Δk/c_p (eqs. 8–9; with eq. 7
/// this makes the net heating zero). Moisture is moved upward, nothing precipitates. When
/// no such level exists (the lowest layer is already drier than q_ref), nothing is done.
/// The paper's other variant, the "qref" scheme (§2c, eqs. 10–15), keeps the full depth to
/// the level of zero buoyancy and scales the humidity reference by a uniform fraction,
/// q_ref2 = f_q q_ref with f_q = ∫ q dp / ∫ q_ref dp (eqs. 10–13: zero net drying), and shifts
/// T_ref by a uniform amount so that the net heating is zero (eqs. 14–15; the paper's sign
/// convention for Δp is not legible in the copy, the condition "predicted precipitation zero"
/// fixes it). `shallow` none (the default): those columns are left alone (the P1 behaviour).
/// Diagnostics: `precip_conv` \[kg m⁻² s⁻¹\], `cape` \[J/kg\]; indicators (1 in the step,
/// else 0; their time means are frequencies) `conv_active` (deep convection),
/// `conv_skipped` (CAPE > 0 but P_q ≤ 0 and no adjustment) and `conv_shallow` (shallow
/// convection done).
///
/// Deep convection runs for P_q > 0 regardless of P_T (the P1 behaviour), unless
/// `deep_requires_pt` is set: then also P_T > 0 is required, as in the paper (§2a, after
/// eq. 4: "The simplest criterion for deep convection is that both P_T and P_q are
/// positive"; §2b "Deep convection: P_T > 0, P_q > 0"); columns with P_q > 0 and
/// P_T ≤ 0 are then left alone and counted in `conv_skipped`. Off by default (P4
/// question 13 side item; the owner decides after seeing the APE numbers).
pub struct SimplifiedBettsMiller {
    pub cond: Condensable,
    /// Relaxation time τ \[s\] (2 h).
    pub tau: f64,
    /// Reference relative humidity (0.7).
    pub rh_ref: f64,
    /// Shallow convection for P_q ≤ 0, P_T > 0 (key `shallow = "shallower" | "qref" |
    /// "none"`; none by default since 2026-11-22, P4 question 13).
    pub shallow: Shallow,
    /// Require P_T > 0 for deep convection (key `deep_requires_pt`, default false).
    pub deep_requires_pt: bool,
    /// Diagnostic switch (key `keep_lowest`, default 0; P4 question 13, 2026-11-26): deep
    /// convection leaves the lowest `keep_lowest` levels unchanged. The parcel still starts
    /// at the lowest level; P_q, P_T, the energy closure and the relaxation use only the
    /// levels from the top down to the last one above them, so the scheme still conserves
    /// moist enthalpy. Not a published variant; only with `shallow = "none"`.
    pub keep_lowest: usize,
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

impl SimplifiedBettsMiller {
    /// Frierson's "shallower" shallow convection (see the type's doc comment) for a column
    /// with CAPE > 0 whose moisture adjustment over `top..l` would moisten. Returns false
    /// (nothing done) when no level p_shall with zero precipitation exists.
    fn shallow_adjust(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        tp: &[f64],
        top: usize,
    ) -> bool {
        let l = st.nlev;
        let planet = ctx.planet;
        let (p, dp) = (ctx.p, ctx.dp);
        let qref = |k: usize| self.rh_ref * self.cond.qsat(planet, tp[k], p[k]);
        let d = |k: usize| (st.q[k] - qref(k)) * dp[k];
        // cumulative from the surface: c = Σ_{k=j}^{l-1} d_k; find the highest j > top with
        // c(j) > 0 (scanning down from the LZB), the zero lies in layer j − 1
        let mut c = 0.0;
        let mut cum = vec![0.0; l + 1];
        for k in (0..l).rev() {
            c += d(k);
            cum[k] = c;
        }
        let Some(j) = (top + 1..l).find(|&j| cum[j] > 0.0) else {
            return false;
        };
        // fraction of layer j − 1 that makes the integral exactly zero (d_{j−1} < 0)
        let f = (cum[j] / -d(j - 1)).clamp(0.0, 1.0);
        let w = |k: usize| if k == j - 1 { f } else { 1.0 };
        let (mut mass, mut heat) = (0.0, 0.0);
        for k in j - 1..l {
            mass += w(k) * dp[k];
            heat += w(k) * (tp[k] - st.t[k]) * dp[k];
        }
        // T_ref2 = T_ref − Δk/c_p with zero column heating (eq. 8; ∫ L (q − q_ref) = 0)
        let dtref = -heat / mass;
        for k in j - 1..l {
            let dtk = w(k) * (tp[k] + dtref - st.t[k]) / self.tau;
            let dqk = -w(k) * (st.q[k] - qref(k)) / self.tau;
            st.add_t(k, dtk);
            st.add_q(0, k, dqk);
        }
        true
    }

    /// Frierson's "qref" shallow convection (see the type's doc comment) over `top..l`.
    /// Returns false when the column holds no reference humidity (nothing done).
    fn qref_adjust(&self, ctx: &ColumnCtx, st: &mut ColumnState, tp: &[f64], top: usize) -> bool {
        let l = st.nlev;
        let planet = ctx.planet;
        let (p, dp) = (ctx.p, ctx.dp);
        let (mut qsum, mut qrsum, mut heat, mut mass) = (0.0, 0.0, 0.0, 0.0);
        for k in top..l {
            qsum += st.q[k] * dp[k];
            qrsum += self.rh_ref * self.cond.qsat(planet, tp[k], p[k]) * dp[k];
            heat += (tp[k] - st.t[k]) * dp[k];
            mass += dp[k];
        }
        if qrsum <= 0.0 {
            return false;
        }
        let fq = qsum / qrsum; // eqs. 12–13
        let dtref = -heat / mass; // eqs. 14–15: zero net heating
        for k in top..l {
            let qref = fq * self.rh_ref * self.cond.qsat(planet, tp[k], p[k]);
            st.add_t(k, (tp[k] + dtref - st.t[k]) / self.tau);
            st.add_q(0, k, -(st.q[k] - qref) / self.tau);
        }
        true
    }
}

impl ColumnScheme for SimplifiedBettsMiller {
    fn name(&self) -> &'static str {
        "simplified_betts_miller"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &[
            "precip_conv",
            "cape",
            "conv_active",
            "conv_skipped",
            "conv_shallow",
        ]
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
        // bottom (exclusive) of the levels convection modifies
        let kb = l - self.keep_lowest.min(l);
        if top >= kb {
            diag[3] += 1.0;
            return;
        }
        let mut pq = 0.0; // ∫ (q − q_ref) dp (positive = drying)
        let mut pt = 0.0; // ∫ c_p/L (T_ref − T) dp
        for k in top..kb {
            pq += (st.q[k] - qref(tp[k], p[k])) * dp[k];
            pt += cp / lv * (tp[k] - st.t[k]) * dp[k];
        }
        if pq <= 0.0 {
            let done = pt > 0.0
                && match self.shallow {
                    Shallow::None => false,
                    Shallow::Shallower => self.shallow_adjust(ctx, st, tp, top),
                    Shallow::Qref => self.qref_adjust(ctx, st, tp, top),
                };
            if done {
                diag[4] += 1.0;
            } else {
                diag[3] += 1.0;
            }
            return;
        }
        if self.deep_requires_pt && pt <= 0.0 {
            diag[3] += 1.0;
            return;
        }
        diag[2] += 1.0;
        // energy closure: shift T_ref by a uniform ΔT so that heating balances drying,
        // Σ c_p (T_ref − T) dp = Σ L (q − q_ref) dp (q_ref held at the first-guess value)
        let mass: f64 = dp[top..kb].iter().sum();
        let dtref = (lv / cp) * (pq - pt) / mass;
        let (mut precip, mut rain, mut snow) = (0.0, 0.0, 0.0);
        let lf = self.cond.latent_heat_fusion;
        for k in top..kb {
            let dtk = (tp[k] + dtref - st.t[k]) / self.tau;
            let dqk = -(st.q[k] - qref(tp[k], p[k])) / self.tau;
            // condensate (net drying) of the layer freezes with f_ice of the layer
            let fi = self.cond.ice_fraction(st.t[k]);
            let fusion = if dqk < 0.0 { -fi * lf / cp * dqk } else { 0.0 };
            st.add_t(k, dtk + fusion);
            st.add_q(0, k, dqk);
            let m = -dqk * dp[k] / g;
            let fi = if dqk < 0.0 { fi } else { 0.0 };
            st.produce(k, m, fi);
            precip += m;
            rain += (1.0 - fi) * m;
            snow += fi * m;
        }
        // The closure makes the column heating equal L × precipitation; report the column
        // water loss (the same number) so the water budget closes.
        sfc.rain += rain;
        sfc.snow += snow;
        diag[0] += precip;
        st.conv_precip += precip;
        st.conv_top = st.conv_top.min(top);
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
            shallow: Shallow::None,
            deep_requires_pt: false,
            keep_lowest: 0,
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

    /// Frierson's "shallower" and "qref" shallow convection on a column with a moist
    /// boundary layer under a dry free troposphere (CAPE > 0, P_q < 0): no precipitation,
    /// column moisture and enthalpy conserved, moisture moved upward, and nothing at all
    /// without shallow convection.
    #[test]
    fn shallow_convection_conserves_and_moistens_aloft() {
        let geo = Geometry::l26();
        let planet = Planet::earth();
        let cond = Condensable::water();
        let ctx = geo.ctx(&planet, None, 1800.0);
        let l = geo.p.len();
        let run = |shallow: Shallow| {
            let sbm = SimplifiedBettsMiller {
                cond: cond.clone(),
                tau: 7200.0,
                rh_ref: 0.7,
                shallow,
                deep_requires_pt: false,
                keep_lowest: 0,
                substeps: 1,
            };
            let mut st = ColumnState::new(l, 1);
            st.begin(1800.0);
            // environment 1 K cooler than the parcel from (300 K, q_s at RH 0.9): buoyant;
            // moist below σ 0.85 (RH 0.9), very dry above (RH 0.1)
            let q0 = 0.9 * cond.qsat(&planet, 300.0, geo.p[l - 1]);
            let mut tp = vec![0.0; l];
            sbm.parcel(&planet, &geo.p, 300.0, q0, &mut tp);
            for k in 0..l {
                st.t[k] = if k == l - 1 {
                    300.0
                } else {
                    (tp[k] - 1.0).max(200.0)
                };
                let rh = if geo.p[k] > 0.85e5 { 0.9 } else { 0.1 };
                st.q[k] = rh * cond.qsat(&planet, st.t[k], geo.p[k]);
            }
            let (t0, q0v) = (st.t.clone(), st.q.clone());
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 5];
            sbm.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(l));
            (st, t0, q0v, sfc, diag)
        };
        for variant in [Shallow::Shallower, Shallow::Qref] {
            let (st, t0, q0, sfc, diag) = run(variant);
            assert!(diag[1] > 0.0, "CAPE {}", diag[1]);
            assert_eq!(diag[2], 0.0, "deep convection fired");
            assert_eq!(
                diag[4], 1.0,
                "{variant:?}: shallow convection not done: {diag:?}"
            );
            assert_eq!(diag[0], 0.0);
            assert_eq!(sfc.rain + sfc.snow, 0.0);
            let g = planet.gravity;
            let dw: f64 = (0..l).map(|k| (st.q[k] - q0[k]) * geo.dp[k] / g).sum();
            let dh: f64 = (0..l)
                .map(|k| planet.cp * (st.t[k] - t0[k]) * geo.dp[k] / g)
                .sum();
            let moved: f64 = (0..l)
                .map(|k| (st.q[k] - q0[k]).abs() * geo.dp[k] / g)
                .sum();
            assert!(moved > 1e-3, "nothing moved: {moved} kg/m²");
            assert!(
                dw.abs() < 1e-9 * moved.max(1.0) + 1e-12,
                "water change {dw} kg/m²"
            );
            assert!(dh.abs() < 1e-6, "enthalpy change {dh} J/m²");
            assert!(st.q[l - 1] < q0[l - 1], "boundary layer not dried");
            let up = (0..l - 1).any(|k| geo.p[k] < 0.85e5 && st.q[k] > q0[k]);
            assert!(up, "nothing moistened above the boundary layer");
        }
        let (st2, t2, q2, _, diag2) = run(Shallow::None);
        assert_eq!(diag2[3], 1.0);
        assert_eq!(diag2[4], 0.0);
        assert_eq!(st2.t, t2);
        assert_eq!(st2.q, q2);
    }
    /// With `deep_requires_pt`, a moist column (P_q > 0) whose buoyant layer sits above a
    /// deeper cold layer (P_T < 0) is left alone and counted as skipped; without the key it
    /// convects (the P1 behaviour).
    #[test]
    fn deep_convection_requires_pt_when_asked() {
        let geo = Geometry::l26();
        let planet = Planet::earth();
        let cond = Condensable::water();
        let ctx = geo.ctx(&planet, None, 1800.0);
        let l = geo.p.len();
        let run = |req: bool| {
            let sbm = SimplifiedBettsMiller {
                cond: cond.clone(),
                tau: 7200.0,
                rh_ref: 0.7,
                shallow: Shallow::None,
                deep_requires_pt: req,
                keep_lowest: 0,
                substeps: 1,
            };
            let mut st = ColumnState::new(l, 1);
            st.begin(1800.0);
            let q0 = 0.95 * cond.qsat(&planet, 300.0, geo.p[l - 1]);
            let mut tp = vec![0.0; l];
            sbm.parcel(&planet, &geo.p, 300.0, q0, &mut tp);
            // levels l-2 .. l-7 warmer than the parcel (inhibition, 3 K), l-8 and l-9
            // slightly buoyant (0.3 K), stable above
            for k in 0..l {
                st.t[k] = if k == l - 1 {
                    300.0
                } else if k >= l - 7 {
                    tp[k] + 3.0
                } else if k >= l - 9 {
                    tp[k] - 0.3
                } else {
                    (tp[k] + 5.0).max(200.0)
                };
                st.q[k] = 0.95 * cond.qsat(&planet, st.t[k], geo.p[k]);
            }
            let (t0, qv) = (st.t.clone(), st.q.clone());
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 5];
            sbm.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(l));
            (st, t0, qv, diag)
        };
        let (_, _, _, d) = run(false);
        assert!(d[1] > 0.0, "CAPE {}", d[1]);
        assert_eq!(d[2], 1.0, "deep convection expected without the key: {d:?}");
        let (st, t0, q0, d) = run(true);
        assert_eq!(d[2], 0.0);
        assert_eq!(d[3], 1.0, "{d:?}");
        assert_eq!(d[0], 0.0);
        assert_eq!(st.t, t0);
        assert_eq!(st.q, q0);
    }
    /// `keep_lowest`: deep convection leaves the lowest levels unchanged and still
    /// conserves moist enthalpy, and the column water lost equals the precipitation.
    #[test]
    fn keep_lowest_leaves_bottom_levels_and_conserves() {
        let geo = Geometry::l26();
        let planet = Planet::earth();
        let cond = Condensable::water();
        let ctx = geo.ctx(&planet, Some(&cond), 1800.0);
        let l = geo.p.len();
        let run = |n: usize| {
            let sbm = SimplifiedBettsMiller {
                cond: cond.clone(),
                tau: 7200.0,
                rh_ref: 0.7,
                shallow: Shallow::None,
                deep_requires_pt: false,
                keep_lowest: n,
                substeps: 1,
            };
            let mut st = ColumnState::new(l, 1);
            st.begin(1800.0);
            let q0 = 0.9 * cond.qsat(&planet, 300.0, geo.p[l - 1]);
            let mut tp = vec![0.0; l];
            sbm.parcel(&planet, &geo.p, 300.0, q0, &mut tp);
            for k in 0..l {
                st.t[k] = if k == l - 1 {
                    300.0
                } else {
                    (tp[k] - 1.0).max(200.0)
                };
                st.q[k] = 0.9 * cond.qsat(&planet, st.t[k], geo.p[k]);
            }
            let (t0, qv) = (st.t.clone(), st.q.clone());
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 5];
            sbm.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(l));
            (st, t0, qv, sfc, diag)
        };
        let (st0, _, q00, _, d0) = run(0);
        assert_eq!(d0[2], 1.0);
        assert!(
            st0.q[l - 1] < q00[l - 1],
            "lowest level not dried without the key"
        );
        for n in [1, 2] {
            let (st, t0, q0, sfc, d) = run(n);
            assert_eq!(d[2], 1.0, "{d:?}");
            for k in l - n..l {
                assert_eq!(st.t[k], t0[k]);
                assert_eq!(st.q[k], q0[k]);
            }
            let g = planet.gravity;
            let dw: f64 = (0..l).map(|k| (st.q[k] - q0[k]) * geo.dp[k] / g).sum();
            let dh: f64 = (0..l)
                .map(|k| {
                    planet.cp * (st.t[k] - t0[k]) * geo.dp[k] / g
                        + cond.latent_heat * (st.q[k] - q0[k]) * geo.dp[k] / g
                })
                .sum();
            let pr = (sfc.rain + sfc.snow) * 1800.0;
            assert!(pr > 0.0);
            assert!((dw + pr).abs() < 1e-9 * pr, "water {dw} vs precip {pr}");
            // ice formed above the freezing level releases fusion heat: allow it
            let lf = cond.latent_heat_fusion * sfc.snow * 1800.0;
            assert!(
                (dh - lf).abs() < 1e-6 * (cond.latent_heat * pr),
                "enthalpy {dh}"
            );
        }
    }
}
