//! Single-column tests: SPEC.md P1 acceptance criteria 1–3 (§9.5.1).

use super::*;

const DRY_LAND: &str = include_str!("../../../../cases/scm/dry_land_diurnal.toml");
const OCEAN: &str = include_str!("../../../../cases/scm/ocean_diurnal.toml");
const STIFF: &str = include_str!("../../../../cases/scm/stiff_land.toml");
const SNOW: &str = include_str!("../../../../cases/scm/snow_melt.toml");
const ICE: &str = include_str!("../../../../cases/scm/sea_ice.toml");

/// Thin, partly open sea ice under warm air: surface and bottom melt until the ice is gone
/// (the transition dumps the snow into the ocean).
fn melting_ice() -> Scm {
    scm(&ICE
        .replace("t_air = 255.0", "t_air = 290.0")
        .replace("t_initial = 250.0", "t_initial = 272.0")
        .replace("ice_thickness = 1.0", "ice_thickness = 0.3")
        .replace("ice_snow = 30.0", "ice_snow = 5.0")
        .replace("mixed_layer_depth = 50.0", "mixed_layer_depth = 10.0")
        .replace("lat = 80.0", "lat = 40.0"))
}

fn scm(text: &str) -> Scm {
    Scm::from_config(&Config::parse(text).unwrap()).unwrap()
}

/// A moist land column with a half-dry bucket (β = 0.5) for the water budget over land.
fn moist_land() -> Scm {
    let text = DRY_LAND.replace("beta = 0.0", "beta = 0.5").replace(
        "[convection]\nscheme = \"dry\"",
        "[convection]\nscheme = \"sbm\"\n\n[moisture]\ncondensable = \"water\"",
    );
    let mut s = scm(&text);
    // a moist, conditionally unstable start so that convection and condensation act
    for k in 0..s.state.nlev {
        let c = s.cond.clone().unwrap();
        s.state.q[k] = 0.95 * c.qsat(&s.planet, s.state.t[k], s.p[k]);
    }
    s
}

/// SPEC.md §9.5.1 tolerances.
const ENERGY_TOL: f64 = 1e-6; // W m⁻²
const WATER_TOL: f64 = 1e-9; // kg m⁻² s⁻¹

fn check(r: &ScmRecord, what: &str) {
    assert!(
        r.energy_residual.abs() < ENERGY_TOL,
        "{what}: energy residual {:.3e} W/m²",
        r.energy_residual
    );
    assert!(
        r.water_residual.abs() < WATER_TOL,
        "{what}: water residual {:.3e} kg/m²/s",
        r.water_residual
    );
}

/// P1 acceptance 1: every scheme and the full chain conserve energy and water of column +
/// surface against their boundary fluxes, over land and ocean, by day and night.
#[test]
fn conservation_per_scheme_and_chain() {
    for (name, mut s) in [
        ("dry_land", scm(DRY_LAND)),
        ("ocean", scm(OCEAN)),
        ("stiff_land", scm(STIFF)),
        ("moist_land", moist_land()),
        ("snow_melt", scm(SNOW)),
        ("sea_ice", scm(ICE)),
        ("melting_ice", melting_ice()),
    ] {
        let (mut emax, mut wmax) = (0.0f64, 0.0f64);
        for step in 0..96 {
            // alternate: each scheme alone, then the full chain
            for i in 0..s.schemes.len() {
                let r = s.step_with(&[i]);
                check(&r, &format!("{name} step {step} {}", s.schemes[i].name()));
                emax = emax.max(r.energy_residual.abs());
                wmax = wmax.max(r.water_residual.abs());
            }
            let r = s.step();
            check(&r, &format!("{name} step {step} chain"));
            emax = emax.max(r.energy_residual.abs());
            wmax = wmax.max(r.water_residual.abs());
        }
        eprintln!("{name}: max |energy residual| {emax:.2e} W/m², max |water residual| {wmax:.2e} kg/m²/s");
    }
}

