//! Slab (mixed-layer) ocean: heat capacity and the prescribed ocean heat transport.

use super::Surface;
use crate::spectral::Transform;
use crate::Real;

/// Heat capacity of a mixed layer of depth `h` \[m\]: ρ_w c_w h with ρ_w = 1000 kg m⁻³,
/// c_w = 4000 J kg⁻¹ K⁻¹ \[J m⁻² K⁻¹\].
pub fn heat_capacity(h: f64) -> f64 {
    1000.0 * 4000.0 * h
}

/// Prescribed ocean heat-flux convergence Q(φ) = Q0 (1 − 2φ²/φ0²) exp(−φ²/φ0²) / cos φ
/// over the ocean with φ0 = 16° (Merlis et al. 2013 form), re-centred so that its area
/// mean over the ocean is exactly zero.
pub fn set_qflux(s: &mut Surface, tr: &Transform, q0: f64) {
    let n = tr.ngrid();
    let phi0 = 16f64.to_radians();
    let nlon = tr.nlon();
    for j in 0..tr.nlat() {
        let lat = tr.grid.lat[j];
        let x = lat * lat / (phi0 * phi0);
        let q = q0 * (1.0 - 2.0 * x) * (-x).exp() / lat.cos();
        for i in 0..nlon {
            let p = j * nlon + i;
            if !s.is_land(p) {
                s.qflux[p] = q as Real;
            }
        }
    }
    let ocean: Vec<Real> = (0..n)
        .map(|p| if s.is_land(p) { 0.0 } else { 1.0 })
        .collect();
    let area = tr.integrate(&ocean);
    if area > 0.0 {
        let mean = tr.integrate(&s.qflux) / area;
        for p in 0..n {
            if !s.is_land(p) {
                s.qflux[p] -= mean as Real;
            }
        }
    }
}
