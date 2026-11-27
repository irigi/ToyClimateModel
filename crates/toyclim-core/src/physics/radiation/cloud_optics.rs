//! Cloud optical properties (SPEC.md §9.4.7) from in-cloud liquid and ice water paths.
//!
//! Shortwave, per Slingo band i: τ = W (a_i + b_i / r_e) with W in g m⁻², 1 − ω = c_i + d_i r_e,
//! g = e_i + f_i r_e (r_e in µm).
//! - Liquid: Slingo, A. (1989), J. Atmos. Sci. 46, 1419–1427, the four bands 0.25–0.69,
//!   0.69–1.19, 1.19–2.38 and 2.38–4.00 µm; valid for r_e 4.2–16 µm.
//! - Ice: Ebert, E. E. and Curry, J. A. (1992), J. Geophys. Res. 97, 3831–3836, valid for
//!   r_e 13–130 µm, in the four-band form of NCAR CCM3/CAM.
//! - The coefficients are transcribed from the NCAR CAM source (`slingo_liq_optics.F90`,
//!   `ebert_curry_ice_optics.F90`, ESCOMP/CAM on GitHub; not GPL), which quotes both
//!   papers. The papers themselves were not accessible (paywalled, 2026-11-18).
//!
//! Longwave: absorption optical depth k W, the same in every band (gray clouds). Liquid
//! k = 0.090361 m² g⁻¹ and ice k = 0.005 + 1/r_e m² g⁻¹ (r_e in µm), both as in CAM. These
//! are vertical optical depths: the solver applies the diffusivity 1.66, so the liquid flux
//! coefficient is 0.150 m² g⁻¹ (SPEC.md §9.4.1 quotes k_lw ≈ 0.14).
//!
//! The model's shortwave bands map onto these by their surface-albedo band: the visible
//! band uses Slingo band 1, the near-IR bands use bands 2–4 combined with weights equal to
//! the star's flux in each (τ, τω and τωg averaged).

use super::planck::fraction_between;

/// Slingo (1989) liquid coefficients `[a, b, c, d, e, f]` per band (a in m² g⁻¹, b in
/// µm m² g⁻¹, d and f in µm⁻¹).
const SLINGO: [[f64; 6]; 4] = [
    [2.817e-2, 1.305, -5.62e-8, 1.63e-7, 0.829, 2.482e-3],
    [2.682e-2, 1.346, -6.94e-6, 2.35e-5, 0.794, 4.226e-3],
    [2.264e-2, 1.454, 4.64e-4, 1.24e-3, 0.754, 6.560e-3],
    [1.281e-2, 1.641, 0.201, 7.56e-3, 0.826, 4.353e-3],
];

/// Ebert & Curry (1992) ice coefficients in the CCM3/CAM four-band form.
const EBERT_CURRY: [[f64; 6]; 4] = [
    [3.448e-3, 2.431, 1.00e-5, 0.0, 0.7661, 5.851e-4],
    [3.448e-3, 2.431, 1.10e-4, 1.405e-5, 0.7730, 5.665e-4],
    [3.448e-3, 2.431, 1.861e-2, 8.328e-4, 0.794, 7.267e-4],
    [3.448e-3, 2.431, 0.46658, 2.05e-5, 0.9595, 1.076e-4],
];

/// Band edges of the four Slingo bands \[m\].
const EDGES: [f64; 5] = [0.25e-6, 0.69e-6, 1.19e-6, 2.38e-6, 4.0e-6];

/// Mass extinction \[m² kg⁻¹\], single-scattering albedo and asymmetry of one band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandOptics {
    pub k_ext: f64,
    pub ssa: f64,
    pub g: f64,
}

/// Cloud optics for fixed effective radii: per surface-albedo band (0 visible, 1 near-IR)
/// for liquid and ice, and the longwave absorption coefficients \[m² kg⁻¹\].
#[derive(Debug, Clone)]
pub struct CloudOpticsParams {
    pub liquid: [BandOptics; 2],
    pub ice: [BandOptics; 2],
    pub k_lw_liquid: f64,
    pub k_lw_ice: f64,
}

fn band(c: &[f64; 6], r: f64) -> BandOptics {
    BandOptics {
        // per g m⁻² → per kg m⁻²
        k_ext: 1000.0 * (c[0] + c[1] / r),
        ssa: (1.0 - c[2] - c[3] * r).min(1.0 - 1e-9),
        g: c[4] + c[5] * r,
    }
}

/// τ-, τω- and τωg-weighted mean of bands with weights `w`.
fn combine(bands: &[BandOptics], w: &[f64]) -> BandOptics {
    let (mut t, mut ws, mut wsg, mut sum) = (0.0, 0.0, 0.0, 0.0);
    for (b, &x) in bands.iter().zip(w) {
        t += x * b.k_ext;
        ws += x * b.k_ext * b.ssa;
        wsg += x * b.k_ext * b.ssa * b.g;
        sum += x;
    }
    BandOptics {
        k_ext: t / sum,
        ssa: ws / t,
        g: wsg / ws,
    }
}

