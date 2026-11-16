//! Surface albedo per tile in two bands, visible (λ ≤ 0.7 µm) and near-IR (SPEC.md §9.4.6);
//! until P3 the broadband value is the mean of the two weighted by the star's visible
//! fraction (Planck spectrum at T_eff).
//!
//! Snow and sea ice: CAM6 Scientific Guide §8.3 (the CCSM3 two-band parameterization used by
//! Shields et al. 2013, Astrobiology 13, 715, §2.2):
//! - dry snow 0.96 / 0.68; melting snow (T_s ≥ T_melt − 1 K) loses 0.10 / 0.15 per K;
//! - bare dry sea ice (thicker than 0.5 m) 0.73 / 0.33; melting loses 0.075 / 0.075 per K;
//! - snow over ice covers f_s = h_s / (h_s + 0.02 m) (with ρ_s = 330 kg m⁻³:
//!   S / (S + 6.6 kg m⁻²)).
//!
//! Ocean 0.06 / 0.06 and bare land 0.15 / 0.30: SPEC.md §9.4.6 starting values **[knobs]**,
//! not from a paper. Snow on land covers f_snow = S / (S + 15 kg m⁻²) **[knob]**.

/// Two-band albedo \[visible, near-IR\].
pub type Bands = [f64; 2];

/// Albedo parameters.
#[derive(Debug, Clone)]
pub struct AlbedoParams {
    /// Fraction of the stellar flux at λ ≤ 0.7 µm.
    pub visible_fraction: f64,
    pub ocean: Bands,
    pub land: Bands,
    pub snow_dry: Bands,
    /// Loss per K above T_melt − 1 K.
    pub snow_melt: Bands,
    pub ice_dry: Bands,
    pub ice_melt: Bands,
    /// S_c of the snow cover fraction S / (S + S_c) on land and on sea ice \[kg m⁻²\].
    pub snow_cover_land: f64,
    pub snow_cover_ice: f64,
}

impl Default for AlbedoParams {
    fn default() -> Self {
        AlbedoParams {
            visible_fraction: visible_fraction(5772.0),
            ocean: [0.06, 0.06],
            land: [0.15, 0.30],
            snow_dry: [0.96, 0.68],
            snow_melt: [0.10, 0.15],
            ice_dry: [0.73, 0.33],
            ice_melt: [0.075, 0.075],
            snow_cover_land: 15.0,
            snow_cover_ice: 0.02 * 330.0,
        }
    }
}

impl AlbedoParams {
    /// Broadband value of a two-band albedo.
    pub fn broadband(&self, a: Bands) -> f64 {
        self.visible_fraction * a[0] + (1.0 - self.visible_fraction) * a[1]
    }
    fn melt(dry: Bands, loss: Bands, ts: f64, t_melt: f64) -> Bands {
        let d = (ts - (t_melt - 1.0)).clamp(0.0, 1.0);
        [dry[0] - loss[0] * d, dry[1] - loss[1] * d]
    }
    pub fn snow(&self, ts: f64, t_melt: f64) -> Bands {
        Self::melt(self.snow_dry, self.snow_melt, ts, t_melt)
    }
    pub fn ice(&self, ts: f64, t_melt: f64) -> Bands {
        Self::melt(self.ice_dry, self.ice_melt, ts, t_melt)
    }
}

/// Blend of a bare surface and snow with cover fraction `f`.
pub fn blend(bare: Bands, snow: Bands, f: f64) -> Bands {
    [
        bare[0] * (1.0 - f) + snow[0] * f,
        bare[1] * (1.0 - f) + snow[1] * f,
    ]
}

/// Fraction of a blackbody spectrum at temperature `t` \[K\] below λ = 0.7 µm, from the
/// series F(x) = 15/π⁴ Σₙ e^{−nx}/n (x³ + 3x²/n + 6x/n² + 6/n³), x = hc/(λkT).
pub fn visible_fraction(t: f64) -> f64 {
    let c2 = 1.438_776_877e-2; // hc/k [m K] (CODATA 2018)
    let x = c2 / (0.7e-6 * t);
    let mut s = 0.0;
    for n in 1..200 {
        let n = n as f64;
        let term = (-n * x).exp() / n
            * (x.powi(3) + 3.0 * x * x / n + 6.0 * x / (n * n) + 6.0 / n.powi(3));
        s += term;
        if term < 1e-15 {
            break;
        }
    }
    15.0 / std::f64::consts::PI.powi(4) * s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_fractions() {
        // the Sun: about half the flux below 0.7 µm; an M dwarf (3000 K) much less
        let sun = visible_fraction(5772.0);
        assert!((0.44..0.52).contains(&sun), "{sun}");
        assert!(visible_fraction(3000.0) < 0.2);
        // whole spectrum: x → 0 gives 1
        assert!((visible_fraction(1e7) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn snow_and_ice_melt_darken() {
        let a = AlbedoParams::default();
        assert_eq!(a.snow(250.0, 273.15), [0.96, 0.68]);
        let m = a.snow(273.15, 273.15);
        assert!((m[0] - 0.86).abs() < 1e-12 && (m[1] - 0.53).abs() < 1e-12);
        let i = a.ice(273.15, 273.15);
        assert!((i[0] - 0.655).abs() < 1e-12 && (i[1] - 0.255).abs() < 1e-12);
    }
}
