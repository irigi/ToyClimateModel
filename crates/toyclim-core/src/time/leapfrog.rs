//! Leapfrog with the Robert–Asselin–Williams (RAW) filter.

use crate::spectral::Complex;
use crate::Real;

/// RAW filter parameters (Williams 2009).
#[derive(Debug, Clone, Copy)]
pub struct RawFilter {
    /// Filter strength ν.
    pub nu: Real,
    /// Williams α (0.5 = no modification of x^{n+1} beyond RA symmetric; 1 = classic RA).
    pub alpha: Real,
}

impl Default for RawFilter {
    fn default() -> Self {
        RawFilter {
            nu: 0.1,
            alpha: 0.53,
        }
    }
}

impl RawFilter {
    /// After computing x^{n+1}: d = (ν/2)(x^{n−1} − 2x^n + x^{n+1});
    /// x^n += α d; x^{n+1} −= (1−α) d.
    pub fn apply(&self, prev: &[Complex], curr: &mut [Complex], next: &mut [Complex]) {
        let h = 0.5 * self.nu;
        for ((p, c), n) in prev.iter().zip(curr.iter_mut()).zip(next.iter_mut()) {
            let d = (*p - *c * 2.0 + *n) * h;
            *c += d * self.alpha;
            *n -= d * (1.0 - self.alpha);
        }
    }
}

/// Multiply a spectral field by per-coefficient factors (e.g. implicit diffusion).
pub fn scale_by(f: &mut [Complex], factors: &[Real]) {
    for (c, s) in f.iter_mut().zip(factors) {
        *c *= *s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_leaves_linear_trend_alone() {
        // x(t) linear in t has zero second difference: no filtering.
        let f = RawFilter::default();
        let prev = [Complex::new(1.0, 2.0)];
        let mut curr = [Complex::new(2.0, 3.0)];
        let mut next = [Complex::new(3.0, 4.0)];
        f.apply(&prev, &mut curr, &mut next);
        assert_eq!(curr[0], Complex::new(2.0, 3.0));
        assert_eq!(next[0], Complex::new(3.0, 4.0));
    }

    #[test]
    fn raw_damps_computational_mode() {
        // Oscillation +1, −1, +1 is the leapfrog computational mode.
        let f = RawFilter::default();
        let prev = [Complex::new(1.0, 0.0)];
        let mut curr = [Complex::new(-1.0, 0.0)];
        let mut next = [Complex::new(1.0, 0.0)];
        f.apply(&prev, &mut curr, &mut next);
        assert!(curr[0].re > -1.0 && next[0].re < 1.0);
    }
}
