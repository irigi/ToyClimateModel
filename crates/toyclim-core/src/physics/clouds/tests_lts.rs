//! Single-column tests of `low_cloud = "slingo_lts"` (stability stratus, see [`super::low`]).

use super::*;
use crate::physics::column::testing::Geometry;
use crate::surface::{Tile, TileKind};

/// Level indices of the inversion pair in L26: σ 0.787 above, σ 0.867 below.
const K_TOP: usize = 21;
const K_BASE: usize = 22;

fn lts_scheme() -> DiagnosticClouds {
    DiagnosticClouds {
        low_cloud: LowCloud::SlingoLts,
        ..DiagnosticClouds::new(Condensable::water())
    }
}

fn ocean(frac: f64, sst: f64) -> Tile {
    Tile::new(TileKind::Ocean, frac, sst, f64::INFINITY, 0.07)
}

/// Run on a column with potential temperature `theta`, relative humidity `rh`, uniform
/// ω and the given surface tiles; checks that the scheme leaves T and q alone.
fn run(
    s: &DiagnosticClouds,
    theta: &[f64],
    rh: &[f64],
    omega: f64,
    tiles: Vec<Tile>,
) -> (Vec<f64>, Vec<f64>) {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let w = vec![omega; 26];
    let mut ctx = geo.ctx(&planet, None, 1200.0);
    ctx.omega = &w;
    let mut st = ColumnState::new(26, 1);
    st.begin(1200.0);
    for k in 0..26 {
        st.t[k] = theta[k] * (ctx.p[k] / 1.0e5).powf(planet.kappa());
        st.q[k] = rh[k] * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
    }
    let (t0, q0) = (st.t.clone(), st.q.clone());
    let mut sfc = SurfaceExchange {
        tiles,
        ..Default::default()
    };
    let mut diag = vec![0.0; s.diagnostics().len()];
    s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
    assert_eq!(st.t, t0);
    assert_eq!(st.q, q0);
    assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
    for k in 0..26 {
        assert!(
            (0.0..=1.0).contains(&sfc.cloud[k]),
            "k {k}: {}",
            sfc.cloud[k]
        );
    }
    (sfc.cloud, diag)
}

/// A marine boundary layer: well mixed (θ = 295 K) below σ 0.867, a 15 K jump to σ 0.787,
/// mildly stable above; moist below, dry above.
fn deck() -> (Vec<f64>, Vec<f64>) {
    let geo = Geometry::l26();
    let theta = (0..26)
        .map(|k| {
            if k <= K_TOP {
                310.0 + 0.04 * (geo.p[K_TOP] - geo.p[k]) / 100.0
            } else {
                295.0
            }
        })
        .collect();
    let rh = (0..26)
        .map(|k| if k >= K_BASE { 0.9 } else { 0.3 })
        .collect();
    (theta, rh)
}

/// LTS = θ(σ nearest 0.7) − SST (p_s = 1000 hPa, φ_s = 0) for the deck column.
fn lts_of(theta: &[f64], sst: f64) -> f64 {
    let geo = Geometry::l26();
    let k700 = (0..26)
        .min_by(|&a, &b| {
            (geo.sigma[a] - 0.7)
                .abs()
                .total_cmp(&(geo.sigma[b] - 0.7).abs())
        })
        .unwrap();
    assert!((geo.sigma[k700] - 0.696).abs() < 2e-3);
    theta[k700] - sst
}