impl CloudOpticsParams {
    /// Optics for liquid and ice effective radii `r_liquid`, `r_ice` \[µm\] (clamped to
    /// the ranges of the fits), liquid longwave coefficient `k_lw_liquid` \[m² g⁻¹\] and a
    /// star of temperature `t_eff` \[K\] (near-IR band weights).
    pub fn new(r_liquid: f64, r_ice: f64, k_lw_liquid: f64, t_eff: f64) -> Self {
        let rl = r_liquid.clamp(4.2, 16.0);
        let ri = r_ice.clamp(13.0, 130.0);
        let w: Vec<f64> = (1..4)
            .map(|i| fraction_between(EDGES[i], EDGES[i + 1], t_eff).max(1e-12))
            .collect();
        let liq: Vec<BandOptics> = SLINGO.iter().map(|c| band(c, rl)).collect();
        let ice: Vec<BandOptics> = EBERT_CURRY.iter().map(|c| band(c, ri)).collect();
        CloudOpticsParams {
            liquid: [liq[0], combine(&liq[1..], &w)],
            ice: [ice[0], combine(&ice[1..], &w)],
            k_lw_liquid: 1000.0 * k_lw_liquid,
            k_lw_ice: 1000.0 * (0.005 + 1.0 / ri),
        }
    }

    /// Shortwave optical depth, single-scattering albedo and asymmetry of a layer with
    /// in-cloud liquid and ice paths `lwp`, `iwp` \[kg m⁻²\] in surface-albedo band `ab`.
    pub fn shortwave(&self, ab: usize, lwp: f64, iwp: f64) -> (f64, f64, f64) {
        let (l, i) = (&self.liquid[ab], &self.ice[ab]);
        let (tl, ti) = (lwp * l.k_ext, iwp * i.k_ext);
        let t = tl + ti;
        if t <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let ws = tl * l.ssa + ti * i.ssa;
        let wsg = tl * l.ssa * l.g + ti * i.ssa * i.g;
        (t, ws / t, wsg / ws)
    }

    /// Longwave absorption optical depth (vertical) of a layer.
    pub fn longwave(&self, lwp: f64, iwp: f64) -> f64 {
        lwp * self.k_lw_liquid + iwp * self.k_lw_ice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquid_cloud_values() {
        let c = CloudOpticsParams::new(10.0, 30.0, 0.090361, 5772.0);
        // visible: τ per 100 g m⁻² at 10 µm = 100 (0.02817 + 0.1305) = 15.9, ω ≈ 1, g ≈ 0.854
        let (t, w, g) = c.shortwave(0, 0.1, 0.0);
        assert!((t - 15.867).abs() < 1e-3, "{t}");
        assert!(w > 0.99999 && w < 1.0);
        assert!((g - 0.85382).abs() < 1e-5, "{g}");
        // near-IR absorbs: ω below 1, still conservative to ~1 %
        let (_, wn, _) = c.shortwave(1, 0.1, 0.0);
        assert!(wn < 0.999 && wn > 0.95, "{wn}");
        // longwave: 20 g m⁻² gives emissivity 1 − exp(−1.66 τ) ≈ 0.95
        let tl = c.longwave(0.02, 0.0);
        assert!((1.0 - (-1.66 * tl).exp() - 0.95).abs() < 0.01, "{tl}");
    }

    #[test]
    fn ice_and_mixtures() {
        let c = CloudOpticsParams::new(10.0, 30.0, 0.090361, 5772.0);
        let (ti, wi, gi) = c.shortwave(0, 0.0, 0.01);
        assert!((ti - 10.0 * (3.448e-3 + 2.431 / 30.0)).abs() < 1e-9, "{ti}");
        assert!(wi > 0.9999 && gi > 0.77 && gi < 0.80);
        let (t, w, g) = c.shortwave(1, 0.05, 0.01);
        let (tl, wl, gl) = c.shortwave(1, 0.05, 0.0);
        let (tj, wj, gj) = c.shortwave(1, 0.0, 0.01);
        assert!((t - tl - tj).abs() < 1e-12);
        assert!((w * t - wl * tl - wj * tj).abs() < 1e-9);
        assert!((g * w * t - gl * wl * tl - gj * wj * tj).abs() < 1e-9);
        assert_eq!(c.shortwave(0, 0.0, 0.0), (0.0, 0.0, 0.0));
        // a cooler star puts more of its near-IR in the absorbing long-wave bands
        let m = CloudOpticsParams::new(10.0, 30.0, 0.090361, 3000.0);
        assert!(m.liquid[1].ssa < c.liquid[1].ssa);
    }
}
