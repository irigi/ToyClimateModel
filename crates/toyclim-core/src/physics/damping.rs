//! Momentum damping: the sponge at the model top and boundary-layer Rayleigh friction.

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};

/// Sponge (SPEC.md §9.4.3, `[dynamics.sponge]`, off by default): linear damping of u, v
/// towards zero where σ < `sigma_top`, rate k(σ) = k_top ((σ_top − σ)/(σ_top − σ_1))², so the
/// time scale is `tau` at the top full level σ_1. Implicit in time; the kinetic energy
/// removed is returned as heat in the same layer (frictional heating).
/// Diagnostic `sponge_heat`: column heating from the removed kinetic energy \[W m⁻²\].
#[derive(Debug, Clone)]
pub struct Sponge {
    /// Bottom of the sponge in σ (0.02).
    pub sigma_top: f64,
    /// Damping time at the top full level \[s\] (1 day).
    pub tau: f64,
}

impl Default for Sponge {
    fn default() -> Self {
        Sponge {
            sigma_top: 0.02,
            tau: 86400.0,
        }
    }
}

impl ColumnScheme for Sponge {
    fn name(&self) -> &'static str {
        "sponge"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["sponge_heat"]
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let s1 = ctx.sigma[0];
        if s1 >= self.sigma_top {
            return;
        }
        let (cp, g) = (ctx.planet.cp, ctx.planet.gravity);
        let mut heat = 0.0;
        for k in 0..st.nlev {
            let s = ctx.sigma[k];
            if s >= self.sigma_top {
                break;
            }
            let rate = ((self.sigma_top - s) / (self.sigma_top - s1)).powi(2) / self.tau;
            let fac = 1.0 / (1.0 + rate * ctx.dt);
            let (u, v) = (st.u[k] * fac, st.v[k] * fac);
            let dke = 0.5 * (u * u + v * v - st.u[k] * st.u[k] - st.v[k] * st.v[k]);
            st.set_u(k, u);
            st.set_v(k, v);
            let t = st.t[k] - dke / cp;
            st.set_t(k, t);
            heat -= dke * ctx.dp[k] / g / ctx.dt;
        }
        diag[0] += heat;
    }
}
