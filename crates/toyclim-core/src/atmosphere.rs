//! Dry-air composition → molar mass, specific gas constant R_d and heat capacity c_p
//! (SPEC.md §9.3.2 `[atmosphere]`).
//!
//! Ideal-gas mixture: M = Σ x_i M_i, R_d = R*/M, c_p = Σ w_i c_p,i with mass fractions
//! w_i = x_i M_i / M. Constants:
//! - R* = 8.314462618 J mol⁻¹ K⁻¹ (CODATA 2018, exact);
//! - molar masses \[g mol⁻¹\]: IUPAC standard atomic weights (N 14.0067, O 15.9994,
//!   Ar 39.948, C 12.0107, H 1.00794);
//! - isobaric molar heat capacity C_p° at 300 K: NIST-JANAF Thermochemical Tables
//!   (Chase 1998; janaf.nist.gov tables N-023, O-029, Ar-001, C-095, checked
//!   2026-11-15): N₂ 29.125, O₂ 29.385, Ar 20.786, CO₂ 37.221 J mol⁻¹ K⁻¹.

use std::collections::BTreeMap;

/// Universal gas constant R* \[J mol⁻¹ K⁻¹\].
pub const R_UNIVERSAL: f64 = 8.314462618;

/// A gas: molar mass \[kg mol⁻¹\] and isobaric molar heat capacity \[J mol⁻¹ K⁻¹\].
#[derive(Debug, Clone, Copy)]
pub struct Gas {
    pub name: &'static str,
    pub molar_mass: f64,
    pub cp_molar: f64,
}

/// Known gases.
pub const GASES: &[Gas] = &[
    Gas {
        name: "N2",
        molar_mass: 28.0134e-3,
        cp_molar: 29.125,
    },
    Gas {
        name: "O2",
        molar_mass: 31.9988e-3,
        cp_molar: 29.385,
    },
    Gas {
        name: "Ar",
        molar_mass: 39.948e-3,
        cp_molar: 20.786,
    },
    Gas {
        name: "CO2",
        molar_mass: 44.0095e-3,
        cp_molar: 37.221,
    },
];

/// Look up a gas by name (case-sensitive chemical formula).
pub fn gas(name: &str) -> Option<&'static Gas> {
    GASES.iter().find(|g| g.name == name)
}

/// Thermodynamic constants of a dry-air mixture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixture {
    /// Mean molar mass \[kg mol⁻¹\].
    pub molar_mass: f64,
    /// R_d \[J kg⁻¹ K⁻¹\].
    pub gas_constant: f64,
    /// c_p \[J kg⁻¹ K⁻¹\].
    pub cp: f64,
}

/// Mixture constants for volume (mole) fractions `x` by gas name. The fractions must sum
/// to 1 within 1e-3.
pub fn mixture(x: &BTreeMap<String, f64>) -> Result<Mixture, String> {
    if x.is_empty() {
        return Err("atmosphere composition is empty".into());
    }
    let mut sum = 0.0;
    let mut m = 0.0;
    let mut cpm = 0.0; // Σ x_i c_p,i (molar)
    for (name, &xi) in x {
        let g = gas(name).ok_or_else(|| {
            let known: Vec<_> = GASES.iter().map(|g| g.name).collect();
            format!("unknown gas {name:?} in composition (known: {known:?})")
        })?;
        if !(xi.is_finite() && xi >= 0.0) {
            return Err(format!("volume fraction of {name} must be >= 0"));
        }
        sum += xi;
        m += xi * g.molar_mass;
        cpm += xi * g.cp_molar;
    }
    if (sum - 1.0).abs() > 1e-3 {
        return Err(format!("volume fractions sum to {sum}, not 1"));
    }
    let (m, cpm) = (m / sum, cpm / sum);
    Ok(Mixture {
        molar_mass: m,
        gas_constant: R_UNIVERSAL / m,
        // Σ w_i c_p,i = Σ (x_i M_i / M)(C_i / M_i) = (Σ x_i C_i) / M
        cp: cpm / m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn earth_air() -> BTreeMap<String, f64> {
        [
            ("N2", 0.7808),
            ("O2", 0.2095),
            ("Ar", 0.0093),
            ("CO2", 400e-6),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
    }

    #[test]
    fn earth_air_matches_standard_values() {
        let m = mixture(&earth_air()).unwrap();
        // Standard dry air: M = 28.965 g/mol, R_d = 287.05, c_p ≈ 1004–1005 J/(kg K).
        assert!((m.molar_mass * 1e3 - 28.965).abs() < 0.01, "{m:?}");
        assert!((m.gas_constant - 287.04).abs() < 0.1, "{m:?}");
        assert!((m.cp - 1004.6).abs() < 2.0, "{m:?}");
    }

    #[test]
    fn errors() {
        let mut x = earth_air();
        x.insert("Xe".into(), 0.0);
        assert!(mixture(&x).is_err());
        let mut y = earth_air();
        y.insert("N2".into(), 0.5);
        assert!(mixture(&y).is_err());
    }
}
