//! Shallow-water test cases (SPEC.md §8, M2).
//!
//! - Williamson et al. (1992), J. Comput. Phys. 102, 211–224: tests 2, 5, 6.
//! - Galewsky, Scott & Polvani (2004), Tellus A 56, 429–440: barotropic instability.
//!
//! Every case builds ψ, χ and g·h on the grid analytically and converts them with the
//! spectral transform; ζ = ∇²ψ, δ = ∇²χ.

use super::{SwInitial, SwState};
use crate::config::InitialCondition;
use crate::planet::Planet;
use crate::spectral::ops::laplacian;
use crate::spectral::Transform;
use crate::Real;
use std::f64::consts::PI;

/// Error from building a case.
pub type CaseResult = Result<SwInitial, String>;

fn param(ic: &InitialCondition, key: &str, default: f64) -> Result<f64, String> {
    match ic.params.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_float()
            .or_else(|| v.as_integer().map(|i| i as f64))
            .ok_or_else(|| format!("initial_condition.{key} must be a number")),
    }
}

fn flag(ic: &InitialCondition, key: &str, default: bool) -> Result<bool, String> {
    match ic.params.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_bool()
            .ok_or_else(|| format!("initial_condition.{key} must be true/false")),
    }
}

/// Build the initial condition named in the config.
pub fn build(tr: &Transform, planet: &Planet, ic: &InitialCondition) -> CaseResult {
    match ic.case.as_str() {
        "williamson2" => Ok(williamson2(tr, planet, param(ic, "alpha", 0.0)?)),
        "williamson5" => Ok(williamson5(tr, planet)),
        "williamson6" | "rossby_haurwitz" => Ok(williamson6(tr, planet)),
        "galewsky" => Ok(galewsky(tr, planet, flag(ic, "perturbed", true)?)),
        "rest" => Ok(at_rest(tr, param(ic, "depth", 1.0e4)?, planet)),
        other => Err(format!(
            "unknown shallow-water case {other:?} (williamson2, williamson5, williamson6, galewsky, rest)"
        )),
    }
}

/// Evaluate f(λ, φ) on the grid.
fn on_grid(tr: &Transform, f: impl Fn(f64, f64) -> f64) -> Vec<Real> {
    let mut out = Vec::with_capacity(tr.ngrid());
    for &lat in &tr.grid.lat {
        for &lon in &tr.grid.lon {
            out.push(f(lon, lat) as Real);
        }
    }
    out
}

/// Assemble the spectral state from grid ψ, total φ = g h and surface geopotential.
fn assemble(tr: &Transform, psi: &[Real], phi_total: &[Real], phis: Vec<Real>) -> SwInitial {
    let lay = &tr.layout;
    let phibar = (tr.integrate(phi_total) / (4.0 * PI * (tr.radius as f64).powi(2))) as Real;
    let dev: Vec<Real> = phi_total.iter().map(|p| p - phibar).collect();
    let vor = laplacian(lay, &tr.analysis(psi), tr.radius);
    SwInitial {
        state: SwState {
            vor,
            div: lay.zeros(),
            phi: tr.analysis(&dev),
        },
        phibar,
        phis_grid: phis,
        coriolis: None,
    }
}

/// Fluid at rest with uniform depth.
pub fn at_rest(tr: &Transform, depth: f64, planet: &Planet) -> SwInitial {
    let n = tr.ngrid();
    assemble(
        tr,
        &vec![0.0; n],
        &vec![(planet.gravity * depth) as Real; n],
        vec![0.0; n],
    )
}

/// Williamson test 2 constants: u₀ = 2πa/12 days, g h₀ = 2.94e4 m² s⁻².
pub fn williamson2_u0(a: f64) -> f64 {
    2.0 * PI * a / (12.0 * 86400.0)
}

/// Analytic g h for Williamson test 2 (steady state, also the exact solution).
pub fn williamson2_gh(planet: &Planet, alpha: f64, lon: f64, lat: f64) -> f64 {
    let a = planet.radius;
    let u0 = williamson2_u0(a);
    let gh0 = 2.94e4;
    let s = -lon.cos() * lat.cos() * alpha.sin() + lat.sin() * alpha.cos();
    gh0 - (a * planet.rotation_rate * u0 + 0.5 * u0 * u0) * s * s
}

