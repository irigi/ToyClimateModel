//! The condensable species and large-scale condensation (SPEC.md §9.4.4).
//!
//! Tracer 0 is the specific humidity q \[kg/kg\] of the condensable. Virtual-temperature
//! effects on the dynamics are neglected (dry dynamical core with a moisture tracer).
//! One saturation curve (over liquid, fixed latent heat) is used at all temperatures.

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use crate::planet::Planet;
use serde::{Deserialize, Serialize};

/// Thermodynamic properties of the condensing species. Data, not code: water is the
/// tested preset; other species (e.g. methane) only need different numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condensable {
    pub name: String,
    /// Latent heat of vaporization L \[J/kg\].
    pub latent_heat: f64,
    /// Gas constant of the vapour R_v \[J kg⁻¹ K⁻¹\].
    pub r_vapor: f64,
    /// Saturation vapour pressure e₀ at T₀ \[Pa\].
    pub e0: f64,
    /// Reference temperature T₀ \[K\].
    pub t0: f64,
    /// Density of the liquid \[kg/m³\] (precipitation depth).
    pub rho_liquid: f64,
}

impl Condensable {
    /// Water with the constants of Thatcher & Jablonowski (2016, eqs. 1–5).
    pub fn water() -> Self {
        Condensable {
            name: "water".into(),
            latent_heat: 2.5e6,
            r_vapor: 461.5,
            e0: 610.78,
            t0: 273.16,
            rho_liquid: 1000.0,
        }
    }

    pub fn preset(name: &str) -> Option<Self> {
        match name {
            "water" => Some(Self::water()),
            _ => None,
        }
    }

    /// ε = R_d / R_v.
    pub fn epsilon(&self, planet: &Planet) -> f64 {
        planet.gas_constant / self.r_vapor
    }

    /// Saturation specific humidity q_sat(T, p) = (ε e₀ / p) exp(−L/R_v (1/T − 1/T₀))
    /// (TJ16 eq. 1).
    pub fn qsat(&self, planet: &Planet, t: f64, p: f64) -> f64 {
        self.epsilon(planet) * self.e0 / p
            * (-(self.latent_heat / self.r_vapor) * (1.0 / t - 1.0 / self.t0)).exp()
    }

    /// dq_sat/dT = q_sat L / (R_v T²).
    pub fn dqsat_dt(&self, qs: f64, t: f64) -> f64 {
        qs * self.latent_heat / (self.r_vapor * t * t)
    }
}

/// Large-scale condensation: supersaturation is removed in one physics step, latent heat
/// warms the air, the condensate falls out immediately (no clouds, no re-evaporation).
/// The removal is linearised about the provisional state, TJ16 eq. (2):
/// C = (q − q_s) / (Δt_p (1 + L² q_s / (c_p R_v T²))).
/// Diagnostic `precip_ls` \[kg m⁻² s⁻¹\].
pub struct LargeScaleCondensation {
    pub cond: Condensable,
}

impl ColumnScheme for LargeScaleCondensation {
    fn name(&self) -> &'static str {
        "large_scale_condensation"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["precip_ls"]
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        if st.humidity().is_none() {
            return;
        }
        let planet = ctx.planet;
        let (lv, rv, cp) = (self.cond.latent_heat, self.cond.r_vapor, planet.cp);
        let mut precip = 0.0;
        for k in 0..st.nlev {
            let (t, q) = (st.t[k], st.q[k]);
            let qs = self.cond.qsat(planet, t, ctx.p[k]);
            if q > qs {
                let c = (q - qs) / (ctx.dt * (1.0 + lv * lv * qs / (cp * rv * t * t)));
                st.add_t(k, lv / cp * c);
                st.add_q(0, k, -c);
                precip += c * ctx.dp[k] / planet.gravity;
            }
        }
        sfc.rain += precip;
        diag[0] += precip;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;

    #[test]
    fn qsat_water_values() {
        let w = Condensable::water();
        let e = Planet::earth();
        // at T0 and 1000 hPa: q_s ≈ ε e0 / p ≈ 0.622 · 6.1078 / 1000 ≈ 3.8 g/kg
        let q0 = w.qsat(&e, 273.16, 1e5);
        assert!((q0 - 0.6219 * 610.78 / 1e5).abs() < 1e-5, "{q0}");
        // ~ 20 g/kg at 300 K
        let q300 = w.qsat(&e, 300.0, 1e5);
        assert!(q300 > 0.018 && q300 < 0.026, "{q300}");
    }

    /// Condensation conserves moist enthalpy c_p T + L q per level, only removes
    /// supersaturation, and the column water lost equals the precipitation.
    #[test]
    fn condensation_conserves_moist_enthalpy_and_water() {
        let geo = Geometry::new(&[0.0, 0.25, 0.5, 0.75, 1.0], 1e5);
        let planet = Planet::earth();
        let w = Condensable::water();
        let ctx = geo.ctx(&planet, Some(&w), 1800.0);
        let mut st = ColumnState::new(4, 1);
        st.begin(1800.0);
        st.t.copy_from_slice(&[220.0, 260.0, 290.0, 300.0]);
        st.q.copy_from_slice(&[1e-5, 1e-3, 0.03, 0.001]);
        let (t0, q0) = (st.t.clone(), st.q.clone());
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0];
        LargeScaleCondensation { cond: w.clone() }.apply(
            &ctx,
            &mut st,
            &mut sfc,
            &mut diag,
            &mut Workspace::new(4),
        );
        let mut dw = 0.0;
        for k in 0..4 {
            let h = planet.cp * (st.t[k] - t0[k]) + w.latent_heat * (st.q[k] - q0[k]);
            assert!(h.abs() < 1e-9, "level {k}: {h}");
            dw += (st.q[k] - q0[k]) * geo.dp[k] / planet.gravity;
        }
        assert!(st.q[2] < q0[2], "supersaturated level must condense");
        assert_eq!(st.q[3], q0[3], "subsaturated level untouched");
        assert!((dw / 1800.0 + sfc.rain).abs() < 1e-12, "{dw} {}", sfc.rain);
        assert_eq!(diag[0], sfc.rain);
    }
}
