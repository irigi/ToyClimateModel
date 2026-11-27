//! σ levels with Lorenz staggering and the Simmons & Burridge (1981) discretization
//! (SPEC.md §5.2–5.3). Index k = 0 is the top level (SPEC's k = 1).

/// Vertical coordinate and the matrices of the semi-implicit scheme.
#[derive(Debug, Clone)]
pub struct SigmaLevels {
    /// Number of full levels L.
    pub nlev: usize,
    /// Half levels σ_{k+1/2}, k = 0..=L (σ = 0 at the top, 1 at the surface).
    pub half: Vec<f64>,
    /// Full levels σ_k from ln σ_k = ln σ_{k+1/2} − α_k (consistent with the hydrostatic
    /// discretization; σ_1 = σ_{3/2}/2).
    pub full: Vec<f64>,
    /// Δσ_k.
    pub dsigma: Vec<f64>,
    /// α_k: ln 2 at the top, 1 − (σ_{k−1/2}/Δσ_k) ln(σ_{k+1/2}/σ_{k−1/2}) below.
    pub alpha: Vec<f64>,
    /// ln(σ_{k+1/2}/σ_{k−1/2}); unused (infinite) at the top, stored as 0.
    pub lnratio: Vec<f64>,
}

impl SigmaLevels {
    /// From half levels (0 → 1, strictly increasing).
    pub fn from_half(half: Vec<f64>) -> Self {
        let nlev = half.len() - 1;
        assert!(nlev >= 1 && half[0] == 0.0 && half[nlev] == 1.0);
        let dsigma: Vec<f64> = (0..nlev).map(|k| half[k + 1] - half[k]).collect();
        let mut alpha = vec![0.0; nlev];
        let mut lnratio = vec![0.0; nlev];
        alpha[0] = std::f64::consts::LN_2;
        for k in 1..nlev {
            lnratio[k] = (half[k + 1] / half[k]).ln();
            alpha[k] = 1.0 - half[k] / dsigma[k] * lnratio[k];
        }
        let full = (0..nlev)
            .map(|k| (half[k + 1].ln() - alpha[k]).exp())
            .collect();
        SigmaLevels {
            nlev,
            half,
            full,
            dsigma,
            alpha,
            lnratio,
        }
    }

    /// L equally spaced levels.
    pub fn equal(nlev: usize) -> Self {
        Self::from_half((0..=nlev).map(|k| k as f64 / nlev as f64).collect())
    }

    /// ω_k/p_k \[s⁻¹\] of one column from the horizontal divergence `div` and v·∇ln p_s `vg`
    /// per full level \[s⁻¹\], written to `out`: the discretisation of the explicit
    /// tendencies (Simmons & Burridge 1981), ω_k/p_k = (v·∇ln p_s)_k − [ln(σ_{k+1/2}/σ_{k−1/2})
    /// Σ_{i<k} Δσ_i D_i + α_k Δσ_k D_k]/Δσ_k with D_i = δ_i + (v·∇ln p_s)_i.
    pub fn omega_over_p(&self, div: &[f64], vg: &[f64], out: &mut [f64]) {
        let mut cum = 0.0;
        for k in 0..self.nlev {
            let d = div[k] + vg[k];
            let above = if k > 0 { self.lnratio[k] * cum } else { 0.0 };
            out[k] = vg[k] - (above + self.alpha[k] * self.dsigma[k] * d) / self.dsigma[k];
            cum += self.dsigma[k] * d;
        }
    }

    /// Hydrostatic matrix G (L×L, row-major) times R: Φ'_k = Σ_j G_kj T'_j.
    /// G_kj = R ln(σ_{j+1/2}/σ_{j−1/2}) for j > k, G_kk = R α_k, 0 for j < k.
    pub fn g_matrix(&self, r: f64) -> Vec<f64> {
        let l = self.nlev;
        let mut g = vec![0.0; l * l];
        for k in 0..l {
            g[k * l + k] = r * self.alpha[k];
            for j in (k + 1)..l {
                g[k * l + j] = r * self.lnratio[j];
            }
        }
        g
    }

    /// τ (L×L): linear T tendency −τ δ. τ_kj = κT̄ ln(σ_{k+1/2}/σ_{k−1/2}) Δσ_j/Δσ_k for
    /// j < k, τ_kk = κ T̄ α_k, 0 for j > k.
    pub fn tau_matrix(&self, kappa: f64, tref: f64) -> Vec<f64> {
        let l = self.nlev;
        let mut t = vec![0.0; l * l];
        for k in 0..l {
            t[k * l + k] = kappa * tref * self.alpha[k];
            for j in 0..k {
                t[k * l + j] = kappa * tref * self.lnratio[k] * self.dsigma[j] / self.dsigma[k];
            }
        }
        t
    }

