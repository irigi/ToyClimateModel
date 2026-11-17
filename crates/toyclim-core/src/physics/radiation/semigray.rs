//! Semi-gray opacities (SPEC.md §9.4.1), the default radiation for worlds.
//!
//! Every optical depth is per layer of mass dm = Δp/g, written for Earth as a function of
//! Δp/p₀ (p₀ = 1e5 Pa) and scaled by (9.81/g), so a heavier or lighter atmosphere scales
//! with its column mass.
//!
//! **Shortwave**, three bands: `vis` (λ < 0.75 µm), `nir_weak` and `nir_strong` (λ > 0.75 µm,
//! split into two pseudo-bands so that water-vapour absorption saturates; the share of the
//! weak band is a **knob**). Band weights from the Planck function at the star's T_eff
//! ([`super::planck`]).
//! - Rayleigh scattering (ω = 1, g = 0): τ_R = τ_R0,band Δp/p₀ (9.81/g) **[knobs]**.
//! - Water vapour (near-IR bands, absorbing): Δτ = k_band q (p/p_ref)^n Δp/g **[knobs]**.
//! - Well-mixed absorbers of the near-IR (O₂, CO₂, CH₄ bands; not in SPEC.md, added
//!   because the RRTMG reference absorbs 15–25 W m⁻² more than water vapour alone in cold,
//!   dry columns): Δτ = τ_dry,band (p/p_ref)^n Δp/p₀ (9.81/g) **[knobs]**, pressure-broadened
//!   like the vapour lines.
//! - Optional ozone-like visible absorber: total column τ_O₃ (0 = off, the default),
//!   distributed in ln p as a Gaussian of width `ozone_width` around `ozone_p`.
//!
//! **Longwave**, two bands after Geen, Czaja & Haigh (2016, GRL 43, 8314): the 8–14 µm
//! `window` and the `rest`; the rest may be split into two pseudo-bands, weak and strong
//! (share `lw_strong_share` of its Planck flux in the strong one, coefficients `lw_rest`
//! and `lw_rest_strong`), as the near-IR in the shortwave: one gray band cannot hold both
//! the optically thick line centres and the thin wings (the two-band fit to RRTMG leaves
//! 12–13 W m⁻² errors at 1 bar). Share 0 is the two-band form. Flux optical depths per
//! Δp/p₀ (Earth):
//! - rest: dτ/d(p/p₀) = (p/p_ref)^n [a + b ln(1 + c q)] + (p/p_ref)^n_c d ln(CO₂/360 ppm);
//! - window: dτ/d(p/p₀) = (p/p_ref)^n [a_w + b_w q] + (p/p_ref)^n_c d_w ln(CO₂/360 ppm)
//!   + c_w q² (p/p_ref);
//! - ln(CO₂/360) stands for L + κL² (`lw_co2_curvature`, 0 in Geen's form);
//! - each clamped at ≥ 0, divided by the diffusivity 1.66 (the solver's τ is vertical).
//!
//! The pressure factors extend Geen's Earth form (n = 0, no p in the q² term) to other
//! surface pressures: line absorption per unit mass is pressure-broadened, (p/p_ref)^n
//! (SPEC.md §9.4.1 puts the knob n on the dry term; the calibration across 0.3–1 bar
//! needs it on the vapour lines too, and a separate exponent n_c on the CO₂ term), and the water-vapour self-continuum in the
//! window scales with the vapour pressure e ∝ q p, hence q² p per unit mass.
//!
//! The defaults are the fit of `tools/calibrate_radiation.py` to RRTMG (2026-11-17,
//! docs/radiation/calibration.md). Geen's two-band starting values, from the Isca
//! documentation of this scheme
//! ("two_stream_gray_rad", Geen scheme, written with R. Geen; the paper's Supporting
//! Information was not accessible): a = 0.1, b = 23.8, c = 254, d = 0.2023; a_w = 0.2150,
//! b_w = 147.11, c_w = 1.0814e4, d_w = 0.0954. They were fitted to SBDART there. The
//! pressure exponent n (pressure broadening of the dry term, for the envelope) is 0 in that
//! form. Geen use a fixed window fraction 0.3732; here the window carries the exact
//! Planck fraction of 8–14 µm at the layer temperature (0.366 at 288 K).
//!
//! All knobs are refitted to RRTMG clear-sky fluxes by `tools/calibrate_radiation.py`;
//! the fitted values and residuals are in `docs/radiation/` and these defaults.

use super::planck;
use super::solver::{LayerOptics, DIFFUSIVITY};
use super::Opacity;
use crate::physics::column::{ColumnCtx, ColumnState};

/// Reference column: p₀ \[Pa\] and g \[m s⁻²\] of the Earth-fitted constants.
const P0: f64 = 1e5;
const G0: f64 = 9.81;

