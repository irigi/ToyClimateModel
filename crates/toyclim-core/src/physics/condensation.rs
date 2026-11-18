//! The condensable species and large-scale condensation (SPEC.md §9.4.4).
//!
//! Tracer 0 is the specific humidity q \[kg/kg\] of the condensable. Virtual-temperature
//! effects on the dynamics are neglected (dry dynamical core with a moisture tracer).
//! One saturation curve (over liquid, fixed latent heat of vaporization) is used at all
//! temperatures, also where the condensate is ice (SPEC.md §9.4.4; documented choice).
//!
//! **Phase** (P2): condensate formed in a layer at temperature T is ice with the fraction
//! f_ice(T) = clamp((T_freeze − T)/ΔT_ice, 0, 1), ΔT_ice = 20 K **[knob]**; latent heating
//! uses L_v + f_ice L_f. Condensables without a latent heat of fusion (the TJ16 bundle)
//! produce liquid only.

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
    /// Latent heat of fusion L_f \[J/kg\] (0 = no ice phase).
    #[serde(default)]
    pub latent_heat_fusion: f64,
    /// Freezing (melting) temperature \[K\].
    #[serde(default = "default_t_freeze")]
    pub t_freeze: f64,
    /// Width of the mixed-phase range below `t_freeze` \[K\] (f_ice ramp).
    #[serde(default = "default_ice_ramp")]
    pub ice_ramp: f64,
}

fn default_t_freeze() -> f64 {
    273.15
}

fn default_ice_ramp() -> f64 {
    20.0
}

impl Condensable {
    /// Water: saturation curve and L_v from Thatcher & Jablonowski (2016, eqs. 1–5);
    /// L_f = 3.34e5 J/kg and T_freeze = 273.15 K (CAM6 Scientific Guide §8.2 table:
    /// L_i = 3.340×10⁵ J kg⁻¹, T_melt = 0 °C).
    pub fn water() -> Self {
        Condensable {
            name: "water".into(),
            latent_heat: 2.5e6,
            r_vapor: 461.5,
            e0: 610.78,
            t0: 273.16,
            rho_liquid: 1000.0,
            latent_heat_fusion: 3.34e5,
            t_freeze: default_t_freeze(),
            ice_ramp: default_ice_ramp(),
        }
    }

    /// The same condensable without an ice phase (idealized bundles, frozen physics).
    pub fn without_fusion(&self) -> Self {
        Condensable {
            latent_heat_fusion: 0.0,
            ..self.clone()
        }
    }

    /// Ice fraction of condensate formed at temperature `t`.
    pub fn ice_fraction(&self, t: f64) -> f64 {
        if self.latent_heat_fusion <= 0.0 {
            0.0
        } else {
            ((self.t_freeze - t) / self.ice_ramp).clamp(0.0, 1.0)
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
        let lf = self.cond.latent_heat_fusion;
        let (mut precip, mut rain, mut snow) = (0.0, 0.0, 0.0);
        for k in 0..st.nlev {
            let (t, q) = (st.t[k], st.q[k]);
            let qs = self.cond.qsat(planet, t, ctx.p[k]);
            if q > qs {
                let c = (q - qs) / (ctx.dt * (1.0 + lv * lv * qs / (cp * rv * t * t)));
                let fi = self.cond.ice_fraction(t);
                st.add_t(k, (lv + fi * lf) / cp * c);
                st.add_q(0, k, -c);
                let m = c * ctx.dp[k] / planet.gravity;
                st.produce(k, m, fi);
                precip += m;
                rain += (1.0 - fi) * m;
                snow += fi * m;
            }
        }
        sfc.rain += rain;
        sfc.snow += snow;
        diag[0] += precip;
    }
}

/// Precipitation fall (SPEC.md §9.4.4): the rain and snow produced in each layer by
/// convection and condensation are carried down layer by layer. In layers warmer than
/// T_freeze falling snow melts, cooling the layer by L_f per kg, limited so that the
/// layer does not cool below T_freeze. Rain does not refreeze; no re-evaporation (off by
/// default; `reevaporation` is a P5 knob). Hands rain and snow to the surface separately
/// (replacing the totals the producing schemes reported). Diagnostics: `prsn` (snowfall
/// at the surface) and `melt` (snow melted in the column) \[kg m⁻² s⁻¹\].
pub struct PrecipitationFall {
    pub cond: Condensable,
}

impl ColumnScheme for PrecipitationFall {
    fn name(&self) -> &'static str {
        "precipitation_fall"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["prsn", "melt"]
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
        let (cp, g) = (ctx.planet.cp, ctx.planet.gravity);
        let lf = self.cond.latent_heat_fusion;
        let (mut rain, mut snow, mut melted) = (0.0, 0.0, 0.0);
        for k in 0..st.nlev {
            rain += st.prod_rain[k];
            snow += st.prod_snow[k];
            st.prod_rain[k] = 0.0;
            st.prod_snow[k] = 0.0;
            if snow > 0.0 && lf > 0.0 && st.t[k] > self.cond.t_freeze {
                // energy available above freezing over the step, per unit time
                let avail = cp * ctx.dp[k] / g * (st.t[k] - self.cond.t_freeze) / ctx.dt;
                let m = snow.min(avail / lf);
                st.add_t(k, -lf * m * g / (cp * ctx.dp[k]));
                snow -= m;
                rain += m;
                melted += m;
            }
        }
        sfc.rain = rain;
        sfc.snow = snow;
        diag[0] += snow;
        diag[1] += melted;
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