/// The moist land case does convect and rain, and the bucket takes P − E.
#[test]
fn moist_land_column_rains() {
    let mut s = moist_land();
    let ip = s.diag_index("precip_conv").unwrap();
    let il = s.diag_index("precip_ls").unwrap();
    let mut rain = 0.0;
    for _ in 0..96 {
        let r = s.step();
        rain += (r.diag[ip] + r.diag[il]) * s.settings.dt;
    }
    assert!(rain > 0.1, "precipitation {rain} kg/m²");
}

/// P1 acceptance 2: land with heat capacity 1e3 J m⁻² K⁻¹, 30 m/s wind, Δt_p = 1 h: T_s
/// shows no 2Δt oscillation (successive differences do not alternate in sign with growing
/// amplitude) over 10 days.
#[test]
fn stiff_land_has_no_two_step_oscillation() {
    let mut s = scm(STIFF);
    assert_eq!(s.settings.dt, 3600.0);
    let ts: Vec<f64> = (0..240).map(|_| s.step().ts).collect();
    assert!(ts.iter().all(|t| t.is_finite() && *t > 150.0 && *t < 400.0));
    let d: Vec<f64> = ts.windows(2).map(|w| w[1] - w[0]).collect();
    // longest run of alternating signs with growing amplitude
    let (mut run, mut longest) = (0, 0);
    for w in d.windows(2) {
        if w[0] * w[1] < 0.0 && w[1].abs() > w[0].abs() && w[1].abs() > 1e-6 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let range = ts.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        - ts.iter().cloned().fold(f64::INFINITY, f64::min);
    eprintln!("stiff land: T_s range {range:.2} K, longest growing alternation {longest}");
    assert!(
        longest < 3,
        "2Δt oscillation: {longest} growing alternations"
    );
}

fn diurnal(mut s: Scm) -> (f64, f64, Vec<f64>) {
    let it = s.diag_index("tas").unwrap();
    let per_day = (86400.0 / s.settings.dt).round() as usize;
    let mut tas = Vec::new();
    let mut night_stable = Vec::new();
    for _ in 0..10 * per_day {
        let r = s.step();
        tas.push(r.diag[it]);
        night_stable.push(r.diag[s.diag_index("shf").unwrap()]);
    }
    let last = &tas[9 * per_day..];
    let max = last.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = last.iter().cloned().fold(f64::INFINITY, f64::min);
    (max - min, min, night_stable[9 * per_day..].to_vec())
}

/// P1 acceptance 3 (land): dry land at 20°, equinox, clear sky: 2 m diurnal range 8–25 K;
/// the nocturnal boundary layer is stable.
#[test]
fn dry_land_diurnal_cycle() {
    let mut s = scm(DRY_LAND);
    let per_day = (86400.0 / s.settings.dt).round() as usize;
    let (it, ish, ipb) = (
        s.diag_index("tas").unwrap(),
        s.diag_index("shf").unwrap(),
        s.diag_index("pblh").unwrap(),
    );
    let kappa = s.planet.kappa();
    let mut rows = Vec::new();
    for n in 0..10 * per_day {
        let r = s.step();
        if n >= 9 * per_day {
            let l = s.state.nlev;
            let th = |k: usize| s.state.t[k] * (1e5 / s.p[k]).powf(kappa);
            rows.push((
                r.diag[it],
                r.diag[ish],
                r.diag[ipb],
                s.surface.ts_cell(0),
                th(l - 2) - th(l - 1),
            ));
        }
    }
    let max = rows.iter().map(|r| r.0).fold(f64::NEG_INFINITY, f64::max);
    let min = rows.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
    let range = max - min;
    // night: the coldest hour of the 2 m temperature
    let night = rows
        .iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .unwrap();
    let day = rows
        .iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .unwrap();
    eprintln!(
        "dry land: tas {min:.1}–{max:.1} K (range {range:.1} K); night: shf {:.1} W/m², pblh {:.0} m, \
         T_s {:.1} K, θ(L−1) − θ(L) {:.2} K; day: shf {:.1} W/m², pblh {:.0} m, T_s {:.1} K",
        night.1, night.2, night.3, night.4, day.1, day.2, day.3
    );
    assert!(
        (8.0..=25.0).contains(&range),
        "2 m diurnal range {range:.2} K"
    );
    // stable nocturnal boundary layer: downward sensible heat flux, potential temperature
    // increasing with height above the lowest level
    assert!(
        night.1 < 0.0,
        "night-time sensible heat flux {:.2}",
        night.1
    );
    assert!(
        night.4 > 0.0,
        "night-time lapse not stable: {:.3} K",
        night.4
    );
}

/// P1 acceptance 3 (ocean): over a 50 m mixed layer the 2 m diurnal range is below 1 K.
#[test]
fn ocean_diurnal_cycle() {
    let (range, min, _) = diurnal(scm(OCEAN));
    eprintln!("ocean: tas diurnal range {range:.3} K (min {min:.1} K)");
    assert!(range < 1.0, "2 m diurnal range {range:.3} K");
}

/// P2 acceptance 1 (snowpack): while snow remains the land surface does not exceed the
/// freezing point; the sun melts snow; melt water reaches the bucket.
#[test]
fn snow_covered_land_stays_at_or_below_freezing() {
    let mut s = scm(SNOW);
    let t_melt = s.surface.params.t_melt;
    let (s0, w0) = (s.surface.snow[0], s.surface.bucket[0]);
    let mut melted_some = false;
    for _ in 0..480 {
        let had_snow = s.surface.snow[0] > 0.0;
        let r = s.step();
        check(&r, "snow_melt");
        if had_snow && s.surface.snow[0] > 0.0 {
            assert!(
                s.surface.soil_t[0][0] <= t_melt + 1e-9,
                "snow-covered T_s {}",
                s.surface.soil_t[0][0]
            );
        }
        melted_some |= s.surface.snow[0] < s0 - 1.0;
    }
    eprintln!(
        "snow melt: snow {s0:.1} → {:.2} kg/m², bucket {w0:.1} → {:.1} kg/m², T_s {:.2} K",
        s.surface.snow[0], s.surface.bucket[0], s.surface.soil_t[0][0]
    );
    assert!(melted_some, "no snow melted");
    assert!(s.surface.bucket[0] > w0);
}

/// P2 acceptance 1 (sea ice): cold air over thick ice grows it at the base; the ice surface
/// stays at or below the melting point; warm air melts thin ice away (conservation through
/// the transition is checked in `conservation_per_scheme_and_chain`).
#[test]
fn sea_ice_grows_in_the_cold_and_melts_in_the_warm() {
    let mut s = scm(ICE);
    let h0 = s.surface.ice_h[0];
    for _ in 0..480 {
        let r = s.step();
        check(&r, "sea_ice");
        assert!(s.surface.ice_ts[0] <= s.surface.params.t_melt + 1e-9);
        assert!(s.surface.t_ml[0] >= s.surface.params.t_freeze_sea - 1e-9);
    }
    let h1 = s.surface.ice_h[0];
    eprintln!(
        "sea ice (cold): h {h0:.3} → {h1:.3} m, snow {:.1} kg/m², T_ice {:.1} K",
        s.surface.ice_snow[0], s.surface.ice_ts[0]
    );
    assert!(h1 > h0, "ice did not grow");
    let mut m = melting_ice();
    let h0 = m.surface.ice_h[0];
    for _ in 0..20 * 48 {
        let r = m.step();
        check(&r, "melting_ice");
        if m.surface.ice_h[0] == 0.0 {
            break;
        }
    }
    eprintln!(
        "sea ice (warm): h {h0:.3} → {:.3} m, T_ml {:.2} K, snow on ice {:.2}",
        m.surface.ice_h[0], m.surface.t_ml[0], m.surface.ice_snow[0]
    );
    assert!(m.surface.ice_h[0] < h0, "ice did not melt");
    assert_eq!(m.surface.ice_h[0], 0.0, "ice did not melt away");
    assert_eq!(m.surface.ice_snow[0], 0.0, "snow left without ice");
    // and the column keeps conserving after the transition (open water only)
    for _ in 0..48 {
        check(&m.step(), "after the ice");
    }
}
