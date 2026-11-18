//! Surface fluxes and boundary-layer mixing of the moist Held–Suarez test of Thatcher &
//! Jablonowski (2016, Geosci. Model Dev. 9, 1263–1292; "TJ16"), which builds on Reed &
//! Jablonowski (2012). Frozen regression physics (SPEC.md §9.1): only the `moist_held_suarez`
//! bundle uses it; the constant exchange coefficient 0.0044 stays here.
//!
//! TJ16 Sect. 2 specifies a sequential (time-split) package in the order large-scale
//! condensation, surface fluxes, boundary-layer mixing, Rayleigh friction, HS temperature
//! relaxation (SBM convection, Sect. 6.1, before the condensation); the bundle in
//! `physics::build` follows it.

use crate::physics::boundary_layer::implicit_diffusion;
use crate::physics::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use crate::physics::condensation::Condensable;

/// Prescribed sea-surface temperature.
#[derive(Debug, Clone)]
pub enum SurfaceTemperature {
    /// TJ16 eq. (6): T_s = ΔT exp(−φ²/(2 Δφ²)) + T_min.
    Gaussian {
        delta_t: f64,
        t_min: f64,
        width: f64,
    },
    Uniform(f64),
}

impl SurfaceTemperature {
    pub fn tj16() -> Self {
        SurfaceTemperature::Gaussian {
            delta_t: 29.0,
            t_min: 271.0,
            width: 26f64.to_radians(),
        }
    }
    pub fn at(&self, lat: f64, _lon: f64) -> f64 {
        match self {
            SurfaceTemperature::Gaussian {
                delta_t,
                t_min,
                width,
            } => delta_t * (-(lat * lat) / (2.0 * width * width)).exp() + t_min,
            SurfaceTemperature::Uniform(t) => *t,
        }
    }
}

/// Height of the lowest full level above the surface, TJ16 eq. (12) \[m\].
fn lowest_level_height(ctx: &ColumnCtx, tv: f64) -> f64 {
    let l = ctx.sigma.len();
    let ln_ratio = -ctx.sigma_half[l - 1].ln(); // ln p_s − ln p_{L−1/2}
    ctx.planet.gas_constant * tv / ctx.planet.gravity * ln_ratio / 2.0
}

/// Bulk sensible and latent heat fluxes at the lowest level over the prescribed SST
/// (TJ16 eqs. 7–8 with the time-implicit update). Diagnostics: `evap` \[kg m⁻² s⁻¹\],
/// `shf`, `lhf` \[W m⁻²\].
///
/// TJ16 eq. (8) adds moisture C_E|v_a|(q_s − q_a)/z_a to the lowest layer with z_a the height
/// of the lowest *full* level (about half the layer depth), while eq. (10) diagnoses
/// E = ρ_a C_E |v_a| (q_s − q_a), smaller by ρ_a z_a g/Δp ≈ 1/2. Eq. (8) is kept and the flux
/// actually applied is reported, so that precipitation balances evaporation.
pub struct Tj16SurfaceFluxes {
    pub cond: Condensable,
    pub sst: SurfaceTemperature,
    /// Bulk transfer coefficients C_H, C_E.
    pub c_h: f64,
    pub c_e: f64,
}

impl ColumnScheme for Tj16SurfaceFluxes {
    fn name(&self) -> &'static str {
        "tj16_surface_fluxes"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        &["evap", "shf", "lhf"]
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
        let k = st.nlev - 1;
        let (ta, qa) = (st.t[k], st.q[k]);
        let wind = (st.u[k] * st.u[k] + st.v[k] * st.v[k]).sqrt();
        let za = lowest_level_height(ctx, ta * (1.0 + 0.608 * qa));
        let ts = self.sst.at(ctx.lat, ctx.lon);
        let qs = self.cond.qsat(planet, ts, ctx.ps);
        let dt = ctx.dt;
        let (ah, ae) = (self.c_h * wind * dt / za, self.c_e * wind * dt / za);
        let t_new = (ta + ah * ts) / (1.0 + ah);
        let q_new = (qa + ae * qs) / (1.0 + ae);
        st.set_t(k, t_new);
        st.set_q(0, k, q_new);
        let layer_mass = ctx.dp[k] / planet.gravity;
        let evap = layer_mass * (q_new - qa) / dt;
        let shf = layer_mass * planet.cp * (t_new - ta) / dt;
        sfc.evap += evap;
        sfc.shf += shf;
        sfc.lhf += evap * self.cond.latent_heat;
        diag[0] += evap;
        diag[1] += shf;
        diag[2] += evap * self.cond.latent_heat;
    }
}

/// Boundary-layer vertical diffusion of potential temperature and q (TJ16 eqs. 15–17):
/// K = C_E |v_a| z_a below p_pbl, decaying as exp(−((p_pbl − p)/p_strato)²) above.
/// Backward-Euler (implicit) tridiagonal solve; zero flux at the top and the surface
/// (surface fluxes are the scheme before).
pub struct Tj16BoundaryLayer {
    pub c_e: f64,
    /// Top of the boundary layer \[Pa\].
    pub p_pbl: f64,
    /// Decay scale above the boundary layer \[Pa\].
    pub p_strato: f64,
    /// Reference pressure for θ \[Pa\].
    pub p0: f64,
}

impl ColumnScheme for Tj16BoundaryLayer {
    fn name(&self) -> &'static str {
        "tj16_boundary_layer"
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        _sfc: &mut SurfaceExchange,
        _diag: &mut [f64],
        work: &mut Workspace,
    ) {
        let l = st.nlev;
        if l < 2 {
            return;
        }
        let planet = ctx.planet;
        let (g, r, kappa) = (planet.gravity, planet.gas_constant, planet.kappa());
        let kb = l - 1;
        let wind = (st.u[kb] * st.u[kb] + st.v[kb] * st.v[kb]).sqrt();
        let qa = st.humidity().map_or(0.0, |q| q[kb]);
        let za = lowest_level_height(ctx, st.t[kb] * (1.0 + 0.608 * qa));
        let k0 = self.c_e * wind * za;
        if k0 <= 0.0 {
            return;
        }
        let [a_half, c, pi, x, den, f, ..] = &mut work.buf;
        // A_{k+1/2} = ρ² g K / (p_{k+1} − p_k) at the interface between k and k+1
        for k in 0..l - 1 {
            let ph = ctx.p_half[k + 1];
            let kk = if ph > self.p_pbl {
                k0
            } else {
                k0 * (-((self.p_pbl - ph) / self.p_strato).powi(2)).exp()
            };
            let rho = ph / (r * 0.5 * (st.t[k] + st.t[k + 1]));
            a_half[k] = rho * rho * g * kk / (ctx.p[k + 1] - ctx.p[k]);
        }
        for k in 0..l {
            c[k] = g * ctx.dt / ctx.dp[k];
            pi[k] = (ctx.p[k] / self.p0).powf(kappa);
            x[k] = st.t[k] / pi[k];
        }
        let (a_half, c) = (&a_half[..l - 1], &c[..l]);
        implicit_diffusion(&mut x[..l], a_half, c, 0.0, den, f);
        for k in 0..l {
            st.set_t(k, x[k] * pi[k]);
        }
        if st.humidity().is_some() {
            x[..l].copy_from_slice(&st.q[..l]);
            implicit_diffusion(&mut x[..l], a_half, c, 0.0, den, f);
            for k in 0..l {
                st.set_q(0, k, x[k]);
            }
        }
    }
}
