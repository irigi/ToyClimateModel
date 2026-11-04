//! M2 acceptance tests that run in a few seconds; longer ones are `#[ignore]`.

use super::cases::{self, error_norms};
use super::*;
use crate::config::Config;

fn sw(case: &str, t: usize, dt: &str, extra: &str) -> ShallowWater {
    let toml = format!(
        "model = \"shallow_water\"\ntruncation = {t}\ndt = \"{dt}\"\nlength = \"1day\"\n\
         output_every = \"{dt}\"\ninitial_condition = {{ case = \"{case}\" {extra} }}\n"
    );
    from_config(&Config::parse(&toml).unwrap()).unwrap()
}

fn run_days(m: &mut ShallowWater, days: f64) {
    let n = (days * 86400.0 / m.settings.dt).round() as u64;
    for _ in 0..n {
        m.step();
    }
}

fn w2_errors(t: usize, alpha: f64, days: f64) -> ((f64, f64, f64), f64) {
    let mut m = sw("williamson2", t, "30min", &format!(", alpha = {alpha}"));
    let mass0 = m.conservation()[0];
    run_days(&mut m, days);
    let h = m.grid_field("h").unwrap();
    let g = m.planet.gravity;
    let truth: Vec<Real> = {
        let mut v = Vec::new();
        for &lat in &m.tr.grid.lat {
            for &lon in &m.tr.grid.lon {
                v.push((cases::williamson2_gh(&m.planet, alpha, lon, lat) / g) as Real);
            }
        }
        v
    };
    let mass1 = m.conservation()[0];
    (error_norms(&m.tr, &h, &truth), (mass1 / mass0 - 1.0).abs())
}

/// M2: Williamson test 2 at T42, Δt = 30 min, 5 days, both α.
#[test]
fn williamson2_t42() {
    for alpha in [0.0, std::f64::consts::FRAC_PI_2 - 0.05] {
        let ((l1, l2, li), dm) = w2_errors(42, alpha, 5.0);
        eprintln!(
            "W2 T42 alpha={alpha:.3}: l1={l1:.2e} l2={l2:.2e} linf={li:.2e} mass drift={dm:.2e}"
        );
        assert!(l2 < 1e-5, "l2 = {l2:e}");
        assert!(dm < 1e-12, "mass drift {dm:e}");
    }
}

/// Resting fluid stays at rest (sanity of signs).
#[test]
fn rest_stays_at_rest() {
    let mut m = sw("rest", 21, "30min", "");
    run_days(&mut m, 2.0);
    let u = m.grid_field("u").unwrap();
    assert!(crate::model::max_abs(&u) < 1e-10);
}

/// Restart: continuing from a saved state reproduces an uninterrupted run bitwise.
#[test]
fn restart_is_bitwise() {
    let mut a = sw("williamson6", 21, "20min", "");
    for _ in 0..10 {
        a.step();
    }
    let (arrays, scalars) = a.restart_state();
    let mut b = sw("williamson6", 21, "20min", "");
    b.load_restart_state(&arrays, &scalars).unwrap();
    for _ in 0..10 {
        a.step();
        b.step();
    }
    let same = a
        .curr
        .vor
        .iter()
        .zip(&b.curr.vor)
        .all(|(x, y)| x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits());
    assert!(same);
}

/// W2 convergence table (record): T21, T42, T85.
#[test]
#[ignore]
fn williamson2_convergence() {
    for t in [21, 42, 85] {
        for alpha in [0.0, std::f64::consts::FRAC_PI_2 - 0.05] {
            let ((l1, l2, li), dm) = w2_errors(t, alpha, 5.0);
            eprintln!(
                "W2 T{t} alpha={alpha:.3}: l1={l1:.2e} l2={l2:.2e} linf={li:.2e} mass={dm:.1e}"
            );
            assert!(l2 < 1e-5);
        }
    }
}

fn relative_drift(a: f64, b: f64) -> f64 {
    (b / a - 1.0).abs()
}

fn test5(t: usize, dt: &str) -> [f64; 2] {
    let mut m = sw("williamson5", t, dt, "");
    let c0 = m.conservation();
    run_days(&mut m, 15.0);
    assert!(m.is_finite(), "T{t} test 5 blew up");
    let c1 = m.conservation();
    let (dm, de) = (relative_drift(c0[0], c1[0]), relative_drift(c0[1], c1[1]));
    eprintln!(
        "W5 T{t}: mass drift {dm:.2e}, energy change {de:.2e}, enstrophy change {:.2e}, AM change {:.2e}",
        relative_drift(c0[2], c1[2]),
        relative_drift(c0[3], c1[3])
    );
    [dm, de]
}

/// M2 test 5 at T42: stable for 15 days, mass conserved.
#[test]
fn williamson5_t42() {
    let [dm, _] = test5(42, "20min");
    assert!(dm < 1e-12);
}

/// M2 test 5 at T85: also total energy change < 1%.
#[test]
#[ignore]
fn williamson5_t85() {
    let [dm, de] = test5(85, "10min");
    assert!(dm < 1e-12);
    assert!(de < 0.01);
}

