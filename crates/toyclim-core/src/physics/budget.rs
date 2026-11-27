//! Moisture-budget probe of the lowest levels (diagnostic only; P4 question 13, owner
//! decision after the diagnosis, 2026-11-26).
//!
//! A [`MoistureProbe`] changes nothing in the column. Inserted after a scheme, it records
//! the tendency of the condensable accumulated since the start of the physics step, as a
//! mass flux into the layer of levels with σ_k ≥ `sigma_min` \[kg m⁻² s⁻¹\]:
//! W' = Σ_{σ_k ≥ σ_min} dq_k Δp_k / g, and the same for the lowest level alone. Probes after
//! the boundary layer, after convection and after the large-scale condensation give, by
//! difference, the share of each scheme (the boundary-layer share holds the surface
//! evaporation and the vertical mixing together; the evaporation is the `evap`
//! diagnostic, so mixing = boundary layer − evaporation). The first probe also records
//! the layer's water W = Σ q Δp/g \[kg m⁻²\] and the relative humidity of the lowest level
//! at the start of the step; the probe after convection the pressure of the convection top
//! when deep convection was active (0 otherwise; time mean / `conv_active` = mean top).
//! Enabled with `[convection] budget_sigma = <σ_min>`; without it no probe is built and the
//! model is unchanged.

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::condensation::Condensable;

/// Where a probe sits in the scheme order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStage {
    BoundaryLayer,
    Convection,
    Condensation,
}

/// Diagnostic probe; see the module documentation.
pub struct MoistureProbe {
    pub cond: Condensable,
    pub sigma_min: f64,
    pub stage: ProbeStage,
}

impl ColumnScheme for MoistureProbe {
    fn name(&self) -> &'static str {
        "moisture_probe"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        match self.stage {
            ProbeStage::BoundaryLayer => &["qlow_bl", "qlev1_bl", "wlow", "rh_low"],
            ProbeStage::Convection => &["qlow_conv", "qlev1_conv", "conv_top_p"],
            ProbeStage::Condensation => &["qlow_ls", "qlev1_ls"],
        }
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let l = st.nlev;
        if st.humidity().is_none() || l == 0 {
            return;
        }
        let g = ctx.planet.gravity;
        let dtp = st.dtp();
        let mut flux = 0.0;
        let mut water = 0.0;
        for k in 0..l {
            if ctx.sigma[k] >= self.sigma_min {
                flux += st.dq[k] * ctx.dp[k] / g;
                water += (st.q[k] - st.dq[k] * dtp) * ctx.dp[k] / g;
            }
        }
        diag[0] += flux;
        diag[1] += st.dq[l - 1] * ctx.dp[l - 1] / g;
        match self.stage {
            ProbeStage::BoundaryLayer => {
                diag[2] += water;
                let q0 = st.q[l - 1] - st.dq[l - 1] * dtp;
                let t0 = st.t[l - 1] - st.dt[l - 1] * dtp;
                diag[3] += q0 / self.cond.qsat(ctx.planet, t0, ctx.p[l - 1]);
            }
            ProbeStage::Convection => {
                if st.conv_top < l {
                    diag[2] += ctx.p[st.conv_top];
                }
            }
            ProbeStage::Condensation => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;
    use crate::planet::Planet;

    /// The probe leaves the column unchanged and reports the layer integral of the
    /// accumulated tendency and the start-of-step water.
    #[test]
    fn probe_reports_layer_tendency_and_changes_nothing() {
        let geo = Geometry::l26();
        let planet = Planet::earth();
        let cond = Condensable::water();
        let ctx = geo.ctx(&planet, Some(&cond), 1800.0);
        let l = geo.p.len();
        let mut st = ColumnState::new(l, 1);
        st.begin(1800.0);
        for k in 0..l {
            st.t[k] = 280.0;
            st.q[k] = 0.005;
        }
        // a tendency of 1e-7 kg/kg/s in the lowest level and one at mid levels
        st.add_q(0, l - 1, 1e-7);
        st.add_q(0, 10, 1e-7);
        let (t0, q0) = (st.t.clone(), st.q.clone());
        let probe = MoistureProbe {
            cond: cond.clone(),
            sigma_min: 0.87,
            stage: ProbeStage::BoundaryLayer,
        };
        let mut diag = [0.0; 4];
        probe.apply(
            &ctx,
            &mut st,
            &mut SurfaceExchange::default(),
            &mut diag,
            &mut Workspace::new(l),
        );
        assert_eq!(st.t, t0);
        assert_eq!(st.q, q0);
        let g = planet.gravity;
        let want = 1e-7 * geo.dp[l - 1] / g;
        assert!((diag[0] - want).abs() < 1e-15, "{} vs {want}", diag[0]);
        assert!((diag[1] - want).abs() < 1e-15);
        let w: f64 = (0..l)
            .filter(|&k| ctx.sigma[k] >= 0.87)
            .map(|k| 0.005 * geo.dp[k] / g)
            .sum();
        assert!((diag[2] - w).abs() < 1e-12 * w, "{} vs {w}", diag[2]);
        let rh = 0.005 / cond.qsat(&planet, 280.0, geo.p[l - 1]);
        assert!((diag[3] - rh).abs() < 1e-12);
    }
}
