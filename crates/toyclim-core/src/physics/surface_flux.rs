//! Surface exchange coefficients and near-surface diagnostics (SPEC.md §9.4.2).
//!
//! Bulk formulas at the lowest model level (height z_a):
//! τ = ρ C_D |V| v, H = ρ C_H |V| (s_s − s_a), E = β ρ C_H |V| (q_sat(T_s) − q_a), with
//! |V|² = u² + v² + w_g² and s the dry static energy (c_p T + Φ; the difference stands for
//! c_p times the potential-temperature difference).
//!
//! **Stability** (Louis 1979, Bound.-Layer Meteor. 17, 187–202; transcribed from the
//! paper): C = C_n F(Ri_B) with the bulk Richardson number Ri_B = g z Δθ_v / (θ_v |V|²)
//! (eq. 11; here with virtual dry static energy as in Frierson et al. 2006),
//! - unstable (eq. 14): F = 1 − b Ri / (1 + c |Ri|^{1/2}), c = C* a² b (z/z₀)^{1/2} (eq. 20),
//!   a² the neutral drag coefficient, b = 9.4, C* = 7.4 (momentum) and 5.3 (heat);
//! - stable (eq. 15): F = 1 / (1 + b′ Ri)², b′ = b/2 = 4.7 — never exactly zero, so the
//!   night-time surface stays coupled (the reason Louis gives for this form).
//!
//! Neutral coefficients C_n = κ² / [ln(z/z₀m) ln(z/z₀h)] with κ = 0.4 (SPEC.md §9.4.2;
//! Louis writes the heat coefficient as a²/R with R = 0.74 and a single z₀, which this
//! form replaces by the separate thermal roughness length).
//!
//! **Roughness**: ocean Charnock z₀m = max(1.5e-5 m, α u*² / g), α = 0.018, from the stored
//! u* of the previous step, z₀h = z₀m; sea ice and snow 1e-3 m; land 0.1 m (or a per-tile
//! value); z₀h = z₀m / 10 over land and ice.
//!
//! **2 m and 10 m diagnostics** interpolate between the surface and the lowest level with
//! the Businger et al. (1971) flux–profile functions in the form Louis (1979, eqs. 6–9)
//! writes them, ψ_m = ln[((1+x)/2)² (1+x²)/2] − 2 arctan x + π/2, x = (1 − γ_m ζ)^{1/4},
//! ψ_h = 2 ln[(1+y)/2], y = (1 − γ_h ζ)^{1/2} for ζ < 0; ψ_m = −β ζ, ψ_h = −β ζ / R for
//! ζ ≥ 0, with ζ = z / L from the Obukhov length of the computed fluxes. Louis does not
//! print γ_m, γ_h, β, R; the values are Businger et al. (1971): γ_m = 15, γ_h = 9,
//! β = 4.7, R = 0.74 (consistent with Louis' b′ = 4.7 and R = 0.74; Businger's paper not
//! fetched).

/// von Kármán constant (SPEC.md §9.4.2).
pub const KARMAN: f64 = 0.4;

/// Louis (1979) b, b′ and C* (momentum, heat).
const LOUIS_B: f64 = 9.4;
const LOUIS_BP: f64 = 4.7;
const CSTAR_M: f64 = 7.4;
const CSTAR_H: f64 = 5.3;

/// Businger et al. (1971) profile constants (see the module doc).
const GAMMA_M: f64 = 15.0;
const GAMMA_H: f64 = 9.0;
const BETA: f64 = 4.7;
const R_BUSINGER: f64 = 0.74;

/// Surface-exchange knobs (SPEC.md §9.4.2, listed in docs/tuning.md).
#[derive(Debug, Clone)]
pub struct ExchangeParams {
    /// Gustiness velocity w_g \[m s⁻¹\] (1).
    pub gustiness: f64,
    /// Charnock constant α (0.018).
    pub charnock: f64,
    /// Minimum ocean roughness \[m\] (1.5e-5).
    pub z0_ocean_min: f64,
    /// Default land roughness \[m\] (0.1).
    pub z0_land: f64,
    /// Sea-ice and snow roughness \[m\] (1e-3).
    pub z0_ice: f64,
    /// z₀h / z₀m over land and ice (0.1).
    pub z0h_ratio: f64,
    /// Gust = 10 m wind + `gust_unstable`·u* (unstable) or `gust_stable`·u* (3, 1).
    pub gust_unstable: f64,
    pub gust_stable: f64,
}

impl Default for ExchangeParams {
    fn default() -> Self {
        ExchangeParams {
            gustiness: 1.0,
            charnock: 0.018,
            z0_ocean_min: 1.5e-5,
            z0_land: 0.1,
            z0_ice: 1.0e-3,
            z0h_ratio: 0.1,
            gust_unstable: 3.0,
            gust_stable: 1.0,
        }
    }
}

/// Louis (1979) stability function F(Ri) for neutral coefficient `cn` (a²), height ratio
/// z/z₀ and the free-convection constant `cstar`.
pub fn louis(ri: f64, cn: f64, z_over_z0: f64, cstar: f64) -> f64 {
    if ri < 0.0 {
        let c = cstar * cn * LOUIS_B * z_over_z0.sqrt();
        1.0 - LOUIS_B * ri / (1.0 + c * (-ri).sqrt())
    } else {
        1.0 / (1.0 + LOUIS_BP * ri).powi(2)
    }
}

/// Drag and heat (= moisture) exchange coefficients at height `z` over roughness lengths
/// (z₀m, z₀h) for bulk Richardson number `ri`.
pub fn coefficients(z: f64, z0m: f64, z0h: f64, ri: f64) -> (f64, f64) {
    let lm = (z / z0m).ln();
    let lh = (z / z0h).ln();
    let cn_m = (KARMAN / lm).powi(2);
    let cn_h = KARMAN * KARMAN / (lm * lh);
    let zz = z / z0m;
    (
        cn_m * louis(ri, cn_m, zz, CSTAR_M),
        cn_h * louis(ri, cn_m, zz, CSTAR_H),
    )
}