    /// Condensation in a cold layer makes snow (latent heat L_v + L_f); falling through a
    /// warm layer it partly melts, cooling that layer no lower than T_freeze. Column
    /// energy: Σ (c_p ΔT + L_v Δq) Δp/g = L_f × snow at the surface × Δt_p; water: the
    /// column loses rain + snow.
    #[test]
    fn snow_forms_falls_and_melts_conserving_energy_and_water() {
        let geo = Geometry::new(&[0.0, 0.3, 0.6, 0.97, 1.0], 1e5);
        let planet = Planet::earth();
        let w = Condensable::water();
        let dt = 1800.0;
        let ctx = geo.ctx(&planet, Some(&w), dt);
        for t_warm in [273.4, 285.0] {
            let mut st = ColumnState::new(4, 1);
            st.begin(dt);
            st.t.copy_from_slice(&[220.0, 245.0, t_warm, 272.0]);
            st.q.copy_from_slice(&[1e-6, 3e-3, 1e-3, 1e-3]);
            let (t0, q0) = (st.t.clone(), st.q.clone());
            let mut sfc = SurfaceExchange::default();
            let mut d = [0.0];
            LargeScaleCondensation { cond: w.clone() }.apply(
                &ctx,
                &mut st,
                &mut sfc,
                &mut d,
                &mut Workspace::new(4),
            );
            let produced_snow: f64 = st.prod_snow.iter().sum();
            assert!(produced_snow > 0.0);
            let mut d2 = [0.0, 0.0];
            PrecipitationFall { cond: w.clone() }.apply(
                &ctx,
                &mut st,
                &mut sfc,
                &mut d2,
                &mut Workspace::new(4),
            );
            let (mut e, mut wat) = (0.0, 0.0);
            for k in 0..4 {
                let m = geo.dp[k] / planet.gravity;
                e += (planet.cp * (st.t[k] - t0[k]) + w.latent_heat * (st.q[k] - q0[k])) * m;
                wat += (st.q[k] - q0[k]) * m;
            }
            let lf = w.latent_heat_fusion;
            assert!((e - lf * sfc.snow * dt).abs() < 1e-6 * lf * produced_snow * dt + 1e-9);
            assert!((wat / dt + sfc.rain + sfc.snow).abs() < 1e-15);
            assert!(d2[1] > 0.0, "no melting at {t_warm}");
            assert!(st.t[2] >= w.t_freeze - 1e-12, "melt layer {}", st.t[2]);
            if t_warm < 274.0 {
                // too little heat above freezing to melt it all
                assert!(sfc.snow > 0.0 && st.t[2] - w.t_freeze < 1e-9);
            } else {
                assert_eq!(sfc.snow, 0.0);
            }
        }
    }
}
