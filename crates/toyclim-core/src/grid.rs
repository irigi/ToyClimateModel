//! Gaussian grid description and area weights.

use crate::spectral::gauss::gauss_legendre;
use serde::Serialize;

/// Alias-free Gaussian grid size (N_lat, N_lon) for triangular truncation T`m`.
///
/// Uses the SPEC.md §4.1 table where available, otherwise N_lat ≥ (3M+1)/2 and
/// N_lon ≥ 3M+1 rounded up to the next even N_lat whose N_lon = 2·N_lat has only small
/// prime factors (2, 3, 5).
pub fn grid_size_for_truncation(m: usize) -> (usize, usize) {
    match m {
        21 => (32, 64),
        31 => (48, 96),
        42 => (64, 128),
        63 => (96, 192),
        85 => (128, 256),
        127 => (192, 384),
        170 => (256, 512),
        _ => {
            let mut nlat = (3 * m + 1).div_ceil(2).max(2);
            loop {
                let nlon = 2 * nlat;
                if nlat.is_multiple_of(2) && nlon > 3 * m && is_smooth(nlon) {
                    return (nlat, nlon);
                }
                nlat += 1;
            }
        }
    }
}

fn is_smooth(mut n: usize) -> bool {
    for p in [2, 3, 5] {
        while n.is_multiple_of(p) {
            n /= p;
        }
    }
    n == 1
}

/// A Gaussian grid. Latitude index 0 is the northernmost row; longitudes start at 0
/// and increase eastward.
#[derive(Debug, Clone, Serialize)]
pub struct GaussianGrid {
    pub nlat: usize,
    pub nlon: usize,
    /// μ_j = sin φ_j, north to south.
    pub mu: Vec<f64>,
    /// Gaussian weights w_j (Σ w_j = 2).
    pub weights: Vec<f64>,
    /// Latitudes φ_j \[rad\].
    pub lat: Vec<f64>,
    /// Longitudes λ_k = 2πk/N_lon \[rad\].
    pub lon: Vec<f64>,
}

impl GaussianGrid {
    /// Build a grid with `nlat` Gaussian latitudes and `nlon` equally spaced longitudes.
    pub fn new(nlat: usize, nlon: usize) -> Self {
        let (mu, weights) = gauss_legendre(nlat);
        let lat = mu.iter().map(|m| m.asin()).collect();
        let lon = (0..nlon)
            .map(|k| 2.0 * std::f64::consts::PI * k as f64 / nlon as f64)
            .collect();
        GaussianGrid {
            nlat,
            nlon,
            mu,
            weights,
            lat,
            lon,
        }
    }

    /// The alias-free grid for truncation T`m`.
    pub fn for_truncation(m: usize) -> Self {
        let (nlat, nlon) = grid_size_for_truncation(m);
        Self::new(nlat, nlon)
    }

    /// Number of grid points.
    pub fn len(&self) -> usize {
        self.nlat * self.nlon
    }

    /// True for an empty grid (never for a constructed grid).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Global integral ∫X dA = 2π a² Σ_j w_j · mean_k X(λ_k, μ_j) of a row-major field
    /// \[unit(X)·m²\]. Summed serially in a fixed order (deterministic).
    pub fn integrate(&self, field: &[f64], radius: f64) -> f64 {
        assert_eq!(field.len(), self.len());
        let mut total = 0.0;
        for j in 0..self.nlat {
            let row = &field[j * self.nlon..(j + 1) * self.nlon];
            let mean: f64 = row.iter().sum::<f64>() / self.nlon as f64;
            total += self.weights[j] * mean;
        }
        2.0 * std::f64::consts::PI * radius * radius * total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_sizes_are_alias_free() {
        for m in [21, 31, 42, 63, 85, 127, 170, 10, 50, 106] {
            let (nlat, nlon) = grid_size_for_truncation(m);
            assert!(2 * nlat > 3 * m, "T{m}");
            assert!(nlon > 3 * m, "T{m}");
        }
    }

    #[test]
    fn area_of_sphere() {
        let g = GaussianGrid::for_truncation(42);
        let ones = vec![1.0; g.len()];
        let a = 6.37122e6;
        let area = g.integrate(&ones, a);
        let exact = 4.0 * std::f64::consts::PI * a * a;
        assert!((area / exact - 1.0).abs() < 1e-14);
    }
}
