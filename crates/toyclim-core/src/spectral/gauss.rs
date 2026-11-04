//! Gaussian latitudes and weights.
//!
//! The nodes μ_j are the roots of the Legendre polynomial P_N(μ), found by Newton
//! iteration from asymptotic initial guesses. Weights are
//! w_j = 2 / ((1 − μ_j²) P'_N(μ_j)²), so Σ w_j = 2.
//! Nodes are returned north to south (μ decreasing), matching the grid convention.

/// Evaluate P_n(x) and P'_n(x) by the three-term recurrence.
fn legendre_and_derivative(n: usize, x: f64) -> (f64, f64) {
    let mut p0 = 1.0;
    let mut p1 = x;
    if n == 0 {
        return (1.0, 0.0);
    }
    for k in 2..=n {
        let kf = k as f64;
        let p2 = ((2.0 * kf - 1.0) * x * p1 - (kf - 1.0) * p0) / kf;
        p0 = p1;
        p1 = p2;
    }
    let nf = n as f64;
    let dp = nf * (x * p1 - p0) / (x * x - 1.0);
    (p1, dp)
}

/// Gaussian nodes μ_j (north to south) and weights w_j for `n` latitudes.
///
/// Computed in f64 regardless of the `Real` precision.
pub fn gauss_legendre(n: usize) -> (Vec<f64>, Vec<f64>) {
    assert!(n > 0, "need at least one Gaussian latitude");
    let mut mu = vec![0.0; n];
    let mut w = vec![0.0; n];
    let nf = n as f64;
    let half = n.div_ceil(2);
    for i in 0..half {
        // Tricomi asymptotic guess for the (i+1)-th root, counting from μ = 1.
        let theta = std::f64::consts::PI * (i as f64 + 0.75) / (nf + 0.5);
        let mut x = (1.0 - (nf - 1.0) / (8.0 * nf * nf * nf)) * theta.cos();
        for _ in 0..100 {
            let (p, dp) = legendre_and_derivative(n, x);
            let dx = p / dp;
            x -= dx;
            if dx.abs() < 1e-16 {
                break;
            }
        }
        let (_, dp) = legendre_and_derivative(n, x);
        let wi = 2.0 / ((1.0 - x * x) * dp * dp);
        mu[i] = x;
        mu[n - 1 - i] = -x;
        w[i] = wi;
        w[n - 1 - i] = wi;
    }
    if !n.is_multiple_of(2) {
        mu[n / 2] = 0.0;
    }
    (mu, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_two() {
        for n in [1, 2, 3, 32, 48, 64, 96, 128, 192, 256] {
            let (_, w) = gauss_legendre(n);
            let s: f64 = w.iter().sum();
            assert!((s - 2.0).abs() < 1e-14, "n={n}: sum={s}");
        }
    }

    #[test]
    fn integrates_monomials_exactly() {
        for n in [32, 64, 128, 256] {
            let (mu, w) = gauss_legendre(n);
            for p in 0..(2 * n) {
                let q: f64 = mu.iter().zip(&w).map(|(m, wi)| wi * m.powi(p as i32)).sum();
                let exact = if p % 2 == 1 {
                    0.0
                } else {
                    2.0 / (p as f64 + 1.0)
                };
                assert!((q - exact).abs() < 1e-13, "n={n} p={p}: {q} vs {exact}");
            }
        }
    }

    #[test]
    fn nodes_north_to_south() {
        let (mu, _) = gauss_legendre(64);
        assert!(mu.windows(2).all(|p| p[0] > p[1]));
        assert!(mu[0] < 1.0 && mu[63] > -1.0);
    }
}
