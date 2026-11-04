//! Small helpers: a deterministic pseudo-random generator for tests and initial
//! perturbations (no external RNG dependency, identical on every platform).

/// SplitMix64 generator.
#[derive(Debug, Clone)]
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [−1, 1).
    pub fn symmetric(&mut self) -> f64 {
        2.0 * self.uniform() - 1.0
    }
}

/// Invert a dense n×n row-major matrix by Gauss–Jordan elimination with partial pivoting.
/// Returns None if the matrix is singular to working precision.
pub fn invert(n: usize, a: &[f64]) -> Option<Vec<f64>> {
    assert_eq!(a.len(), n * n);
    let mut m = a.to_vec();
    let mut inv = vec![0.0; n * n];
    for i in 0..n {
        inv[i * n + i] = 1.0;
    }
    for col in 0..n {
        let piv =
            (col..n).max_by(|&x, &y| m[x * n + col].abs().total_cmp(&m[y * n + col].abs()))?;
        if m[piv * n + col].abs() < 1e-300 {
            return None;
        }
        if piv != col {
            for k in 0..n {
                m.swap(piv * n + k, col * n + k);
                inv.swap(piv * n + k, col * n + k);
            }
        }
        let d = 1.0 / m[col * n + col];
        for k in 0..n {
            m[col * n + k] *= d;
            inv[col * n + k] *= d;
        }
        for r in 0..n {
            if r != col {
                let f = m[r * n + col];
                if f != 0.0 {
                    for k in 0..n {
                        m[r * n + k] -= f * m[col * n + k];
                        inv[r * n + k] -= f * inv[col * n + k];
                    }
                }
            }
        }
    }
    Some(inv)
}

/// Real eigenvalues of a general n×n matrix by the unshifted QR algorithm on its Hessenberg
/// form is overkill here; this uses simple QR iterations (adequate for the small,
/// well-conditioned vertical-mode matrices, whose eigenvalues are real and distinct).
pub fn eigenvalues_real(n: usize, a: &[f64], iters: usize) -> Vec<f64> {
    let mut m = a.to_vec();
    for _ in 0..iters {
        // Gram–Schmidt QR of m (columns)
        let mut q = vec![0.0; n * n];
        let mut r = vec![0.0; n * n];
        for j in 0..n {
            let mut v: Vec<f64> = (0..n).map(|i| m[i * n + j]).collect();
            for k in 0..j {
                let dot: f64 = (0..n).map(|i| q[i * n + k] * m[i * n + j]).sum();
                r[k * n + j] = dot;
                for i in 0..n {
                    v[i] -= dot * q[i * n + k];
                }
            }
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            r[j * n + j] = norm;
            for i in 0..n {
                q[i * n + j] = if norm > 0.0 { v[i] / norm } else { 0.0 };
            }
        }
        // m = R Q
        let mut next = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                next[i * n + j] = (0..n).map(|k| r[i * n + k] * q[k * n + j]).sum();
            }
        }
        m = next;
    }
    (0..n).map(|i| m[i * n + i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_and_eigen() {
        let a = [4.0, 1.0, 0.5, 1.0, 3.0, 0.2, 0.5, 0.2, 2.0];
        let inv = invert(3, &a).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let s: f64 = (0..3).map(|k| a[i * 3 + k] * inv[k * 3 + j]).sum();
                assert!((s - if i == j { 1.0 } else { 0.0 }).abs() < 1e-14);
            }
        }
        let mut ev = eigenvalues_real(3, &a, 500);
        ev.sort_by(f64::total_cmp);
        let tr: f64 = ev.iter().sum();
        assert!((tr - 9.0).abs() < 1e-10);
        assert!(invert(2, &[1.0, 2.0, 2.0, 4.0]).is_none());
    }
}