/// Under subsidence (gate shuts the RH low cloud), the stratus sits in the lower level of
/// the inversion pair with min(1, max(0, 0.057 LTS − 0.5573)), capped by the RH (0.9); it
/// rises with LTS and is zero below LTS = 9.78 K; the diagnostics carry it and the LTS.
#[test]
fn lts_stratus_rises_with_stability() {
    let (theta, rh) = deck();
    let s = lts_scheme();
    let mut last = -1.0;
    for sst in [312.0, 305.0, 300.0, 296.0, 292.0, 288.0, 280.0] {
        let lts = lts_of(&theta, sst);
        // capped by the boundary-layer RH 0.9
        let want = (0.057 * lts - 0.5573).clamp(0.0, 0.9);
        let (m, diag) = run(&s, &theta, &rh, 0.05, vec![ocean(1.0, sst)]);
        assert!((diag[2] - lts).abs() < 1e-9, "{} vs {lts}", diag[2]);
        assert!((diag[1] - want).abs() < 1e-12, "{} vs {want}", diag[1]);
        for k in 0..26 {
            let w = if k == K_BASE { want } else { 0.0 };
            assert!((m[k] - w).abs() < 1e-12, "SST {sst} k {k}: {} vs {w}", m[k]);
        }
        if lts < 0.5573 / 0.057 {
            assert_eq!(m[K_BASE], 0.0);
        }
        assert!(m[K_BASE] >= last);
        last = m[K_BASE];
        if want > 0.0 {
            assert!(m[26 + K_BASE] + m[52 + K_BASE] > 0.0, "in-cloud water");
        }
    }
    assert!((last - 0.9).abs() < 1e-12, "{last}");
}

/// Open ocean only: no stratus over land or sea ice or without tiles; a cell half ocean
/// gets f_ocn inside the clamp; the SST is the ocean tiles' temperature, not the land's.
#[test]
fn lts_stratus_open_ocean_only() {
    let (theta, rh) = deck();
    let s = lts_scheme();
    let sst = 290.0;
    let lts = lts_of(&theta, sst);
    let land = Tile::new(TileKind::Land, 1.0, 270.0, 1e6, 0.2);
    let ice = Tile::new(TileKind::SeaIce, 1.0, 270.0, 1e6, 0.6);
    for tiles in [vec![], vec![land], vec![ice], vec![ocean(0.005, sst), land]] {
        let (m, diag) = run(&s, &theta, &rh, 0.05, tiles);
        assert_eq!(m[K_BASE], 0.0);
        assert_eq!((diag[1], diag[2]), (0.0, 0.0));
    }
    let mut half = land;
    half.frac = 0.5;
    let (m, diag) = run(&s, &theta, &rh, 0.05, vec![ocean(0.5, sst), half]);
    let want = (0.5 * (0.057 * lts - 0.5573)).clamp(0.0, 1.0);
    assert!(want > 0.0);
    assert!((m[K_BASE] - want).abs() < 1e-12, "{} vs {want}", m[K_BASE]);
    assert!((diag[2] - lts).abs() < 1e-9);
}

/// The RH cap max(RH_k, RH_{k+1}) and the combination with the RH cloud by maximum; the
/// gate still suppresses the RH low cloud under subsidence; `lts_slope = 0` is the gate
/// alone (identical to "slingo" without its inversion cloud).
#[test]
fn lts_stratus_cap_gate_and_combination() {
    let (theta, mut rh) = deck();
    let s = lts_scheme();
    let sst = 285.0;
    let strat = (0.057 * lts_of(&theta, sst) - 0.5573).clamp(0.0, 1.0);
    assert!(strat > 0.5, "{strat}");
    // dry placement level, moister level below: capped by the larger of the two (freeze-dry
    // off here: it would scale the dry layer's cloud, as for any low cloud)
    rh[K_BASE] = 0.2;
    rh[K_BASE + 1] = 0.35;
    let no_fd = DiagnosticClouds {
        freeze_dry_q: 0.0,
        ..lts_scheme()
    };
    let (m, _) = run(&no_fd, &theta, &rh, 0.05, vec![ocean(1.0, sst)]);
    assert!((m[K_BASE] - 0.35).abs() < 1e-12, "{}", m[K_BASE]);
    let (m, _) = run(&s, &theta, &rh, 0.05, vec![ocean(1.0, sst)]);
    assert!(
        m[K_BASE] < 0.35 && m[K_BASE] > 0.15 * 0.35,
        "freeze-dry: {}",
        m[K_BASE]
    );
    // nearly saturated layer under ascent: its RH cloud beats a weaker stratus (SST 292 K)
    let (theta, mut rh) = deck();
    rh[K_BASE] = 0.999;
    let sst = 292.0;
    let strat = (0.057 * lts_of(&theta, sst) - 0.5573).clamp(0.0, 1.0);
    let (m_up, _) = run(&s, &theta, &rh, -0.2, vec![ocean(1.0, sst)]);
    let c6 = DiagnosticClouds::slingo_rh(0.999, 0.8);
    assert!(c6 > strat && strat > 0.3, "{c6} {strat}");
    assert!((m_up[K_BASE] - c6).abs() < 1e-12);
    // under subsidence the stratus remains, the RH cloud of the other low layers is gone
    let (m_dn, _) = run(&s, &theta, &rh, 0.05, vec![ocean(1.0, sst)]);
    assert!((m_dn[K_BASE] - strat).abs() < 1e-12);
    for k in K_BASE + 1..26 {
        assert_eq!(m_dn[k], 0.0, "k {k}");
    }
    // gate alone: the same as "slingo" with no inversion cloud (θ neutral there)
    let gate = DiagnosticClouds {
        lts_slope: 0.0,
        ..lts_scheme()
    };
    let slingo = DiagnosticClouds {
        low_cloud: LowCloud::Slingo,
        ..lts_scheme()
    };
    let neutral = vec![300.0; 26];
    let rh = vec![0.95; 26];
    for omega in [0.05, -0.03, -0.2] {
        let (a, _) = run(&gate, &neutral, &rh, omega, vec![ocean(1.0, 300.0)]);
        let (b, _) = run(&slingo, &neutral, &rh, omega, vec![ocean(1.0, 300.0)]);
        assert_eq!(a[..26], b[..26], "ω {omega}");
    }
}

