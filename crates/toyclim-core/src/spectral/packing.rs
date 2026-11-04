//! Triangular spectral storage (SPEC.md §4.1).
//!
//! Coefficients f_ℓ^m with 0 ≤ m ≤ ℓ ≤ M are stored m-major, contiguous in ℓ:
//! `[ (0,0) (1,0) … (M,0) | (1,1) … (M,1) | … | (M,M) ]`.
//! Only m ≥ 0 is stored; for real fields f_ℓ^{−m} = conj(f_ℓ^m).

use crate::Real;

/// Complex spectral coefficient.
pub type Complex = realfft::num_complex::Complex<Real>;

/// Index helper for triangular truncation T`mmax`.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Truncation M.
    pub mmax: usize,
    /// Start of each m block.
    offset: Vec<usize>,
    /// ℓ of each packed index.
    pub l_of: Vec<usize>,
    /// m of each packed index.
    pub m_of: Vec<usize>,
}

impl Layout {
    pub fn new(mmax: usize) -> Self {
        let mut offset = Vec::with_capacity(mmax + 2);
        let mut l_of = Vec::new();
        let mut m_of = Vec::new();
        let mut k = 0;
        for m in 0..=mmax {
            offset.push(k);
            for l in m..=mmax {
                l_of.push(l);
                m_of.push(m);
                k += 1;
            }
        }
        offset.push(k);
        Layout {
            mmax,
            offset,
            l_of,
            m_of,
        }
    }

    /// Number of stored coefficients, (M+1)(M+2)/2.
    pub fn len(&self) -> usize {
        self.l_of.len()
    }

    pub fn is_empty(&self) -> bool {
        self.l_of.is_empty()
    }

    /// Packed index of (ℓ, m). Requires m ≤ ℓ ≤ M.
    #[inline]
    pub fn idx(&self, l: usize, m: usize) -> usize {
        debug_assert!(m <= l && l <= self.mmax);
        self.offset[m] + (l - m)
    }

    /// Range of packed indices for zonal wavenumber m.
    #[inline]
    pub fn m_range(&self, m: usize) -> std::ops::Range<usize> {
        self.offset[m]..self.offset[m + 1]
    }

    /// A zero spectral field.
    pub fn zeros(&self) -> Vec<Complex> {
        vec![Complex::new(0.0, 0.0); self.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexing_is_dense_and_consistent() {
        let lay = Layout::new(21);
        assert_eq!(lay.len(), 22 * 23 / 2);
        let mut seen = vec![false; lay.len()];
        for m in 0..=21 {
            for l in m..=21 {
                let k = lay.idx(l, m);
                assert!(!seen[k]);
                seen[k] = true;
                assert_eq!((lay.l_of[k], lay.m_of[k]), (l, m));
            }
            assert_eq!(lay.m_range(m).len(), 22 - m);
        }
        assert!(seen.iter().all(|&s| s));
    }
}
