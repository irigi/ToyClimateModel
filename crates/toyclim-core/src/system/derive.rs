//! Derived orbital quantities and consistency checks of explicit overrides
//! (SPEC.md §9.3.2).

use super::star::Star;
use std::f64::consts::TAU;

/// Total solar irradiance at 1 AU \[W m⁻²\] (Kopp & Lean 2011, 1360.8 ± 0.5, rounded as in
/// SPEC.md). A named parameter, not a knob.
pub const SOLAR_CONSTANT: f64 = 1361.0;

/// Sidereal orbital period of a body at 1 AU around 1 M☉ \[s\] (365.256 days).
pub const YEAR_1AU: f64 = 365.256 * 86400.0;

/// Relative tolerance between an explicit override and the derived value.
pub const OVERRIDE_TOLERANCE: f64 = 0.01;

/// Instellation S = S☉ · L / a² \[W m⁻²\] at semi-major axis `a` \[AU\].
pub fn instellation(star: &Star, a: f64) -> f64 {
    SOLAR_CONSTANT * star.luminosity / (a * a)
}

/// Orbital period P = 365.256 d · √(a³ / M) \[s\] (Kepler's third law, planet mass
/// neglected).
pub fn orbital_period(star: &Star, a: f64) -> f64 {
    YEAR_1AU * (a * a * a / star.mass).sqrt()
}

/// How the planet spins (SPEC.md §9.3.2 `[orbit] spin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spin {
    /// Rotation set independently by the planet's rotation period.
    Free,
    /// One rotation per orbit.
    Synchronous,
    /// p rotations per q orbits.
    Resonant { p: u32, q: u32 },
}

impl Spin {
    /// Sidereal rotation rate Ω \[s⁻¹\] implied by the spin state for orbital period
    /// `period` \[s\]; None for free rotation.
    pub fn rotation_rate(self, period: f64) -> Option<f64> {
        match self {
            Spin::Free => None,
            Spin::Synchronous => Some(TAU / period),
            Spin::Resonant { p, q } => Some(p as f64 / q as f64 * TAU / period),
        }
    }
}

/// Use `given` if present, after checking it against `derived` (relative difference at
/// most [`OVERRIDE_TOLERANCE`] unless `allow_inconsistent`); otherwise `derived`.
pub fn checked_override(
    what: &str,
    derived: f64,
    given: Option<f64>,
    allow_inconsistent: bool,
) -> Result<f64, String> {
    let Some(v) = given else {
        return Ok(derived);
    };
    let rel = (v - derived).abs() / derived.abs().max(f64::MIN_POSITIVE);
    if rel > OVERRIDE_TOLERANCE && !allow_inconsistent {
        return Err(format!(
            "{what} = {v} differs from the derived value {derived:.6} by {:.2}% \
             (> {:.0}%); fix it or set allow_inconsistent = true in that block",
            rel * 100.0,
            OVERRIDE_TOLERANCE * 100.0
        ));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: f64 = 86400.0;

    #[test]
    fn earth_derivations() {
        let s = Star::sun();
        assert!((instellation(&s, 1.0) / 1361.0 - 1.0).abs() < 1e-4);
        assert!((orbital_period(&s, 1.0) / (365.256 * DAY) - 1.0).abs() < 1e-4);
    }

    /// TRAPPIST-1e (Agol et al. 2021, Tables 6–7): a = 0.02925 AU, S = 0.646 S⊕,
    /// P = 6.0996 d. SPEC.md P0: a 6.1-day year and about 900 W m⁻² within 5%.
    #[test]
    fn trappist1e_derivations() {
        let s = Star::trappist1();
        let a = 0.02925;
        let p = orbital_period(&s, a) / DAY;
        let f = instellation(&s, a);
        assert!((p - 6.1).abs() < 0.05, "year {p} d");
        assert!(
            (p / 6.0996 - 1.0).abs() < 0.01,
            "year {p} d vs Agol 6.0996 d"
        );
        assert!((f / 900.0 - 1.0).abs() < 0.05, "instellation {f}");
        assert!(
            (f / (0.646 * 1361.0) - 1.0).abs() < 0.01,
            "instellation {f}"
        );
        // The rounded SPEC.md input a = 0.0293 AU gives the same to the stated accuracy.
        let p2 = orbital_period(&s, 0.0293) / DAY;
        assert!((p2 - 6.1).abs() < 0.05 && (instellation(&s, 0.0293) / 900.0 - 1.0).abs() < 0.05);
    }

    #[test]
    fn spin_rates() {
        let p = 10.0 * DAY;
        assert_eq!(Spin::Free.rotation_rate(p), None);
        assert!((Spin::Synchronous.rotation_rate(p).unwrap() - TAU / p).abs() < 1e-20);
        let r = Spin::Resonant { p: 3, q: 2 }.rotation_rate(p).unwrap();
        assert!((r - 1.5 * TAU / p).abs() < 1e-20);
    }

    #[test]
    fn overrides() {
        assert_eq!(checked_override("x", 100.0, None, false), Ok(100.0));
        assert_eq!(checked_override("x", 100.0, Some(100.9), false), Ok(100.9));
        assert!(checked_override("x", 100.0, Some(102.0), false).is_err());
        assert_eq!(checked_override("x", 100.0, Some(102.0), true), Ok(102.0));
    }
}