/// Ozone column optical depth and centre pressure \[Pa\] of the Earth configuration
/// (visible band; Earth cases set `ozone_tau` and `ozone_p` to them), fitted to RRTMG with
/// climlab's ozone profile (docs/radiation/calibration.md).
pub const EARTH_OZONE_TAU: f64 = 0.02425;
pub const EARTH_OZONE_P: f64 = 6429.6;

/// Window Planck fraction tabulated in temperature (the series of [`planck`] per layer
/// and step is a large share of the radiation cost): 50–1000 K every 0.25 K, linear
/// interpolation (error < 1e-7), direct evaluation outside.
#[derive(Debug, Clone, Default)]
struct WindowTable {
    edges: [f64; 2],
    values: Vec<f64>,
}

const TABLE_T0: f64 = 50.0;
const TABLE_DT: f64 = 0.25;
const TABLE_N: usize = 3801;

impl WindowTable {
    fn new(edges: [f64; 2]) -> Self {
        WindowTable {
            edges,
            values: (0..TABLE_N)
                .map(|i| {
                    planck::fraction_between(edges[0], edges[1], TABLE_T0 + i as f64 * TABLE_DT)
                })
                .collect(),
        }
    }
    fn get(&self, t: f64) -> f64 {
        let x = (t - TABLE_T0) / TABLE_DT;
        if x >= 0.0 && x < (TABLE_N - 1) as f64 {
            let i = x as usize;
            let w = x - i as f64;
            (1.0 - w) * self.values[i] + w * self.values[i + 1]
        } else {
            planck::fraction_between(self.edges[0], self.edges[1], t)
        }
    }
}

/// Cache of the window Planck fractions (filled on first use; not a knob).
#[derive(Debug, Clone, Default)]
pub struct PlanckCache(std::sync::OnceLock<WindowTable>);

/// Knobs of the semi-gray scheme (`[radiation]` keys of the same names).
#[derive(Debug, Clone)]
pub struct SemiGray {
    /// Edge between `vis` and the near-IR bands \[m\].
    pub vis_edge: f64,
    /// Share of the near-IR flux in `nir_weak`.
    pub nir_weak_share: f64,
    /// Rayleigh column optical depth per band at p₀, Earth gravity.
    pub rayleigh: [f64; 3],
    /// Column absorption optical depth of the well-mixed gases in the two near-IR bands at
    /// p₀, Earth gravity.
    pub dry_abs: [f64; 2],
    /// Water-vapour mass absorption coefficients of the two near-IR bands \[m² kg⁻¹\].
    pub k_wv: [f64; 2],
    /// Pressure scaling of the shortwave vapour absorption: reference pressure \[Pa\] and
    /// exponent.
    pub wv_p_ref: f64,
    pub wv_n: f64,
    /// Ozone-like visible absorber: column optical depth, centre pressure \[Pa\], width in
    /// ln p.
    pub ozone_tau: f64,
    pub ozone_p: f64,
    pub ozone_width: f64,
    /// Longwave `rest` band (weak part): a, b, c, d; the strong part and its share of the
    /// rest band's Planck flux.
    pub lw_rest: [f64; 4],
    pub lw_rest_strong: [f64; 4],
    pub lw_strong_share: f64,
    /// Longwave `window` band: a_w, b_w, c_w, d_w.
    pub lw_window: [f64; 4],
    /// Window edges \[m\].
    pub window: [f64; 2],
    /// Pressure exponent n of the line terms and its reference pressure \[Pa\].
    pub lw_n: f64,
    pub lw_p_ref: f64,
    /// Pressure exponent of the CO₂ terms and the curvature κ of their amount dependence.
    pub lw_n_co2: f64,
    pub lw_co2_curvature: f64,
    /// CO₂ volume mixing ratio \[ppm\] (from the atmosphere composition).
    pub co2_ppm: f64,
    /// Window Planck fractions for `window` (built on first use).
    pub table: PlanckCache,
}

impl Default for SemiGray {
    fn default() -> Self {
        SemiGray {
            vis_edge: 0.75e-6,
            nir_weak_share: 0.717843,
            rayleigh: [0.18658, 0.0, 0.02],
            dry_abs: [0.0, 0.405527],
            k_wv: [0.00246404, 0.237921],
            wv_p_ref: P0,
            wv_n: 0.372311,
            ozone_tau: 0.0,
            ozone_p: 1000.0,
            ozone_width: 1.0,
            lw_rest: [0.532771, 24.6202, 518.083, 0.0],
            lw_rest_strong: [7.83502, 30.9345, 19261.6, 3.83602],
            lw_strong_share: 0.426911,
            lw_window: [0.0, 346.427, 2046.88, 0.0658702],
            window: [8e-6, 14e-6],
            lw_n: 0.453076,
            lw_p_ref: P0,
            lw_n_co2: 0.0,
            lw_co2_curvature: 0.223756,
            co2_ppm: 400.0,
            table: PlanckCache::default(),
        }
    }
}

