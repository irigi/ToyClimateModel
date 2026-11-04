//! Initial conditions for the primitive-equation model (SPEC.md §8, M4–M5).

use super::vertical::SigmaLevels;
use super::{PeInitial, PeState};
use crate::config::InitialCondition;
use crate::planet::Planet;
use crate::spectral::{Complex, Transform};
use crate::util::SplitMix64;
use crate::Real;
use std::f64::consts::PI;

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
pub fn build(
    tr: &Transform,
    planet: &Planet,
    lev: &SigmaLevels,
    ic: &InitialCondition,
) -> Result<PeInitial, String> {
    match ic.case.as_str() {
        "rest" | "isothermal" => Ok(isothermal(
            tr,
            planet,
            lev,
            param(ic, "temperature", 250.0)?,
            param(ic, "ps", planet.p0)?,
            param(ic, "noise", 0.0)?,
            param(ic, "seed", 1.0)? as u64,
        )),
        "gravity_wave" => gravity_wave(
            tr,
            planet,
            lev,
            param(ic, "mode", 0.0)? as usize,
            param(ic, "degree", 3.0)? as usize,
            param(ic, "amplitude", 0.01)?,
        ),
        "jw06" | "jablonowski_williamson" => {
            Ok(jw06(tr, planet, lev, flag(ic, "perturbed", true)?))
        }
        other => Err(format!(
            "unknown primitive-equation case {other:?} (rest, gravity_wave, jw06)"
        )),
    }
}

/// Spectral coefficient of a constant field c: c·√2 at (0,0).
fn constant(tr: &Transform, c: f64) -> Vec<Complex> {
    let mut v = tr.spec_zeros();
    v[0] = Complex::new((c * std::f64::consts::SQRT_2) as Real, 0.0);
    v
}

/// Isothermal atmosphere at rest with uniform surface pressure; optional random
/// temperature noise (K, standard amplitude) on wavenumbers 1 ≤ ℓ ≤ min(M, 21).
pub fn isothermal(
    tr: &Transform,
    planet: &Planet,
    lev: &SigmaLevels,
    temp: f64,
    ps: f64,
    noise: f64,
    seed: u64,
) -> PeInitial {
    let l = lev.nlev;
    let mut s = PeState::zeros(l, tr.layout.len());
    let mut rng = SplitMix64(seed);
    for k in 0..l {
        s.tmp[k] = constant(tr, temp - planet.t_ref);
        if noise > 0.0 {
            for idx in 0..tr.layout.len() {
                let ell = tr.layout.l_of[idx];
                if (1..=21).contains(&ell) {
                    let im = if tr.layout.m_of[idx] == 0 {
                        0.0
                    } else {
                        rng.symmetric()
                    };
                    s.tmp[k][idx] +=
                        Complex::new((noise * rng.symmetric()) as Real, (noise * im) as Real);
                }
            }
        }
    }
    s.lnps = constant(tr, ps.ln());
    PeInitial {
        state: s,
        phis_grid: tr.grid_zeros(),
    }
}

/// Eigenvalues (ascending) and eigenvectors of B = Gτ + R T̄ 1 wᵀ.
pub fn vertical_modes(planet: &Planet, lev: &SigmaLevels) -> (Vec<f64>, Vec<Vec<f64>>) {
    let l = lev.nlev;
    let b = lev.b_matrix(planet.gas_constant, planet.kappa(), planet.t_ref);
    let mut ev = crate::util::eigenvalues_real(l, &b, 4000);
    ev.sort_by(|a, b| b.total_cmp(a)); // descending: mode 0 = external (fastest)
    let vecs = ev
        .iter()
        .map(|&lam| {
            // inverse iteration with a slightly shifted matrix
            let shift = lam * (1.0 + 1e-9) + 1e-12;
            let mut m = b.clone();
            for i in 0..l {
                m[i * l + i] -= shift;
            }
            let inv = crate::util::invert(l, &m).expect("shifted matrix singular");
            let mut x = vec![1.0; l];
            for _ in 0..8 {
                let y: Vec<f64> = (0..l)
                    .map(|i| (0..l).map(|j| inv[i * l + j] * x[j]).sum())
                    .collect();
                let n = y.iter().map(|v| v * v).sum::<f64>().sqrt();
                x = y.iter().map(|v| v / n).collect();
            }
            x
        })
        .collect();
    (ev, vecs)
}

