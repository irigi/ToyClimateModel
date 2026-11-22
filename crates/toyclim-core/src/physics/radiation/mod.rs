//! Radiation (SPEC.md §9.4.1).
//!
//! - [`gray`]: the Frierson et al. (2006) gray scheme with its own solver, kept unchanged
//!   as `scheme = "gray"` for the idealized comparators (its shortwave is part of the
//!   benchmark definitions).
//! - [`solver`]: the generic multi-band two-stream solver; [`TwoStream`] is the column
//!   scheme around it, with the optical properties from an [`Opacity`] provider.
//! - Providers: [`semigray::SemiGray`] (`scheme = "semigray"`, the default for worlds) and
//!   [`GrayOpacity`] (the gray scheme's longwave optical depths in the generic solver; used
//!   to test the solver against the gray scheme).

pub mod cloud_optics;
pub mod gray;
pub mod offline;
pub mod planck;
pub mod semigray;
pub mod solver;

pub use gray::{Clouds, GrayRadiation};

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use cloud_optics::CloudOpticsParams;
use solver::{Boundary, CloudOptics, Fluxes, LayerOptics, SwScratch};

/// Stefan–Boltzmann constant \[W m⁻² K⁻⁴\] (CODATA 2018).
pub const STEFAN_BOLTZMANN: f64 = 5.670374e-8;

/// Gas optical properties of a column, per band.
pub trait Opacity: Send + Sync {
    fn sw_bands(&self) -> usize;
    fn lw_bands(&self) -> usize;
    /// Layer optical properties per band for one column (gas only; `out` has the shape
    /// set by the caller).
    fn layer_optics(&self, ctx: &ColumnCtx, col: &ColumnState, out: &mut LayerOptics);
    /// Fraction of the stellar flux in each shortwave band for a star at `t_eff` \[K\].
    fn sw_weights(&self, t_eff: f64, out: &mut [f64]);
    /// Surface albedo band (0 visible, 1 near-IR) seen by shortwave band `band`.
    fn sw_albedo_band(&self, band: usize) -> usize;
    /// Planck fraction of σT⁴ in each longwave band at temperature `t` \[K\].
    fn lw_planck_fraction(&self, t: f64, out: &mut [f64]);
}

/// The two-stream column scheme (SPEC.md §9.3.3 step 1).
///
/// Diagnostics as the gray scheme: `rsdt`, `olr`, `asr`, `rsds`, `rlds` \[W m⁻²\].
/// The surface gets the downward fluxes; each tile's broadband `albedo` is replaced by the
/// value that its band albedos give for this step's surface band fluxes, so the surface
/// solve absorbs exactly what the solver computed.
pub struct TwoStream {
    pub opacity: Box<dyn Opacity>,
    /// Shortwave band weights of the star.
    pub weights: Vec<f64>,
    /// Surface albedo \[visible, near-IR\] and temperature when the model has no tiles.
    pub albedo: [f64; 2],
    pub t_surface_default: f64,
    /// Smallest cos zenith used for the slant path (the beam's path is finite on a sphere).
    pub mu_min: f64,
    /// Shortwave reflected at the top before any absorption (a cloud stand-in until P4,
    /// as the gray scheme's `reflectance`): a fixed fraction, or the fraction that makes
    /// the column's planetary albedo equal `planetary_albedo` (SPEC.md P3 acceptance 3).
    /// The shortwave is linear in the incoming flux, so the rest of the beam is the
    /// clear solve scaled by (1 − r).
    pub reflectance: f64,
    pub planetary_albedo: Option<f64>,
    /// Cloud optics; with them the scheme reads the clouds the cloud diagnosis left in
    /// [`SurfaceExchange::cloud`] in the previous physics step (SPEC.md §9.3.3).
    pub clouds: Option<CloudOpticsParams>,
    /// Diagnostic CRE split (P4 question 13): with clouds whose memory carries the
    /// stratiform-only fraction (fourth block, `[clouds] cre_split`), one more solve with the
    /// convective cloud removed (same in-cloud water), reported as `olr_nc` and `asr_nc`.
    /// The fluxes that heat the model are unchanged.
    pub cre_split: bool,
}