impl SemiGray {
    /// The CO₂ amount term: L + κL² with L = ln(CO₂/360 ppm). The curvature κ lets the
    /// forcing per doubling grow with the amount, as in the RRTMG reference (about 3.4,
    /// 4.1 and 5.1 W m⁻² per doubling at 100–280, 400–800 and 2000–10000 ppm for the
    /// 290 K column).
    fn co2_log(&self) -> f64 {
        let l = (self.co2_ppm.max(1e-9) / 360.0).ln();
        l + self.lw_co2_curvature * l * l
    }
}

impl Opacity for SemiGray {
    fn sw_bands(&self) -> usize {
        3
    }
    fn lw_bands(&self) -> usize {
        3
    }
    fn layer_optics(&self, ctx: &ColumnCtx, st: &ColumnState, out: &mut LayerOptics) {
        let l = st.nlev;
        let g = ctx.planet.gravity;
        let q = st.humidity();
        let lc = self.co2_log();
        let [a, b, c, d] = self.lw_rest;
        let [aw, bw, cw, dw] = self.lw_window;
        let [a2, b2, c2, d2] = self.lw_rest_strong;
        // ozone weights (Gaussian in ln p), normalised over the column
        let oz = |k: usize| {
            let x = (ctx.p[k] / self.ozone_p).ln() / self.ozone_width;
            (-0.5 * x * x).exp() * ctx.dp[k] / ctx.p[k]
        };
        let oz_sum: f64 = if self.ozone_tau > 0.0 {
            (0..l).map(oz).sum()
        } else {
            0.0
        };
        for k in 0..l {
            let qk = q.map_or(0.0, |q| q[k].max(0.0));
            let x = ctx.dp[k] / P0 * (G0 / g); // Δp/p₀ of an Earth column of equal mass
            let m = ctx.dp[k] / g; // layer mass [kg m⁻²]
                                   // shortwave
            let mut abs_vis = 0.0;
            if oz_sum > 0.0 {
                abs_vis = self.ozone_tau * oz(k) / oz_sum;
            }
            let pw = (ctx.p[k] / self.wv_p_ref).powf(self.wv_n);
            let absorb = [
                abs_vis,
                (self.k_wv[0] * qk * m + self.dry_abs[0] * x) * pw,
                (self.k_wv[1] * qk * m + self.dry_abs[1] * x) * pw,
            ];
            for band in 0..3 {
                let ray = self.rayleigh[band] * x;
                let t = ray + absorb[band];
                let i = band * l + k;
                out.sw_tau[i] = t;
                out.sw_ssa[i] = if t > 0.0 { ray / t } else { 0.0 };
                out.sw_g[i] = 0.0;
            }
            // longwave (flux optical depths → vertical)
            let pr = ctx.p[k] / self.lw_p_ref;
            let pd = pr.powf(self.lw_n);
            let pc = pr.powf(self.lw_n_co2) * lc;
            let rest = (pd * (a + b * (c * qk).ln_1p()) + d * pc).max(0.0);
            let strong = (pd * (a2 + b2 * (c2 * qk).ln_1p()) + d2 * pc).max(0.0);
            let win = (pd * (aw + bw * qk) + dw * pc + cw * qk * qk * pr).max(0.0);
            out.lw_tau[k] = win * x / DIFFUSIVITY;
            out.lw_tau[l + k] = rest * x / DIFFUSIVITY;
            out.lw_tau[2 * l + k] = strong * x / DIFFUSIVITY;
        }
    }
    fn sw_weights(&self, t_eff: f64, out: &mut [f64]) {
        let vis = planck::fraction_below(self.vis_edge, t_eff);
        out[0] = vis;
        out[1] = (1.0 - vis) * self.nir_weak_share;
        out[2] = (1.0 - vis) * (1.0 - self.nir_weak_share);
    }
    fn sw_albedo_band(&self, band: usize) -> usize {
        usize::from(band > 0)
    }
    fn lw_planck_fraction(&self, t: f64, out: &mut [f64]) {
        let tab = self.table.0.get_or_init(|| WindowTable::new(self.window));
        let w = if tab.edges == self.window {
            tab.get(t)
        } else {
            planck::fraction_between(self.window[0], self.window[1], t)
        };
        out[0] = w;
        out[1] = (1.0 - w) * (1.0 - self.lw_strong_share);
        out[2] = (1.0 - w) * self.lw_strong_share;
    }
}

/// `[radiation]` keys of the semi-gray scheme.
pub const KEYS: &[&str] = &[
    "vis_edge",
    "nir_weak_share",
    "rayleigh",
    "k_wv",
    "dry_abs",
    "wv_p_ref",
    "wv_n",
    "ozone_tau",
    "ozone_p",
    "ozone_width",
    "lw_rest",
    "lw_window",
    "lw_rest_strong",
    "lw_strong_share",
    "lw_n",
    "lw_n_co2",
    "lw_co2_curvature",
    "lw_p_ref",
    "albedo",
    "t_surface_default",
    "reflectance",
    "planetary_albedo",
];
