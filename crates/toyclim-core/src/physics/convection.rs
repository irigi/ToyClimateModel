//! Simplified Betts–Miller convection (Frierson 2007, J. Atmos. Sci. 64, 1959–1976).
//!
//! A parcel from the lowest level is lifted dry-adiabatically to its lifting condensation
//! level and moist-adiabatically above. Where the parcel is buoyant (CAPE > 0), T and q
//! relax towards reference profiles T_ref = T_parcel + ΔT and q_ref = RH_ref q_sat(T_ref)
//! with time scale τ, from the lowest level up to the level of neutral buoyancy.
//! ΔT (uniform in the column) is chosen so the scheme conserves moist enthalpy:
//! ∫ c_p δT dp = −∫ L δq dp ("deep convection" with the energy-conserving closure).
//! Columns where the moisture adjustment would moisten (P_q ≤ 0) are left alone
//! (Frierson's shallow-convection variants are not implemented).
//! Diagnostics: `precip_conv` \[kg m⁻² s⁻¹\], `cape` \[J/kg\].

use super::moist::Condensable;
use super::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::Real;

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
    /// Parcel temperature at each level (index 0 = top) for a parcel starting at the
    /// lowest level, plus the LCL level index (lowest level index where it is saturated).
    fn parcel(&self, planet: &Planet, p: &[f64], t0: f64, q0: f64) -> Vec<f64> {
        let l = p.len();
        let kappa = planet.kappa();
        let (lv, cp, rd) = (self.cond.latent_heat, planet.cp, planet.gas_constant);
        let eps = self.cond.epsilon(planet);
        let mut tp = vec![0.0; l];
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
        tp
    }
}

impl ColumnPhysics for SimplifiedBettsMiller {
    fn name(&self) -> &'static str {
        "simplified_betts_miller"
    }
    fn diagnostic_names(&self) -> Vec<&'static str> {
        vec!["precip_conv", "cape"]
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let l = col.t.len();
        if col.tracers.len() < l || l < 3 {
            return;
        }
        let ps = col.ps as f64;
        let p: Vec<f64> = col.sigma.iter().map(|&s| s as f64 * ps).collect();
        let dp: Vec<f64> = (0..l)
            .map(|k| (col.sigma_half[k + 1] - col.sigma_half[k]) as f64 * ps)
            .collect();
        let t: Vec<f64> = col.t.iter().map(|&x| x as f64).collect();
        let q: Vec<f64> = col.tracers[..l].iter().map(|&x| x as f64).collect();
        let tp = self.parcel(planet, &p, t[l - 1], q[l - 1]);
        // CAPE and level of neutral buoyancy (scan upward from the surface)
        let rd = planet.gas_constant;
        let mut cape = 0.0;
        let mut top = l; // highest (smallest index) level of the convecting layer
        for k in (0..l - 1).rev() {
            let b = tp[k] - t[k];
            if b > 0.0 {
                cape += rd * b * (p[k + 1] / p[k]).ln();
                top = k;
            } else if cape > 0.0 {
                break;
            }
        }
        out.diag[1] += cape as Real;
        if cape <= 0.0 || top >= l {
            return;
        }
        let (lv, cp, g) = (self.cond.latent_heat, planet.cp, planet.gravity);
        let tau = self.tau;
        // first guess with ΔT = 0
        let qref = |tr: f64, pk: f64| self.rh_ref * self.cond.qsat(planet, tr, pk);
        let mut pq = 0.0; // ∫ −δq dp/g · τ … (positive = drying)
        let mut pt = 0.0; // ∫ c_p/L δT dp/g
        for k in top..l {
            pq += (q[k] - qref(tp[k], p[k])) * dp[k];
            pt += cp / lv * (tp[k] - t[k]) * dp[k];
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
            let tr = tp[k] + dtref;
            let dtk = (tr - t[k]) / tau;
            let dqk = -(q[k] - qref(tp[k], p[k])) / tau;
            out.dt[k] += dtk as Real;
            out.dtracers[k] += dqk as Real;
            precip -= dqk * dp[k] / g;
        }
        out.diag[0] += precip.max(0.0) as Real;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let b = mk(32).parcel(&planet, &p, 300.0, 0.018);
        for n in [1, 2, 4, 8, 16] {
            let a = mk(n).parcel(&planet, &p, 300.0, 0.018);
            let err = a
                .iter()
                .zip(&b)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f64::max);
            eprintln!("substeps {n}: max parcel T difference {err:.4} K");
        }
        let a = mk(2).parcel(&planet, &p, 300.0, 0.018);
        let err = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max);
        assert!(err < 0.02, "max difference {err} K");
    }
}
