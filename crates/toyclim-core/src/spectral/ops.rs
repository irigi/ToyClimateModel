//! Spectral-space operators (SPEC.md §4.4–4.5).

use super::packing::{Complex, Layout};
use crate::Real;

/// ∇²: multiply by −ℓ(ℓ+1)/a².
pub fn laplacian(lay: &Layout, f: &[Complex], radius: Real) -> Vec<Complex> {
    let ia2 = 1.0 / (radius * radius);
    f.iter()
        .zip(&lay.l_of)
        .map(|(c, &l)| c * (-((l * (l + 1)) as Real) * ia2))
        .collect()
}

/// ∇⁻²: divide by −ℓ(ℓ+1)/a²; the ℓ = 0 coefficient is set to zero.
pub fn inv_laplacian(lay: &Layout, f: &[Complex], radius: Real) -> Vec<Complex> {
    let a2 = radius * radius;
    f.iter()
        .zip(&lay.l_of)
        .map(|(c, &l)| {
            if l == 0 {
                Complex::new(0.0, 0.0)
            } else {
                c * (-a2 / ((l * (l + 1)) as Real))
            }
        })
        .collect()
}

/// Implicit hyperdiffusion factors 1 / (1 + 2Δt τ⁻¹ [ℓ(ℓ+1)/(M(M+1))]^p) per packed index,
/// with ℓ = 0 left untouched. `dt_eff` is the step over which diffusion acts (2Δt for
/// leapfrog), `tau` the e-folding time at ℓ = M \[s\].
pub fn hyperdiffusion_factors(lay: &Layout, order: u32, tau: f64, dt_eff: f64) -> Vec<Real> {
    let mm = lay.mmax as f64;
    let norm = mm * (mm + 1.0);
    lay.l_of
        .iter()
        .map(|&l| {
            if l == 0 || tau <= 0.0 {
                1.0
            } else {
                let x = (l as f64 * (l as f64 + 1.0) / norm).powi(order as i32);
                (1.0 / (1.0 + dt_eff / tau * x)) as Real
            }
        })
        .collect()
}

/// a += s·b
pub fn axpy(a: &mut [Complex], s: Real, b: &[Complex]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y * s;
    }
}

/// Set ψ to −a u₀ sin φ: the spectral coefficient of solid-body rotation with speed u₀ at the
/// equator. sin φ = μ = √(2/3) P̄_1^0.
pub fn solid_body_psi(lay: &Layout, u0: Real, radius: Real) -> Vec<Complex> {
    let mut psi = lay.zeros();
    psi[lay.idx(1, 0)] = Complex::new(-radius * u0 * (2.0 as Real / 3.0).sqrt(), 0.0);
    psi
}