impl TwoStream {
    pub fn new(opacity: Box<dyn Opacity>, t_eff: f64) -> Self {
        let mut weights = vec![0.0; opacity.sw_bands()];
        opacity.sw_weights(t_eff, &mut weights);
        TwoStream {
            opacity,
            weights,
            albedo: [0.06, 0.06],
            t_surface_default: 288.0,
            mu_min: 0.05,
            reflectance: 0.0,
            planetary_albedo: None,
            clouds: None,
            cre_split: false,
        }
    }

    /// Optics, boundary conditions and the solve for one column.
    pub fn fluxes(
        &self,
        ctx: &ColumnCtx,
        st: &ColumnState,
        sfc: &SurfaceExchange,
        clouds: Option<&solver::CloudOptics>,
        out: &mut Fluxes,
    ) {
        self.fluxes_with(ctx, st, sfc, clouds, out, &mut RadWork::default());
    }

    /// As [`Self::fluxes`] with reusable scratch space.
    pub fn fluxes_with(
        &self,
        ctx: &ColumnCtx,
        st: &ColumnState,
        sfc: &SurfaceExchange,
        clouds: Option<&solver::CloudOptics>,
        out: &mut Fluxes,
        w: &mut RadWork,
    ) {
        let l = st.nlev;
        let (nsw, nlw) = (self.opacity.sw_bands(), self.opacity.lw_bands());
        w.optics.resize(l, nsw, nlw);
        self.opacity.layer_optics(ctx, st, &mut w.optics);
        let RadWork {
            optics,
            frac,
            source,
            lw_sfc,
            albedo,
            sw,
            ..
        } = w;
        frac.clear();
        frac.resize(nlw, 0.0);
        source.clear();
        source.resize(nlw * l, 0.0);
        for k in 0..l {
            self.opacity.lw_planck_fraction(st.t[k], frac);
            let b = STEFAN_BOLTZMANN * st.t[k].powi(4);
            for (band, f) in frac.iter().enumerate() {
                source[band * l + k] = f * b;
            }
        }
        lw_sfc.clear();
        lw_sfc.resize(nlw, 0.0);
        let mut alb2 = [0.0; 2];
        if sfc.tiles.is_empty() {
            self.opacity
                .lw_planck_fraction(self.t_surface_default, frac);
            let b = STEFAN_BOLTZMANN * self.t_surface_default.powi(4);
            for (x, f) in lw_sfc.iter_mut().zip(frac.iter()) {
                *x = f * b;
            }
            alb2 = self.albedo;
        } else {
            for t in &sfc.tiles {
                self.opacity.lw_planck_fraction(t.ts, frac);
                let b = t.frac * STEFAN_BOLTZMANN * t.ts.powi(4);
                for (x, f) in lw_sfc.iter_mut().zip(frac.iter()) {
                    *x += f * b;
                }
                alb2[0] += t.frac * t.albedo_bands[0];
                alb2[1] += t.frac * t.albedo_bands[1];
            }
        }
        albedo.clear();
        albedo.extend((0..nsw).map(|b| alb2[self.opacity.sw_albedo_band(b)]));
        let bnd = Boundary {
            s0: ctx.insolation,
            mu0: ctx.cos_zenith.max(self.mu_min),
            sw_weights: &self.weights,
            albedo,
            lw_source: source,
            lw_sfc,
        };
        solver::solve(optics, clouds, &bnd, out, sw);
        let s0 = ctx.insolation;
        if s0 > 0.0 && (self.reflectance > 0.0 || self.planetary_albedo.is_some()) {
            let r = match self.planetary_albedo {
                Some(a) => {
                    let ac = out.sw_up_top / s0;
                    ((a - ac) / (1.0 - ac)).clamp(0.0, 1.0)
                }
                None => self.reflectance,
            };
            for x in out.sw_net.iter_mut().chain(out.sw_down_sfc.iter_mut()) {
                *x *= 1.0 - r;
            }
            out.sw_up_top = r * s0 + (1.0 - r) * out.sw_up_top;
            out.clear_sw_up_top = r * s0 + (1.0 - r) * out.clear_sw_up_top;
        }
    }

