//! Geography from a heightmap: land fraction and model orography on the Gaussian grid.

use crate::spectral::Transform;
use crate::Real;

/// Land fraction and filtered surface geopotential of a heightmap.
pub struct Geography {
    /// Land fraction per Gaussian cell (fraction of heightmap pixels above 0 m).
    pub land_fraction: Vec<f64>,
    /// Spectrally filtered surface geopotential Φ_s \[m² s⁻²\] on the grid.
    pub phis: Vec<Real>,
}

/// Box-average the heightmap `h` (`[lat][lon]`, row 0 north, metres; ≤ 0 is ocean) onto
/// the Gaussian cells: land fraction, and elevation as the mean of max(h, 0). The
/// geopotential g·z is filtered in spectral space against Gibbs ringing with
/// exp(−c (ℓ(ℓ+1)/(M(M+1)))²), c chosen so the factor at ℓ = M is `filter_at_m`.
pub fn from_heightmap(
    tr: &Transform,
    gravity: f64,
    h: &[f32],
    nlat_h: usize,
    nlon_h: usize,
    filter_at_m: f64,
) -> Geography {
    let (_, frac) = tr.grid.box_average(h, nlat_h, nlon_h, |v| v > 0.0);
    let hpos: Vec<f32> = h.iter().map(|&v| v.max(0.0)).collect();
    let (elev, _) = tr.grid.box_average(&hpos, nlat_h, nlon_h, |_| false);
    let phis_raw: Vec<Real> = elev.iter().map(|&z| (gravity * z) as Real).collect();
    let mut spec = tr.analysis(&phis_raw);
    let mm = (tr.mmax() * (tr.mmax() + 1)) as f64;
    let c = -filter_at_m.clamp(1e-6, 1.0).ln();
    for (x, &l) in spec.iter_mut().zip(&tr.layout.l_of) {
        *x *= (-c * ((l * (l + 1)) as f64 / mm).powi(2)).exp() as Real;
    }
    Geography {
        land_fraction: frac,
        phis: tr.synthesis(&spec),
    }
}