/// Random columns with mixed tiles, ω and options: fractions in range, no tendencies (in
/// `run`), the stratus diagnostic in [0, 1].
#[test]
fn lts_random_columns_in_range() {
    let geo = Geometry::l26();
    let mut seed = 987654321u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for case in 0..300 {
        let theta: Vec<f64> = (0..26)
            .map(|k| 285.0 + 40.0 * (1.0 - geo.sigma[k]) + 10.0 * rnd())
            .collect();
        let rh: Vec<f64> = (0..26).map(|_| 1.1 * rnd()).collect();
        let f = rnd();
        let tiles = vec![
            ocean(f, 270.0 + 35.0 * rnd()),
            Tile::new(TileKind::Land, 1.0 - f, 290.0, 1e6, 0.2),
        ];
        let s = DiagnosticClouds {
            convective: if case % 2 == 0 {
                ConvectiveCloud::Column
            } else {
                ConvectiveCloud::Anvil
            },
            cre_split: case % 3 == 0,
            ..lts_scheme()
        };
        let (m, diag) = run(&s, &theta, &rh, 0.4 * rnd() - 0.2, tiles);
        let i = if s.cre_split { 3 } else { 1 };
        assert!((0.0..=1.0).contains(&diag[i]), "{}", diag[i]);
        assert_eq!(m.len(), if s.cre_split { 104 } else { 78 });
    }
}

/// Configuration: the option parses, its knobs need it, the Slingo eq. (8) knobs do not go
/// with it, and the default stays "rh".
#[test]
fn lts_config_keys() {
    use crate::physics::RadiationEnv;
    let env = RadiationEnv::default();
    let cond = Condensable::water();
    let parse = |s: &str| {
        let t: toml::Table = toml::from_str(s).unwrap();
        config::diagnostic_clouds(&t, Some(&cond), &env).map(|(c, _)| c)
    };
    let c = parse("low_cloud = \"slingo_lts\"\nlts_slope = 0.05\nlow_omega = 0.2").unwrap();
    assert_eq!(c.low_cloud, LowCloud::SlingoLts);
    assert_eq!(
        (c.lts_slope, c.lts_intercept, c.low_omega),
        (0.05, -0.5573, 0.2)
    );
    assert!(c.needs_omega());
    assert_eq!(parse("").unwrap().low_cloud, LowCloud::Rh);
    assert!(parse("lts_slope = 0.05").is_err());
    assert!(parse("low_cloud = \"slingo\"\nlts_slope = 0.05").is_err());
    assert!(parse("low_cloud = \"slingo_lts\"\ninv_slope = 6").is_err());
    assert!(parse("low_cloud = \"slingo_lts\"\nlts_slope = -1").is_err());
    assert!(parse("low_omega = 0.2").is_err());
}