    /// In-cloud optics of the column's clouds (`sfc.cloud`, see [`SurfaceExchange::cloud`])
    /// into `c`; false when the column has none.
    pub fn cloud_optics(&self, sfc: &SurfaceExchange, nlev: usize, c: &mut CloudOptics) -> bool {
        let Some(par) = &self.clouds else {
            return false;
        };
        let m = &sfc.cloud;
        if m.len() < 3 * nlev || m[..nlev].iter().all(|&f| f <= 0.0) {
            return false;
        }
        let (nsw, nlw) = (self.opacity.sw_bands(), self.opacity.lw_bands());
        c.optics.resize(nlev, nsw, nlw);
        c.frac.clear();
        c.frac.extend_from_slice(&m[..nlev]);
        let o = &mut c.optics;
        for k in 0..nlev {
            let (lwp, iwp) = (m[nlev + k], m[2 * nlev + k]);
            for b in 0..nsw {
                let (t, w, g) = par.shortwave(self.opacity.sw_albedo_band(b), lwp, iwp);
                let i = b * nlev + k;
                o.sw_tau[i] = t;
                o.sw_ssa[i] = w;
                o.sw_g[i] = g;
            }
            let tl = par.longwave(lwp, iwp);
            for b in 0..nlw {
                o.lw_tau[b * nlev + k] = tl;
            }
        }
        true
    }
}

/// Reusable scratch space of [`TwoStream`].
#[derive(Debug, Clone, Default)]
pub struct RadWork {
    optics: LayerOptics,
    frac: Vec<f64>,
    source: Vec<f64>,
    lw_sfc: Vec<f64>,
    albedo: Vec<f64>,
    sw: SwScratch,
    clouds: CloudOptics,
}

thread_local! {
    static WORK: std::cell::RefCell<(RadWork, Fluxes)> = std::cell::RefCell::default();
    static WORK_NC: std::cell::RefCell<Fluxes> = std::cell::RefCell::default();
}

