//! M4 tests.
use super::*;
use crate::config::Config;

pub(crate) fn pe(t: usize, lev: &str, dt: &str, case: &str, planet: &str) -> Primitive {
    let toml = format!(
        "model = \"primitive\"\ntruncation = {t}\n{lev}\ndt = \"{dt}\"\nlength = \"1day\"\n\
         output_every = \"{dt}\"\ninitial_condition = {case}\n[planet]\n{planet}\n"
    );
    from_config(&Config::parse(&toml).unwrap()).unwrap()
}

pub(crate) fn run_days(m: &mut Primitive, days: f64) {
    let n = (days * 86400.0 / m.settings.dt).round() as u64;
    for _ in 0..n {
        m.step();
    }
}

/// M4 test 1: an isothermal resting atmosphere stays at rest.
#[test]
fn resting_atmosphere() {
    let mut m = pe(
        21,
        "levels = 10",
        "30min",
        "{ case = \"rest\", temperature = 250.0 }",
        "preset = \"earth\"",
    );
    run_days(&mut m, 10.0);
    let umax = m.conservation()[3];
    eprintln!("M4.1 rest T21L10 10 days: max|u| = {umax:.2e}");
    assert!(umax < 1e-10);
}

fn gravity_wave_run(mode: usize, degree: usize, dt_min: f64) -> (f64, f64, f64) {
    let case = format!(
        "{{ case = \"gravity_wave\", mode = {mode}, degree = {degree}, amplitude = 0.01 }}"
    );
    let mut m = pe(
        21,
        "levels = 10",
        &format!("{dt_min}min"),
        &case,
        "preset = \"earth\"\nrotation_rate = 0.0",
    );
    let (ev, vecs) = cases::vertical_modes(&m.planet, &m.levels);
    let k = (0..10)
        .max_by(|&a, &b| vecs[mode][a].abs().total_cmp(&vecs[mode][b].abs()))
        .unwrap();
    let idx = m.tr.layout.idx(degree, 0);
    let a = m.planet.radius;
    let omega = ((degree * (degree + 1)) as f64 / (a * a) * ev[mode]).sqrt();
    let dt = m.settings.dt;
    let omega_discrete = (omega * dt).atan() / dt;
    let nsteps = (30.0 * 86400.0 / dt) as usize;
    let mut series = Vec::with_capacity(nsteps + 1);
    series.push(m.curr.div[k][idx].re as f64);
    for _ in 0..nsteps {
        m.step();
        series.push(m.curr.div[k][idx].re as f64);
    }
    // zero crossings (linear interpolation)
    let mut cross = Vec::new();
    for i in 1..series.len() {
        let (y0, y1) = (series[i - 1], series[i]);
        if y0 != 0.0 && y0.signum() != y1.signum() {
            cross.push((i as f64 - 1.0 + y0 / (y0 - y1)) * dt);
        }
    }
    let measured =
        std::f64::consts::PI * (cross.len() - 1) as f64 / (cross.last().unwrap() - cross[0]);
    // growth: max amplitude in the last 5 days vs the first 5 days
    let w = (5.0 * 86400.0 / dt) as usize;
    let amp = |s: &[f64]| s.iter().fold(0.0f64, |x, v| x.max(v.abs()));
    let growth = amp(&series[series.len() - w..]) / amp(&series[..w]);
    eprintln!(
        "M4.2 mode {mode} l={degree} dt={dt_min}min: omega {measured:.6e} vs normal mode {omega:.6e} \
         (time-discrete {omega_discrete:.6e}); rel err {:.2e} (vs continuous {:.2e}); growth {growth:.4}",
        measured / omega_discrete - 1.0,
        measured / omega - 1.0
    );
    (
        measured / omega_discrete - 1.0,
        measured / omega - 1.0,
        growth,
    )
}

/// Quick version of M4 test 2 (external mode only).
#[test]
fn linear_gravity_wave_external() {
    let (e, _, growth) = gravity_wave_run(0, 3, 20.0);
    assert!(e.abs() < 0.01 && growth <= 1.0 + 1e-6);
}

/// M4 test 2: linear gravity waves on a non-rotating planet oscillate at the discrete
/// normal-mode frequencies and do not grow (30 days, four modes).
#[test]
#[ignore]
fn linear_gravity_waves() {
    for (mode, degree) in [(0, 3), (1, 5), (2, 8), (4, 10)] {
        let (e, _, growth) = gravity_wave_run(mode, degree, 10.0);
        assert!(e.abs() < 0.01, "mode {mode}: {e:e}");
        assert!(growth <= 1.0 + 1e-6, "mode {mode}: growth {growth}");
    }
}