/// Williamson test 2: steady geostrophic flow, rotated by `alpha`.
pub fn williamson2(tr: &Transform, planet: &Planet, alpha: f64) -> SwInitial {
    let a = planet.radius;
    let u0 = williamson2_u0(a);
    let psi = on_grid(tr, |lon, lat| {
        -a * u0 * (lat.sin() * alpha.cos() - lon.cos() * lat.cos() * alpha.sin())
    });
    let gh = on_grid(tr, |lon, lat| williamson2_gh(planet, alpha, lon, lat));
    let mut init = assemble(tr, &psi, &gh, vec![0.0; tr.ngrid()]);
    // Williamson et al. (1992), test case 2: the Coriolis parameter is rotated with the flow,
    // f = 2Ω(−cos λ cos φ sin α + sin φ cos α), so the state is steady for any α.
    let om = planet.rotation_rate;
    init.coriolis = Some(on_grid(tr, |lon, lat| {
        2.0 * om * (-lon.cos() * lat.cos() * alpha.sin() + lat.sin() * alpha.cos())
    }));
    init
}

/// Williamson test 5 mountain height h_s(λ, φ) \[m\]: 2000 m · (1 − r/R), R = π/9,
/// centre (3π/2, π/6).
pub fn williamson5_hs(lon: f64, lat: f64) -> f64 {
    let (lc, pc, r0) = (1.5 * PI, PI / 6.0, PI / 9.0);
    let r = ((lon - lc).powi(2) + (lat - pc).powi(2)).sqrt().min(r0);
    2000.0 * (1.0 - r / r0)
}

/// Williamson test 5: zonal flow over an isolated mountain. u₀ = 20 m/s, h₀ = 5960 m.
pub fn williamson5(tr: &Transform, planet: &Planet) -> SwInitial {
    let a = planet.radius;
    let g = planet.gravity;
    let (u0, h0) = (20.0, 5960.0);
    let psi = on_grid(tr, |_, lat| -a * u0 * lat.sin());
    let hs = on_grid(tr, williamson5_hs);
    let gh = on_grid(tr, |lon, lat| {
        g * h0
            - (a * planet.rotation_rate * u0 + 0.5 * u0 * u0) * lat.sin().powi(2)
            - g * williamson5_hs(lon, lat)
    });
    let phis = hs.iter().map(|h| h * g as Real).collect();
    assemble(tr, &psi, &gh, phis)
}

/// Williamson test 6 constants.
pub const RH_OMEGA: f64 = 7.848e-6;
pub const RH_K: f64 = 7.848e-6;
pub const RH_R: i32 = 4;
pub const RH_H0: f64 = 8000.0;

/// Analytic g h for the Rossby–Haurwitz wave at t = 0.
pub fn williamson6_gh(planet: &Planet, lon: f64, lat: f64) -> f64 {
    let a = planet.radius;
    let om = planet.rotation_rate;
    let (w, k, r) = (RH_OMEGA, RH_K, RH_R as f64);
    let c = lat.cos();
    let ri = RH_R;
    let fa = 0.5 * w * (2.0 * om + w) * c * c
        + 0.25
            * k
            * k
            * c.powi(2 * ri)
            * ((r + 1.0) * c * c + (2.0 * r * r - r - 2.0) - 2.0 * r * r / (c * c));
    let fb = 2.0 * (om + w) * k / ((r + 1.0) * (r + 2.0))
        * c.powi(ri)
        * ((r * r + 2.0 * r + 2.0) - (r + 1.0).powi(2) * c * c);
    let fc = 0.25 * k * k * c.powi(2 * ri) * ((r + 1.0) * c * c - (r + 2.0));
    planet.gravity * RH_H0 + a * a * (fa + fb * (r * lon).cos() + fc * (2.0 * r * lon).cos())
}

/// Williamson test 6: Rossby–Haurwitz wave, R = 4.
pub fn williamson6(tr: &Transform, planet: &Planet) -> SwInitial {
    let a = planet.radius;
    let psi = on_grid(tr, |lon, lat| {
        -a * a * RH_OMEGA * lat.sin()
            + a * a * RH_K * lat.cos().powi(RH_R) * lat.sin() * (RH_R as f64 * lon).cos()
    });
    let gh = on_grid(tr, |lon, lat| williamson6_gh(planet, lon, lat));
    assemble(tr, &psi, &gh, vec![0.0; tr.ngrid()])
}

/// Analytic nondivergent phase speed of the RH wave \[rad/s, eastward\].
pub fn rossby_haurwitz_speed(planet: &Planet) -> f64 {
    let r = RH_R as f64;
    (r * (3.0 + r) * RH_OMEGA - 2.0 * planet.rotation_rate) / ((1.0 + r) * (2.0 + r))
}