/// Linear gravity-wave test: at rest, isothermal T = T̄, with a small T' perturbation in
/// spherical harmonic (ℓ = `degree`, m = 0) whose vertical structure excites only the
/// vertical normal mode `mode` (0 = external). δ then oscillates as sin(ωt) e_mode with
/// ω² = ℓ(ℓ+1)/a² · λ_mode.
pub fn gravity_wave(
    tr: &Transform,
    planet: &Planet,
    lev: &SigmaLevels,
    mode: usize,
    degree: usize,
    amplitude: f64,
) -> Result<PeInitial, String> {
    let l = lev.nlev;
    if mode >= l || degree == 0 || degree > tr.mmax() {
        return Err("gravity_wave: need mode < levels and 1 ≤ degree ≤ M".into());
    }
    let (_, vecs) = vertical_modes(planet, lev);
    // δ_t(0) = Λ G T' ∝ e_mode → T' = G⁻¹ e_mode (p' = 0).
    let g = lev.g_matrix(planet.gas_constant);
    let ginv = crate::util::invert(l, &g).ok_or("G singular")?;
    let t: Vec<f64> = (0..l)
        .map(|i| (0..l).map(|j| ginv[i * l + j] * vecs[mode][j]).sum())
        .collect();
    let tmax = t.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut init = isothermal(tr, planet, lev, planet.t_ref, planet.p0, 0.0, 1);
    let idx = tr.layout.idx(degree, 0);
    for k in 0..l {
        init.state.tmp[k][idx] = Complex::new((amplitude * t[k] / tmax) as Real, 0.0);
    }
    Ok(init)
}

/// Jablonowski & Williamson (2006) constants (QJRMS 132, 2943–2975, §2).
pub mod jw {
    pub const ETA0: f64 = 0.252;
    pub const ETAT: f64 = 0.2;
    pub const U0: f64 = 35.0;
    pub const T0: f64 = 288.0;
    pub const GAMMA: f64 = 0.005;
    pub const DELTA_T: f64 = 4.8e5;
    pub const UP: f64 = 1.0;
    /// L26 interface coefficients (Jablonowski & Williamson 2006, NCAR TN-469+STR,
    /// Table B.1): η_{k+1/2} = a + b at p_s = 1000 hPa = p₀, top to bottom.
    pub const A26: [f64; 27] = [
        0.002194067,
        0.004895209,
        0.009882418,
        0.01805201,
        0.02983724,
        0.04462334,
        0.06160587,
        0.07851243,
        0.07731271,
        0.07590131,
        0.07424086,
        0.07228744,
        0.06998933,
        0.06728574,
        0.06410509,
        0.06036322,
        0.05596111,
        0.05078225,
        0.04468960,
        0.03752191,
        0.02908949,
        0.02084739,
        0.01334443,
        0.00708499,
        0.00252136,
        0.0,
        0.0,
    ];
    pub const B26: [f64; 27] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.01505309, 0.03276228, 0.05359622, 0.07810627,
        0.1069411, 0.1408637, 0.1807720, 0.2277220, 0.2829562, 0.3479364, 0.4243822, 0.5143168,
        0.6201202, 0.7235355, 0.8176768, 0.8962153, 0.9534761, 0.9851122, 1.0,
    ];

    /// σ half levels from the L26 table evaluated at σ = η. The model top must be σ = 0,
    /// so the first interface (η = 0.00219) is replaced by 0.
    pub fn sigma_half_l26() -> Vec<f64> {
        let mut h: Vec<f64> = A26.iter().zip(B26.iter()).map(|(a, b)| a + b).collect();
        h[0] = 0.0;
        h
    }
}

