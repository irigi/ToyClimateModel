//! Fractions of blackbody flux in wavelength intervals: shortwave band weights of the star
//! and longwave Planck fractions (SPEC.md §9.4.1).
//!
//! F(λ, T), the fraction of σT⁴ emitted at wavelengths below λ, from the series
//! F = 15/π⁴ Σₙ e^{−nx}/n (x³ + 3x²/n + 6x/n² + 6/n³), x = c₂/(λT) (the integral of the
//! Planck function in closed form term by term; e.g. Widger & Woodall 1976, Bull. Amer.
//! Meteor. Soc. 57, 1217). Converges quickly for x ≳ 1 (all bands used here); for small x
//! more terms are summed.

/// Second radiation constant hc/k \[m K\] (CODATA 2018).
pub const C2: f64 = 1.438_776_877e-2;

/// Fraction of a blackbody's flux at temperature `t` \[K\] below wavelength `lambda` \[m\].
pub fn fraction_below(lambda: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let x = C2 / (lambda * t);
    let mut s = 0.0;
    for n in 1..4000 {
        let n = n as f64;
        let term = (-n * x).exp() / n
            * (x.powi(3) + 3.0 * x * x / n + 6.0 * x / (n * n) + 6.0 / n.powi(3));
        s += term;
        if term < 1e-15 {
            break;
        }
    }
    (15.0 / std::f64::consts::PI.powi(4) * s).clamp(0.0, 1.0)
}

/// Fraction of a blackbody's flux at `t` between wavelengths `l1` < `l2` \[m\].
pub fn fraction_between(l1: f64, l2: f64, t: f64) -> f64 {
    (fraction_below(l2, t) - fraction_below(l1, t)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independent check: Simpson quadrature of the Planck function.
    fn quadrature(l1: f64, l2: f64, t: f64) -> f64 {
        let (h, c, k) = (6.626_070_15e-34, 2.997_924_58e8, 1.380_649e-23);
        let b = |l: f64| 2.0 * h * c * c / l.powi(5) / ((h * c / (l * k * t)).exp() - 1.0);
        let n = 200_000;
        let dl = (l2 - l1) / n as f64;
        let mut s = b(l1) + b(l2);
        for i in 1..n {
            s += b(l1 + i as f64 * dl) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        s * dl / 3.0 * std::f64::consts::PI / (super::super::STEFAN_BOLTZMANN * t.powi(4))
    }

    /// SPEC.md §9.4.1: about 0.54 below 0.75 µm at 5772 K and about 0.11 at 3000 K.
    #[test]
    fn stellar_band_fractions() {
        for (t, want) in [(5772.0, 0.54), (3000.0, 0.11)] {
            let f = fraction_below(0.75e-6, t);
            let q = quadrature(0.01e-6, 0.75e-6, t);
            assert!((f - q).abs() < 2e-4, "{t}: series {f} quadrature {q}");
            assert!((f - want).abs() < 0.01, "{t}: {f}");
        }
    }

    #[test]
    fn window_fraction_and_limits() {
        for t in [200.0, 250.0, 288.0, 320.0] {
            let f = fraction_between(8e-6, 14e-6, t);
            let q = quadrature(8e-6, 14e-6, t);
            assert!((f - q).abs() < 1e-5, "{t}: {f} vs {q}");
        }
        assert!((fraction_between(8e-6, 14e-6, 288.0) - 0.366).abs() < 0.002);
        assert!(fraction_below(1.0, 300.0) > 0.999_999);
        assert!(fraction_below(1e-9, 300.0) < 1e-12);
        assert_eq!(fraction_below(1e-6, 0.0), 0.0);
    }
}