/// Galewsky jet u(φ) \[m/s\].
pub fn galewsky_u(lat: f64) -> f64 {
    let umax = 80.0;
    let p0 = PI / 7.0;
    let p1 = PI / 2.0 - p0;
    if lat <= p0 || lat >= p1 {
        return 0.0;
    }
    let en = (-4.0 / (p1 - p0).powi(2)).exp();
    umax / en * (1.0 / ((lat - p0) * (lat - p1))).exp()
}

/// ∫_{−π/2}^{φ} a u (f + u tan φ / a) dφ' by composite Simpson (the integrand vanishes
/// outside the jet).
fn galewsky_balance_integral(planet: &Planet, lat: f64) -> f64 {
    let p0 = PI / 7.0;
    let p1 = PI / 2.0 - p0;
    if lat <= p0 {
        return 0.0;
    }
    let top = lat.min(p1);
    let a = planet.radius;
    let f = |p: f64| {
        let u = galewsky_u(p);
        a * u * (2.0 * planet.rotation_rate * p.sin() + u * p.tan() / a)
    };
    let n = 20000; // even
    let h = (top - p0) / n as f64;
    let mut s = f(p0) + f(top);
    for i in 1..n {
        let x = p0 + i as f64 * h;
        s += if i % 2 == 1 { 4.0 } else { 2.0 } * f(x);
    }
    s * h / 3.0
}

/// Galewsky et al. (2004) barotropically unstable jet; mean depth 10 km. With
/// `perturbed`, adds h' = 120 m cos φ exp(−(λ/α)²) exp(−((φ₂−φ)/β)²).
pub fn galewsky(tr: &Transform, planet: &Planet, perturbed: bool) -> SwInitial {
    let g = planet.gravity;
    // Zonal flow: U = u cos φ, V = 0 → ζ from the curl; δ = 0 analytically.
    let u_grid = on_grid(tr, |_, lat| galewsky_u(lat) * lat.cos());
    let zeros = vec![0.0 as Real; tr.ngrid()];
    let vor = tr.curl(&u_grid, &zeros);
    // Balanced height: gh = gh0 − I(φ), with gh0 such that the mean depth is 10 km.
    let integ: Vec<f64> = tr
        .grid
        .lat
        .iter()
        .map(|&lat| galewsky_balance_integral(planet, lat))
        .collect();
    let mean_integ: f64 = integ
        .iter()
        .zip(&tr.grid.weights)
        .map(|(i, w)| i * w)
        .sum::<f64>()
        / 2.0;
    let gh0 = g * 1.0e4 + mean_integ;
    let (alpha, beta, phi2) = (1.0 / 3.0, 1.0 / 15.0, PI / 4.0);
    let nlon = tr.nlon();
    let mut gh = vec![0.0 as Real; tr.ngrid()];
    for (j, &lat) in tr.grid.lat.iter().enumerate() {
        for (k, &lon) in tr.grid.lon.iter().enumerate() {
            let mut v = gh0 - integ[j];
            if perturbed {
                let l = if lon > PI { lon - 2.0 * PI } else { lon };
                v += g
                    * 120.0
                    * lat.cos()
                    * (-(l / alpha).powi(2)).exp()
                    * (-((phi2 - lat) / beta).powi(2)).exp();
            }
            gh[j * nlon + k] = v as Real;
        }
    }
    let phibar = (tr.integrate(&gh) / (4.0 * PI * (tr.radius as f64).powi(2))) as Real;
    let dev: Vec<Real> = gh.iter().map(|p| p - phibar).collect();
    SwInitial {
        state: SwState {
            vor,
            div: tr.layout.zeros(),
            phi: tr.analysis(&dev),
        },
        phibar,
        phis_grid: zeros,
        coriolis: None,
    }
}

/// Williamson et al. (1992) normalized error norms (l1, l2, l∞) of `x` against `truth`.
pub fn error_norms(tr: &Transform, x: &[Real], truth: &[Real]) -> (f64, f64, f64) {
    let diff: Vec<Real> = x.iter().zip(truth).map(|(a, b)| a - b).collect();
    let abs = |v: &[Real]| v.iter().map(|x| x.abs()).collect::<Vec<_>>();
    let sq = |v: &[Real]| v.iter().map(|x| x * x).collect::<Vec<_>>();
    let l1 = tr.integrate(&abs(&diff)) / tr.integrate(&abs(truth));
    let l2 = (tr.integrate(&sq(&diff)) / tr.integrate(&sq(truth))).sqrt();
    let linf = crate::model::max_abs(&diff) / crate::model::max_abs(truth);
    (l1, l2, linf)
}