/// M2 test 6 cross-check: the nonlinear balance equation ∇²(φ + E) = C(ηU, ηV) with δ = 0
/// reproduces the analytic Rossby–Haurwitz height up to a constant.
#[test]
fn rossby_haurwitz_balance() {
    let m = sw("williamson6", 42, "20min", "");
    let tr = &m.tr;
    let (u, v) = tr.uv_from_vort_div(&m.curr.vor, &tr.spec_zeros());
    let vor = tr.synthesis(&m.curr.vor);
    let n = tr.ngrid();
    let nlon = tr.nlon();
    let (mut au, mut av, mut ke) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for j in 0..tr.nlat() {
        let f = (2.0 * m.planet.rotation_rate * tr.grid.mu[j]) as Real;
        for k in j * nlon..(j + 1) * nlon {
            au[k] = (vor[k] + f) * u[k];
            av[k] = (vor[k] + f) * v[k];
            ke[k] = 0.5 * (u[k] * u[k] + v[k] * v[k]) * tr.inv_cos2[j];
        }
    }
    let rhs = tr.curl(&au, &av);
    let mut phi = crate::spectral::ops::inv_laplacian(&tr.layout, &rhs, tr.radius);
    let ke_s = tr.analysis(&ke);
    for (p, e) in phi.iter_mut().zip(&ke_s) {
        *p -= *e;
    }
    phi[0] = Complex::new(0.0, 0.0);
    let mut exact = m.curr.phi.clone();
    exact[0] = Complex::new(0.0, 0.0);
    let (g1, g2) = (tr.synthesis(&phi), tr.synthesis(&exact));
    let err = g1
        .iter()
        .zip(&g2)
        .map(|(a, b)| (a - b).abs() as f64)
        .fold(0.0, f64::max)
        / crate::model::max_abs(&g2);
    eprintln!("RH balance cross-check: max rel error {err:.2e}");
    assert!(err < 1e-8);
}

/// Phase (radians) of the (ℓ = 5, m = 4) vorticity coefficient.
fn rh_phase(m: &ShallowWater) -> f64 {
    let c = m.curr.vor[m.tr.layout.idx(5, 4)];
    (c.im as f64).atan2(c.re as f64)
}

/// M2 test 6: 14 days at T42; wave 4 retained; energy/enstrophy drift < 1%; phase speed
/// within 15% of the nondivergent value.
#[test]
fn williamson6_t42() {
    let mut m = sw("williamson6", 42, "20min", "");
    let c0 = m.conservation();
    let mut phase = rh_phase(&m);
    let mut travelled = 0.0; // eastward, radians
    for _ in 0..14 {
        run_days(&mut m, 1.0);
        let p = rh_phase(&m);
        // pattern cos(4λ + arg c): eastward motion decreases arg c
        let mut d = phase - p;
        while d > std::f64::consts::PI {
            d -= 2.0 * std::f64::consts::PI;
        }
        while d < -std::f64::consts::PI {
            d += 2.0 * std::f64::consts::PI;
        }
        travelled += d / 4.0;
        phase = p;
    }
    let c1 = m.conservation();
    let speed = travelled.to_degrees() / 14.0;
    let exact = cases::rossby_haurwitz_speed(&m.planet).to_degrees() * 86400.0;
    // wave-4 dominance: (ℓ=5, m=4) is still the largest non-zonal vorticity coefficient
    let lay = &m.tr.layout;
    let c54 = m.curr.vor[lay.idx(5, 4)].norm();
    let others = (0..lay.len())
        .filter(|&k| lay.m_of[k] != 0 && k != lay.idx(5, 4))
        .map(|k| m.curr.vor[k].norm())
        .fold(0.0, Real::max);
    let (de, dz) = (relative_drift(c0[1], c1[1]), relative_drift(c0[2], c1[2]));
    eprintln!(
        "W6 T42: phase speed {speed:.2} °/day (nondivergent {exact:.2}), energy drift {de:.2e}, \
         enstrophy drift {dz:.2e}, |c54| / max other non-zonal = {:.1}",
        c54 / others
    );
    assert!(c54 > 3.0 * others, "wave-4 pattern lost");
    assert!(de < 0.01 && dz < 0.01);
    assert!(((speed - exact) / exact).abs() < 0.15);
}

/// Galewsky unperturbed jet at T85: max |Δζ| / max |ζ| < 1% after 6 days.
#[test]
#[ignore]
fn galewsky_unperturbed_t85() {
    let mut m = sw("galewsky", 85, "8min", ", perturbed = false");
    let z0 = m.tr.synthesis(&m.curr.vor);
    run_days(&mut m, 6.0);
    let z1 = m.tr.synthesis(&m.curr.vor);
    let d: Vec<Real> = z0.iter().zip(&z1).map(|(a, b)| a - b).collect();
    let r = crate::model::max_abs(&d) / crate::model::max_abs(&z0);
    eprintln!("Galewsky unperturbed T85: max|dzeta|/max|zeta| = {r:.2e}");
    assert!(r < 0.01);
}