    /// B = G τ + R T̄ 1 wᵀ, whose eigenvalues are the squared gravity-wave speeds of the
    /// vertical normal modes (frequency² = Λ_ℓ · eigenvalue).
    pub fn b_matrix(&self, r: f64, kappa: f64, tref: f64) -> Vec<f64> {
        let l = self.nlev;
        let g = self.g_matrix(r);
        let tau = self.tau_matrix(kappa, tref);
        let mut b = vec![0.0; l * l];
        for i in 0..l {
            for j in 0..l {
                let gt: f64 = (0..l).map(|k| g[i * l + k] * tau[k * l + j]).sum();
                b[i * l + j] = gt + r * tref * self.dsigma[j];
            }
        }
        b
    }
}

/// Half levels of the "L26" set: Jablonowski & Williamson (2006), Table 1 (η = σ here).
pub const L26_HALF: [f64; 27] = [
    0.0,
    0.004895209,
    0.009882418,
    0.01805201,
    0.02983724,
    0.04462334,
    0.06160587,
    0.07851243,
    0.0923658,
    0.10866359,
    0.12783708,
    0.15039371,
    0.17693043,
    0.20814944,
    0.24487709,
    0.28808522,
    0.33891731,
    0.39871865,
    0.4690718,
    0.55183871,
    0.64920969,
    0.74438289,
    0.83102123,
    0.90330029,
    0.95599746,
    0.9851122,
    1.0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydrostatic_matrix_integrates_isothermal_exactly() {
        // For an isothermal atmosphere Φ − Φ_s = R T ln(1/σ) at the full levels defined by α.
        let s = SigmaLevels::equal(10);
        let r = 287.0;
        let g = s.g_matrix(r);
        for k in 0..10 {
            let phi: f64 = (0..10).map(|j| g[k * 10 + j] * 250.0).sum();
            let exact = -r * 250.0 * s.full[k].ln();
            assert!((phi / exact - 1.0).abs() < 1e-12, "k={k}");
        }
        assert!(s.dsigma.iter().sum::<f64>() == 1.0);
    }

    /// Uniform divergence without a surface-pressure gradient: ∂ω/∂p = −δ gives ω/p = −δ
    /// exactly below the top level (the α_k identity), and a pressure gradient adds v·∇ln p_s
    /// with the column's integrated D vanishing only where it should.
    #[test]
    fn omega_over_p_uniform_divergence() {
        let s = SigmaLevels::from_half(L26_HALF.to_vec());
        let l = s.nlev;
        let (div, vg) = (vec![2e-6; l], vec![0.0; l]);
        let mut out = vec![0.0; l];
        s.omega_over_p(&div, &vg, &mut out);
        for (k, w) in out.iter().enumerate().skip(1) {
            assert!((w / -2e-6 - 1.0).abs() < 1e-12, "k={k}: {w}");
        }
        // convergence below, divergence above with zero column mean: ω = 0 at the surface
        // half level, so ω/p at the lowest full level is small and of the sign of the flow
        let div: Vec<f64> = (0..l)
            .map(|k| if s.full[k] > 0.8 { -1e-5 } else { 0.0 })
            .collect();
        let mean: f64 = (0..l).map(|k| s.dsigma[k] * div[k]).sum();
        let div: Vec<f64> = div.iter().map(|d| d - mean).collect();
        s.omega_over_p(&div, &vg, &mut out);
        let k8 = s.full.iter().position(|&x| x > 0.8).unwrap();
        assert!(
            out[k8 - 1] < 0.0,
            "ascent above the convergence: {}",
            out[k8 - 1]
        );
        assert!(out[l - 1].abs() < out[k8].abs(), "{out:?}");
    }

    #[test]
    fn normal_mode_speeds_are_positive() {
        let s = SigmaLevels::equal(20);
        let b = s.b_matrix(287.04, 287.04 / 1004.64, 300.0);
        let ev = crate::util::eigenvalues_real(20, &b, 3000);
        assert!(ev.iter().all(|&e| e > 0.0), "{ev:?}");
        let mx = ev.iter().cloned().fold(0.0, f64::max);
        // external (Lamb) mode of an isothermal atmosphere: c² = R T̄ / (1 − κ) ≈ 1.21e5
        let lamb = 287.04 * 300.0 / (1.0 - 287.04 / 1004.64);
        assert!((mx / lamb - 1.0).abs() < 0.06, "{mx} vs {lamb}");
    }
}