fn jw_levels() -> String {
    let h = cases::jw::sigma_half_l26();
    format!("sigma_half = {h:?}")
}

const JW_PLANET: &str = "preset = \"earth\"\nradius = 6.371229e6\nrotation_rate = 7.29212e-5\n\
                         gas_constant = 287.0\ncp = 1004.5\ngravity = 9.80616";

/// Max change of the zonal-mean zonal wind \[m/s\].
fn zonal_mean_u_change(m: &Primitive, u0: &[Real]) -> f64 {
    let u1 = m.field("u").unwrap().data;
    let nlon = m.tr.nlon();
    u0.chunks(nlon)
        .zip(u1.chunks(nlon))
        .map(|(a, b)| {
            let za: f64 = a.iter().map(|&x| x as f64).sum::<f64>() / nlon as f64;
            let zb: f64 = b.iter().map(|&x| x as f64).sum::<f64>() / nlon as f64;
            (za - zb).abs()
        })
        .fold(0.0, f64::max)
}

/// JW06 steady state, quick: T21 L26, 2 days.
#[test]
fn jw_steady_state_short() {
    let mut m = pe(
        21,
        &jw_levels(),
        "30min",
        "{ case = \"jw06\", perturbed = false }",
        JW_PLANET,
    );
    let u0 = m.field("u").unwrap().data;
    run_days(&mut m, 2.0);
    let d = zonal_mean_u_change(&m, &u0);
    eprintln!("JW T21 steady 2 days: max |d zonal-mean u| = {d:.3e}");
    assert!(d < 0.5);
}

/// M4 test 4 (criterion part): JW06 unperturbed at T85 L26 keeps the zonal-mean u within
/// 0.5 m/s for 10 days.
#[test]
#[ignore]
fn jw_steady_state_t85() {
    let mut m = pe(
        85,
        &jw_levels(),
        "10min",
        "{ case = \"jw06\", perturbed = false }",
        JW_PLANET,
    );
    let u0 = m.field("u").unwrap().data;
    run_days(&mut m, 10.0);
    let d = zonal_mean_u_change(&m, &u0);
    eprintln!("JW T85 steady 10 days: max |d zonal-mean u| = {d:.3e}");
    assert!(d < 0.5);
}

/// M4 test 3: adiabatic conservation over 30 days (JW06 perturbed, T42 L26): mass drift
/// < 1e-10; energy and angular momentum drifts recorded.
#[test]
#[ignore]
fn conservation_30_days() {
    let mut m = pe(
        42,
        &jw_levels(),
        "20min",
        "{ case = \"jw06\", perturbed = true }",
        JW_PLANET,
    );
    let c0 = m.conservation();
    run_days(&mut m, 30.0);
    let c1 = m.conservation();
    let rel = |i: usize| (c1[i] - c0[i]) / c0[i];
    eprintln!(
        "M4.3 30 days T42L26: mass {:.2e}, energy {:.2e}, angular momentum {:.2e}",
        rel(0),
        rel(1),
        rel(2)
    );
    assert!(rel(0).abs() < 1e-10);
}

/// Restart of the PE model is bitwise.
#[test]
fn pe_restart_is_bitwise() {
    let mk = || {
        pe(
            21,
            &jw_levels(),
            "30min",
            "{ case = \"jw06\", perturbed = true }",
            JW_PLANET,
        )
    };
    let mut a = mk();
    for _ in 0..4 {
        a.step();
    }
    let (arr, sc) = a.restart_state();
    let mut b = mk();
    b.load_restart_state(&arr, &sc).unwrap();
    for _ in 0..4 {
        a.step();
        b.step();
    }
    let bits = |s: &PeState| -> Vec<u64> {
        s.tmp
            .iter()
            .flatten()
            .chain(s.lnps.iter())
            .flat_map(|c| [c.re.to_bits() as u64, c.im.to_bits() as u64])
            .collect()
    };
    assert_eq!(bits(&a.curr), bits(&b.curr));
}

/// Thread count does not change results (bitwise).
#[test]
fn pe_thread_independent() {
    let run = |threads: usize| {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        pool.install(|| {
            let mut m = pe(
                21,
                "levels = 8",
                "30min",
                "{ case = \"rest\", temperature = 260.0, noise = 0.5 }",
                "preset = \"earth\"",
            );
            for _ in 0..6 {
                m.step();
            }
            m.curr
                .tmp
                .iter()
                .flatten()
                .flat_map(|c| [c.re.to_bits() as u64, c.im.to_bits() as u64])
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(run(1), run(3));
}