/// JW06 horizontal-mean temperature T̄(η).
fn jw_tbar(planet: &Planet, eta: f64) -> f64 {
    let rd = planet.gas_constant;
    let g = planet.gravity;
    let mut t = jw::T0 * eta.powf(rd * jw::GAMMA / g);
    if eta < jw::ETAT {
        t += jw::DELTA_T * (jw::ETAT - eta).powi(5);
    }
    t
}

/// The bracketed latitude terms of JW06 eqs. (6)–(7).
fn jw_terms(lat: f64) -> (f64, f64) {
    let (s, c) = (lat.sin(), lat.cos());
    (
        -2.0 * s.powi(6) * (c * c + 1.0 / 3.0) + 10.0 / 63.0,
        1.6 * c.powi(3) * (s * s + 2.0 / 3.0) - PI / 4.0,
    )
}

/// Jablonowski & Williamson (2006) baroclinic wave on σ = η levels, p_s = 1000 hPa.
pub fn jw06(tr: &Transform, planet: &Planet, lev: &SigmaLevels, perturbed: bool) -> PeInitial {
    let l = lev.nlev;
    let a = planet.radius;
    let om = planet.rotation_rate;
    let rd = planet.gas_constant;
    let (nlat, nlon) = (tr.nlat(), tr.nlon());
    let mut s = PeState::zeros(l, tr.layout.len());
    let zeros = tr.grid_zeros();
    let (lc, pc) = (PI / 9.0, 2.0 * PI / 9.0);
    for k in 0..l {
        let eta = lev.full[k];
        let ev = (eta - jw::ETA0) * PI / 2.0;
        let mut ucos = vec![0.0 as Real; nlat * nlon];
        let mut t = vec![0.0 as Real; nlat * nlon];
        for j in 0..nlat {
            let lat = tr.grid.lat[j];
            let (t1, t2) = jw_terms(lat);
            let temp = jw_tbar(planet, eta)
                + 0.75 * eta * PI * jw::U0 / rd
                    * ev.sin()
                    * ev.cos().sqrt()
                    * (t1 * 2.0 * jw::U0 * ev.cos().powf(1.5) + t2 * a * om);
            let ubase = jw::U0 * ev.cos().powf(1.5) * (2.0 * lat).sin().powi(2);
            for i in 0..nlon {
                let lon = tr.grid.lon[i];
                let mut u = ubase;
                if perturbed {
                    let x = (pc.sin() * lat.sin() + pc.cos() * lat.cos() * (lon - lc).cos())
                        .clamp(-1.0, 1.0);
                    let r = a * x.acos();
                    u += jw::UP * (-(r / (a / 10.0)).powi(2)).exp();
                }
                ucos[j * nlon + i] = (u * lat.cos()) as Real;
                t[j * nlon + i] = (temp - planet.t_ref) as Real;
            }
        }
        let (d, z) = tr.div_curl(&ucos, &zeros);
        s.vor[k] = z;
        s.div[k] = d;
        s.tmp[k] = tr.analysis(&t);
    }
    s.lnps = constant(tr, 1.0e5f64.ln());
    // Surface geopotential: eq. (7) at η = 1 (Φ̄(1) = 0).
    let evs = (1.0 - jw::ETA0) * PI / 2.0;
    let mut phis = Vec::with_capacity(nlat * nlon);
    for j in 0..nlat {
        let (t1, t2) = jw_terms(tr.grid.lat[j]);
        let v = jw::U0 * evs.cos().powf(1.5) * (t1 * jw::U0 * evs.cos().powf(1.5) + t2 * a * om);
        phis.extend(std::iter::repeat_n(v as Real, nlon));
    }
    PeInitial {
        state: s,
        phis_grid: phis,
    }
}
