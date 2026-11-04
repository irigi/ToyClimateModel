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

/// Bilinear interpolation weights for one point on a Gaussian grid: linear in latitude
/// between neighbouring rows (clamped to the outermost rows near the poles) and periodic
/// linear in longitude.
#[derive(Debug, Clone, Copy)]
pub struct PointSampler {
    j0: usize,
    j1: usize,
    wj: f64,
    k0: usize,
    k1: usize,
    wk: f64,
}

impl PointSampler {
    /// Sampler for latitude/longitude in radians.
    pub fn new(grid: &GaussianGrid, lat: f64, lon: f64) -> Self {
        let n = grid.nlat;
        let (j0, j1, wj) = if lat >= grid.lat[0] {
            (0, 0, 0.0)
        } else if lat <= grid.lat[n - 1] {
            (n - 1, n - 1, 0.0)
        } else {
            // lat[j] ≥ lat > lat[j+1]
            let j = grid.lat.partition_point(|&x| x >= lat) - 1;
            let w = (grid.lat[j] - lat) / (grid.lat[j] - grid.lat[j + 1]);
            (j, j + 1, w)
        };
        let dl = 2.0 * std::f64::consts::PI / grid.nlon as f64;
        let x = lon.rem_euclid(2.0 * std::f64::consts::PI) / dl;
        let k0 = (x.floor() as usize) % grid.nlon;
        PointSampler {
            j0,
            j1,
            wj,
            k0,
            k1: (k0 + 1) % grid.nlon,
            wk: x - x.floor(),
        }
    }

    /// Interpolated value of a `[lat][lon]` field with `nlon` longitudes.
    #[inline]
    pub fn sample(&self, field: &[crate::Real], nlon: usize) -> f64 {
        let at = |j: usize, k: usize| field[j * nlon + k] as f64;
        let r0 = at(self.j0, self.k0) * (1.0 - self.wk) + at(self.j0, self.k1) * self.wk;
        let r1 = at(self.j1, self.k0) * (1.0 - self.wk) + at(self.j1, self.k1) * self.wk;
        r0 * (1.0 - self.wj) + r1 * self.wj
    }
}

/// A regular equirectangular target grid: `height` rows from 90°N to 90°S (cell centres,
/// linear in latitude) and `width` columns from 0°E eastward.
#[derive(Debug, Clone)]
pub struct LatLonRegridder {
    pub width: usize,
    pub height: usize,
    nlon_src: usize,
    samplers: Vec<PointSampler>,
}

impl LatLonRegridder {
    pub fn new(grid: &GaussianGrid, width: usize, height: usize) -> Self {
        let mut samplers = Vec::with_capacity(width * height);
        for r in 0..height {
            let lat = std::f64::consts::FRAC_PI_2
                - std::f64::consts::PI * (r as f64 + 0.5) / height as f64;
            for c in 0..width {
                let lon = 2.0 * std::f64::consts::PI * (c as f64 + 0.5) / width as f64;
                samplers.push(PointSampler::new(grid, lat, lon));
            }
        }
        LatLonRegridder {
            width,
            height,
            nlon_src: grid.nlon,
            samplers,
        }
    }

    /// The SPEC §7 default: 2·N_lon × N_lon.
    pub fn default_for(grid: &GaussianGrid) -> Self {
        Self::new(grid, 2 * grid.nlon, grid.nlon)
    }

    /// Regrid one `[lat][lon]` field into a row-major `[height][width]` f32 image.
    pub fn regrid(&self, field: &[crate::Real]) -> Vec<f32> {
        self.samplers
            .iter()
            .map(|s| s.sample(field, self.nlon_src) as f32)
            .collect()
    }
}

#[cfg(test)]
mod sampler_tests {
    use super::*;

    #[test]
    fn bilinear_reproduces_linear_fields_and_nodes() {
        let g = GaussianGrid::new(32, 64);
        // field = latitude in radians (linear in lat → exact between rows)
        let mut f: Vec<crate::Real> = Vec::new();
        for j in 0..g.nlat {
            for _ in 0..g.nlon {
                f.push(g.lat[j] as crate::Real);
            }
        }
        let tol = if std::mem::size_of::<crate::Real>() == 4 {
            1e-6
        } else {
            1e-12
        };
        for lat in [-1.2, -0.3, 0.0, 0.77, 1.3] {
            let s = PointSampler::new(&g, lat, 1.0);
            assert!((s.sample(&f, g.nlon) - lat).abs() < tol);
        }
        // at a node, the node value
        let s = PointSampler::new(&g, g.lat[5], g.lon[7]);
        let mut h: Vec<crate::Real> = vec![0.0; g.len()];
        h[5 * g.nlon + 7] = 3.0;
        assert!((s.sample(&h, g.nlon) - 3.0).abs() < 1e-12);
        // periodic in longitude
        let s = PointSampler::new(&g, 0.1, 2.0 * std::f64::consts::PI - 1e-9);
        assert!(s.k1 == 0);
    }
}