impl ColumnScheme for TwoStream {
    fn name(&self) -> &'static str {
        "two_stream_radiation"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        if self.clouds.is_some() && self.cre_split {
            &[
                "rsdt", "olr", "asr", "rsds", "rlds", "clt", "olr_cs", "asr_cs", "olr_nc", "asr_nc",
            ]
        } else if self.clouds.is_some() {
            &[
                "rsdt", "olr", "asr", "rsds", "rlds", "clt", "olr_cs", "asr_cs",
            ]
        } else {
            &["rsdt", "olr", "asr", "rsds", "rlds"]
        }
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
        let split = self.cre_split && self.clouds.is_some() && sfc.cloud.len() >= 4 * l;
        let mut nc = (0.0, 0.0);
        let f = WORK.with(|w| {
            let (work, f) = &mut *w.borrow_mut();
            let mut cl = std::mem::take(&mut work.clouds);
            let cloudy = self.cloud_optics(sfc, l, &mut cl);
            self.fluxes_with(ctx, st, sfc, cloudy.then_some(&cl), f, work);
            if split {
                // the same clouds without the convective part (stratiform-only fraction)
                WORK_NC.with(|g| {
                    let g = &mut *g.borrow_mut();
                    let strat = &sfc.cloud[3 * l..4 * l];
                    let any = cloudy && strat.iter().any(|&c| c > 0.0);
                    if any {
                        cl.frac.clear();
                        cl.frac.extend_from_slice(strat);
                    }
                    self.fluxes_with(ctx, st, sfc, any.then_some(&cl), g, work);
                    nc = (g.lw_up[0], g.sw_net[0]);
                });
            }
            work.clouds = cl;
            std::mem::take(f)
        });
        let g = ctx.planet.gravity;
        for k in 0..l {
            let sw = f.sw_net[k] - f.sw_net[k + 1];
            let lw = (f.lw_down[k] - f.lw_up[k]) - (f.lw_down[k + 1] - f.lw_up[k + 1]);
            st.add_t(k, g / (ctx.planet.cp * ctx.dp[k]) * (sw + lw));
        }
        let sw_down: f64 = f.sw_down_sfc.iter().sum();
        if sw_down > 0.0 {
            // effective broadband albedo of each tile for this step's band fluxes
            for t in &mut sfc.tiles {
                let mut r = 0.0;
                for (b, d) in f.sw_down_sfc.iter().enumerate() {
                    r += d * t.albedo_bands[self.opacity.sw_albedo_band(b)];
                }
                t.albedo = r / sw_down;
            }
        }
        sfc.sw_down = sw_down;
        sfc.lw_down = f.lw_down[l];
        sfc.toa_net = f.sw_net[0] - f.lw_up[0];
        diag[0] += ctx.insolation;
        diag[1] += f.lw_up[0];
        diag[2] += f.sw_net[0];
        diag[3] += sw_down;
        diag[4] += f.lw_down[l];
        if self.clouds.is_some() {
            diag[5] += f.cloud_cover;
            diag[6] += f.clear_olr;
            diag[7] += ctx.insolation - f.clear_sw_up_top;
            if self.cre_split {
                diag[8] += if split { nc.0 } else { f.lw_up[0] };
                diag[9] += if split { nc.1 } else { f.sw_net[0] };
            }
        }
        // hand the flux arrays back for the next column
        WORK.with(|w| w.borrow_mut().1 = f);
    }
}

/// The gray scheme's optical depths as an [`Opacity`]: one longwave band with the
/// Frierson τ(σ) (divided by the diffusivity, since the gray τ is a flux optical depth)
/// and one absorbing shortwave band τ_sw σ² (vertical; the gray scheme's own shortwave
/// differs: single pass, top reflectance).
pub struct GrayOpacity {
    pub gray: GrayRadiation,
}

impl Opacity for GrayOpacity {
    fn sw_bands(&self) -> usize {
        1
    }
    fn lw_bands(&self) -> usize {
        1
    }
    fn layer_optics(&self, ctx: &ColumnCtx, st: &ColumnState, out: &mut LayerOptics) {
        let g = &self.gray;
        let tau0 = if g.latitude_dependence {
            g.tau_eq + (g.tau_pole - g.tau_eq) * ctx.lat.sin().powi(2)
        } else {
            g.tau_eq
        };
        let tau = |s: f64| tau0 * (g.f_linear * s + (1.0 - g.f_linear) * s.powi(4));
        let h = ctx.sigma_half;
        for k in 0..st.nlev {
            let mut d = tau(h[k + 1]) - tau(h[k]);
            if g.tau_q > 0.0 {
                if let Some(q) = st.humidity() {
                    let v = q[k] * ctx.dp[k] / ctx.planet.gravity;
                    d += g.tau_q * v;
                }
            }
            out.lw_tau[k] = d / solver::DIFFUSIVITY;
            out.sw_tau[k] = g.tau_sw * (h[k + 1].powi(2) - h[k].powi(2));
        }
    }
    fn sw_weights(&self, _t_eff: f64, out: &mut [f64]) {
        out[0] = 1.0;
    }
    fn sw_albedo_band(&self, _band: usize) -> usize {
        0
    }
    fn lw_planck_fraction(&self, _t: f64, out: &mut [f64]) {
        out[0] = 1.0;
    }
}

#[cfg(test)]
mod tests;
