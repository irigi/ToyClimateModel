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
