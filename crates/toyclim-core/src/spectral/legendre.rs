//! Normalized associated Legendre functions (SPEC.md §4.2).
//!
//! ∫_{−1}^{1} P̄_ℓ^m P̄_{ℓ'}^m dμ = δ_{ℓℓ'}, no Condon–Shortley phase.
//! H_ℓ^m ≡ (1 − μ²) dP̄_ℓ^m/dμ = −ℓ ε_{ℓ+1}^m P̄_{ℓ+1}^m + (ℓ+1) ε_ℓ^m P̄_{ℓ−1}^m.
//! All values are computed in f64.

/// ε_ℓ^m = √((ℓ² − m²)/(4ℓ² − 1)); zero for ℓ < m or ℓ = 0.
pub fn epsilon(l: usize, m: usize) -> f64 {
    if l < m || l == 0 {
        return 0.0;
    }
    let (l, m) = (l as f64, m as f64);
    ((l * l - m * m) / (4.0 * l * l - 1.0)).sqrt()
}

/// P̄_ℓ^m(μ) for m ≤ ℓ ≤ `lmax`, returned as a vector indexed by ℓ − m.
pub fn pbar_column(m: usize, lmax: usize, mu: f64) -> Vec<f64> {
    let mut out = vec![0.0; lmax + 1 - m];
    let s = (1.0 - mu * mu).max(0.0).sqrt();
    // P̄_m^m by the sectoral recurrence.
    let mut pmm = std::f64::consts::FRAC_1_SQRT_2;
    for k in 1..=m {
        let kf = k as f64;
        pmm *= ((2.0 * kf + 1.0) / (2.0 * kf)).sqrt() * s;
    }
    out[0] = pmm;
    if lmax == m {
        return out;
    }
    out[1] = (2.0 * m as f64 + 3.0).sqrt() * mu * pmm;
    for l in (m + 2)..=lmax {
        let i = l - m;
        out[i] = (mu * out[i - 1] - epsilon(l - 1, m) * out[i - 2]) / epsilon(l, m);
    }
    out
}

/// P̄_ℓ^m and H_ℓ^m at μ for m ≤ ℓ ≤ `lmax`, each indexed by ℓ − m.
pub fn pbar_and_h(m: usize, lmax: usize, mu: f64) -> (Vec<f64>, Vec<f64>) {
    let p = pbar_column(m, lmax + 1, mu);
    let n = lmax + 1 - m;
    let mut h = vec![0.0; n];
    for i in 0..n {
        let l = m + i;
        let up = -(l as f64) * epsilon(l + 1, m) * p[i + 1];
        let down = if i > 0 {
            (l as f64 + 1.0) * epsilon(l, m) * p[i - 1]
        } else {
            0.0
        };
        h[i] = up + down;
    }
    (p[..n].to_vec(), h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::gauss::gauss_legendre;

    #[test]
    fn low_order_closed_forms() {
        let mu: f64 = 0.37;
        let s = (1.0 - mu * mu).sqrt();
        let p0 = pbar_column(0, 2, mu);
        assert!((p0[0] - 1.0 / 2f64.sqrt()).abs() < 1e-15);
        assert!((p0[1] - (1.5f64).sqrt() * mu).abs() < 1e-15);
        assert!((p0[2] - (2.5f64).sqrt() * 0.5 * (3.0 * mu * mu - 1.0)).abs() < 1e-15);
        let p1 = pbar_column(1, 1, mu);
        // P̄_1^1 = √(3/4) s (no Condon–Shortley phase).
        assert!((p1[0] - (0.75f64).sqrt() * s).abs() < 1e-15);
    }

    #[test]
    fn h_matches_finite_difference() {
        for (m, lmax) in [(0, 10), (3, 12), (7, 9)] {
            let mu = 0.41;
            let (_, h) = pbar_and_h(m, lmax, mu);
            let d = 1e-6;
            let pp = pbar_column(m, lmax, mu + d);
            let pm = pbar_column(m, lmax, mu - d);
            for i in 0..h.len() {
                let fd = (1.0 - mu * mu) * (pp[i] - pm[i]) / (2.0 * d);
                assert!(
                    (h[i] - fd).abs() < 1e-7,
                    "m={m} l={} {} {}",
                    m + i,
                    h[i],
                    fd
                );
            }
        }
    }

    /// M1 test 2: orthonormality at T170 on its Gaussian grid.
    #[test]
    fn orthonormality_t170() {
        let mmax = 170;
        let (mu, w) = gauss_legendre(256);
        for m in [0, 1, 2, 17, 85, 120, 169, 170] {
            let cols: Vec<Vec<f64>> = mu.iter().map(|&x| pbar_column(m, mmax, x)).collect();
            let n = mmax + 1 - m;
            let mut worst: f64 = 0.0;
            for a in 0..n {
                for b in a..n {
                    let s: f64 = (0..mu.len()).map(|j| w[j] * cols[j][a] * cols[j][b]).sum();
                    let target = if a == b { 1.0 } else { 0.0 };
                    worst = worst.max((s - target).abs());
                }
            }
            eprintln!("M1.2 orthonormality T170 m={m}: {worst:.2e}");
            assert!(worst < 1e-13, "m={m}: worst {worst:e}");
        }
    }
}