/// Roughness lengths (z₀m, z₀h) \[m\] of a tile.
pub fn roughness(
    p: &ExchangeParams,
    kind: crate::surface::TileKind,
    z0_tile: f64,
    ustar_prev: f64,
    gravity: f64,
) -> (f64, f64) {
    use crate::surface::TileKind;
    match kind {
        TileKind::Ocean => {
            let z0 = (p.charnock * ustar_prev * ustar_prev / gravity).max(p.z0_ocean_min);
            (z0, z0)
        }
        TileKind::SeaIce => (p.z0_ice, p.z0_ice * p.z0h_ratio),
        TileKind::Land => {
            let z0 = if z0_tile > 0.0 { z0_tile } else { p.z0_land };
            (z0, z0 * p.z0h_ratio)
        }
    }
}

/// Businger–Dyer integrated stability functions ψ_m(ζ), ψ_h(ζ).
fn psi_m(zeta: f64) -> f64 {
    if zeta < 0.0 {
        let x = (1.0 - GAMMA_M * zeta).powf(0.25);
        (((1.0 + x) / 2.0).powi(2) * (1.0 + x * x) / 2.0).ln() - 2.0 * x.atan()
            + std::f64::consts::FRAC_PI_2
    } else {
        -BETA * zeta
    }
}

fn psi_h(zeta: f64) -> f64 {
    if zeta < 0.0 {
        let y = (1.0 - GAMMA_H * zeta).sqrt();
        2.0 * ((1.0 + y) / 2.0).ln()
    } else {
        -BETA * zeta / R_BUSINGER
    }
}

/// Fraction of the surface-to-`za` difference reached at height `z` (≤ z_a) for roughness
/// `z0` and inverse Obukhov length `inv_l`: [ln(z/z₀) − ψ(z/L) + ψ(z₀/L)] / [same at z_a].
pub fn profile_fraction(z: f64, za: f64, z0: f64, inv_l: f64, heat: bool) -> f64 {
    if z >= za {
        return 1.0;
    }
    let psi = |zeta: f64| if heat { psi_h(zeta) } else { psi_m(zeta) };
    let f = |zz: f64| (zz / z0).ln() - psi(zz * inv_l) + psi(z0 * inv_l);
    let (fz, fa) = (f(z), f(za));
    if fa > 0.0 && fz > 0.0 {
        (fz / fa).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Inverse Obukhov length 1/L = −κ g (w′θ_v′) / (θ_v u*³) \[m⁻¹\] from the kinematic
/// virtual heat flux `w_thv` \[K m s⁻¹\].
pub fn inverse_obukhov(ustar: f64, w_thv: f64, theta_v: f64, gravity: f64) -> f64 {
    let us = ustar.max(1e-3);
    -KARMAN * gravity * w_thv / (theta_v * us * us * us)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC.md P1 acceptance 4: neutral 10 m drag coefficient over the ocean at 10 m/s is
    /// 1.0e-3 to 1.6e-3 (Charnock roughness iterated to its fixed point).
    #[test]
    fn neutral_ocean_drag_at_10_m_per_s() {
        let p = ExchangeParams::default();
        let (z, u) = (10.0, 10.0);
        let mut ustar = 0.0;
        let mut cd = 0.0;
        for _ in 0..50 {
            let (z0m, z0h) = roughness(&p, crate::surface::TileKind::Ocean, 0.0, ustar, 9.81);
            cd = coefficients(z, z0m, z0h, 0.0).0;
            ustar = cd.sqrt() * u;
        }
        eprintln!("C_D(10 m, 10 m/s, neutral) = {cd:.3e}, u* = {ustar:.3} m/s");
        assert!((1.0e-3..=1.6e-3).contains(&cd), "C_D = {cd}");
    }

    /// F and dF/dRi are continuous at Ri = 0 (b = 2b′), F decreases monotonically with Ri
    /// and stays positive in stable conditions.
    #[test]
    fn louis_function_shape() {
        let (cn, zz) = (1.2e-3, 1e5);
        for cs in [CSTAR_M, CSTAR_H] {
            assert_eq!(louis(0.0, cn, zz, cs), 1.0);
            let e = 1e-12;
            let dl = (louis(0.0, cn, zz, cs) - louis(-e, cn, zz, cs)) / e;
            let dr = (louis(e, cn, zz, cs) - louis(0.0, cn, zz, cs)) / e;
            assert!((dl - dr).abs() < 1e-3 * dl.abs(), "{dl} {dr}");
            let mut prev = f64::INFINITY;
            for i in -50..=200 {
                let f = louis(i as f64 * 0.05, cn, zz, cs);
                assert!(f < prev && f > 0.0);
                prev = f;
            }
        }
        // unstable enhancement
        assert!(louis(-1.0, cn, zz, CSTAR_H) > 1.3);
    }

    #[test]
    fn profile_fraction_limits() {
        // neutral: logarithmic
        let r = profile_fraction(2.0, 60.0, 0.01, 0.0, true);
        assert!((r - (200f64).ln() / 6000f64.ln()).abs() < 1e-12);
        // very stable: closer to the surface value than neutral; unstable: further
        assert!(profile_fraction(2.0, 60.0, 0.01, 0.1, true) < r);
        assert!(profile_fraction(2.0, 60.0, 0.01, -0.1, true) > r);
        assert_eq!(profile_fraction(70.0, 60.0, 0.01, 0.0, false), 1.0);
    }
}
