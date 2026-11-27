//! Single-column tests of the diagnostic clouds.

use super::*;
use crate::physics::column::testing::Geometry;
use crate::physics::radiation::solver::cloud_cover;

fn scheme() -> DiagnosticClouds {
    DiagnosticClouds::new(Condensable::water())
}

#[test]
fn fraction_limits_and_profile() {
    assert_eq!(DiagnosticClouds::stratiform(0.5, 0.8), 0.0);
    assert_eq!(DiagnosticClouds::stratiform(1.01, 0.8), 1.0);
    let c = DiagnosticClouds::stratiform(0.95, 0.8);
    assert!((c - (1.0 - 0.25f64.sqrt())).abs() < 1e-12);
    let s = scheme();
    assert!((s.rh_crit(1.0) - 0.95).abs() < 1e-12);
    assert!((s.rh_crit(0.4) - 0.9).abs() < 1e-6);
    assert_eq!(s.convective(0.0), 0.0);
    let ten = 10.0 / 86400.0;
    assert!((s.convective(ten) - 0.125 * 11f64.ln()).abs() < 1e-12);
    assert_eq!(s.convective(1.0), 0.8);
    // freeze-dry: factor q / 3 g/kg between 0.15 and 1, at p ≥ 750 hPa only; off with q_fd = 0
    let off = DiagnosticClouds {
        freeze_dry_q: 0.0,
        ..scheme()
    };
    assert_eq!(off.freeze_dry(1e-4, 95000.0), 1.0);
    let f = scheme();
    assert!((f.freeze_dry(1.5e-3, 90000.0) - 0.5).abs() < 1e-12);
    assert_eq!(f.freeze_dry(1e-4, 90000.0), 0.15);
    assert_eq!(f.freeze_dry(5e-3, 90000.0), 1.0);
    assert_eq!(f.freeze_dry(1e-4, 50000.0), 1.0);
}

/// Freeze-dry in a cold, dry saturated column: low cloud reduced, still no tendencies,
/// fractions in [0, 1]; a warm moist column is unchanged.
#[test]
fn freeze_dry_column() {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let ctx = geo.ctx(&planet, None, 1200.0);
    let run = |s: &DiagnosticClouds, t_sfc: f64| {
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = t_sfc - 50.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.97 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        let q0 = st.q.clone();
        let mut sfc = SurfaceExchange::default();
        s.apply(&ctx, &mut st, &mut sfc, &mut [0.0], &mut Workspace::new(26));
        assert_eq!(st.q, q0);
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        assert!(sfc.cloud[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
        sfc.cloud
    };
    let off = DiagnosticClouds {
        freeze_dry_q: 0.0,
        ..scheme()
    };
    let on = scheme();
    let (a, b) = (run(&off, 245.0), run(&on, 245.0));
    assert!(b[25] < 0.5 * a[25] && b[25] > 0.0, "{} {}", a[25], b[25]);
    assert_eq!(a[5], b[5]); // aloft unchanged
    assert_eq!(run(&off, 300.0), run(&on, 300.0));
}

/// CRE split: same fraction and water blocks as without it, a fourth block with the
/// stratiform-only fraction (≤ the total), and the convective fraction and in-cloud path.
#[test]
fn cre_split_memory() {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let ctx = geo.ctx(&planet, None, 1200.0);
    let run = |s: &DiagnosticClouds| {
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.93 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        st.conv_precip = 20.0 / 86400.0;
        st.conv_top = 8;
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0; 3];
        s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        (sfc.cloud, diag)
    };
    let (a, da) = run(&scheme());
    let on = DiagnosticClouds {
        cre_split: true,
        ..scheme()
    };
    let (b, db) = run(&on);
    assert_eq!(a.len(), 78);
    assert_eq!(b.len(), 104);
    assert_eq!(&a[..], &b[..78]);
    assert_eq!(da[0], db[0]);
    let cc = on.convective(20.0 / 86400.0);
    assert!((db[1] - cc).abs() < 1e-12);
    assert!(db[2] > 0.0);
    for k in 0..26 {
        assert!(b[78 + k] <= b[k] + 1e-15);
        if (8..25).contains(&k) && b[k] > 0.0 {
            let expect = 1.0 - (1.0 - b[78 + k]) * (1.0 - cc);
            assert!((b[k] - expect).abs() < 1e-12, "{k}");
        }
    }
    // without convection the stratiform-only block equals the total
    let mut st = ColumnState::new(26, 1);
    st.begin(1200.0);
    for k in 0..26 {
        st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
        st.q[k] = 0.93 * on.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
    }
    let mut sfc = SurfaceExchange::default();
    let mut d = [0.0; 3];
    on.apply(&ctx, &mut st, &mut sfc, &mut d, &mut Workspace::new(26));
    assert_eq!(&sfc.cloud[..26], &sfc.cloud[78..104]);
    assert_eq!(d[1], 0.0);
}

/// CRE split without low stratiform cloud (`cre_split_low_sigma`): the first three
/// blocks and the diagnostics are unchanged; the fourth block equals the total above σ_low
/// and the convective fraction alone at and below it (zero where there is no convection).
#[test]
fn cre_split_low_memory() {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let ctx = geo.ctx(&planet, None, 1200.0);
    let run = |s: &DiagnosticClouds, conv: bool| {
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.97 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        if conv {
            st.conv_precip = 20.0 / 86400.0;
            st.conv_top = 8;
        }
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0; 3];
        s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        (sfc.cloud, diag)
    };
    for convective in [ConvectiveCloud::Column, ConvectiveCloud::Anvil] {
        let conv = DiagnosticClouds {
            cre_split: true,
            convective,
            ..scheme()
        };
        let low = DiagnosticClouds {
            cre_split_low_sigma: 0.7,
            ..conv.clone()
        };
        for with_conv in [false, true] {
            let (a, da) = run(&conv, with_conv);
            let (b, db) = run(&low, with_conv);
            assert_eq!(&a[..78], &b[..78]);
            assert_eq!(da, db);
            let mut dropped = false;
            for k in 0..26 {
                let (c, cs, x) = (b[k], a[78 + k], b[78 + k]);
                if ctx.sigma[k] < 0.7 {
                    assert_eq!(x, c, "{k}");
                } else {
                    // the convective cover of the layer: c = 1 − (1 − cs)(1 − cv)
                    let cv = if cs < 1.0 {
                        1.0 - (1.0 - c) / (1.0 - cs)
                    } else {
                        x
                    };
                    assert!((x - cv).abs() < 1e-12, "{k}: {x} {cv}");
                    assert!(x <= c + 1e-15);
                    if !with_conv {
                        assert_eq!(x, 0.0);
                    }
                    dropped |= cs > 0.0;
                }
            }
            assert!(dropped, "the test column must have low stratiform cloud");
        }
    }
}

/// The scheme leaves T and q alone, stores fractions in [0, 1] and positive paths, and
/// a saturated layer is overcast.
#[test]
fn column_memory_and_no_tendencies() {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let ctx = geo.ctx(&planet, None, 1200.0);
    let s = scheme();
    let mut st = ColumnState::new(26, 1);
    st.begin(1200.0);
    for k in 0..26 {
        st.t[k] = 290.0 - 60.0 * (1.0 - ctx.sigma[k]);
        st.q[k] = 0.5 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
    }
    st.q[20] = 1.05 * s.cond.qsat(ctx.planet, st.t[20], ctx.p[20]);
    st.conv_precip = 20.0 / 86400.0;
    st.conv_top = 10;
    let (t0, q0) = (st.t.clone(), st.q.clone());
    let mut sfc = SurfaceExchange::default();
    let mut diag = [0.0; 1];
    s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
    assert_eq!(st.t, t0);
    assert_eq!(st.q, q0);
    assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
    let m = &sfc.cloud;
    assert_eq!(m.len(), 78);
    assert_eq!(m[20], 1.0);
    let cv = s.convective(st.conv_precip);
    assert!((m[12] - cv).abs() < 1e-12, "{}", m[12]);
    assert_eq!(m[5], 0.0); // above the convection, dry
    assert_eq!(m[25], 0.0); // lowest level: no convective cloud
    assert!(m.iter().all(|&x| x >= 0.0));
    assert!(m[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
    assert!(cloud_cover(&m[..26]) >= 1.0 - 1e-12 && diag[0] > 0.0);
    // warm layer liquid, cold layer ice
    assert!(m[26 + 20] > 0.0);
}
/// Slingo (1987) low cloud on a column with temperature from potential temperature
/// `theta` and relative humidity `rh` per level, uniform ω, optional convection.
fn slingo_run(
    s: &DiagnosticClouds,
    theta: &[f64],
    rh: &[f64],
    omega: f64,
    conv: bool,
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
    if conv {
        st.conv_precip = 20.0 / 86400.0;
        st.conv_top = 8;
    }
    let (t0, q0) = (st.t.clone(), st.q.clone());
    let mut sfc = SurfaceExchange::default();
    let mut diag = vec![0.0; s.diagnostics().len()];
    s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
    assert_eq!(st.t, t0);
    assert_eq!(st.q, q0);
    assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
    (sfc.cloud, diag)
}

fn slingo() -> DiagnosticClouds {
    DiagnosticClouds {
        low_cloud: LowCloud::Slingo,
        ..scheme()
    }
}

/// Eqs. (6)–(7): no RH low cloud under subsidence, half at ω = −0.05 Pa/s, all of
/// [(RH − 0.8)/0.2]² from ω = −0.1 Pa/s; nothing in the lowest layer; levels above σ 0.8
/// keep the Sundqvist cloud; no inversion in a neutral column.
#[test]
fn slingo_low_suppressed_under_subsidence() {
    let geo = Geometry::l26();
    let theta = vec![300.0; 26]; // neutral: Δθ/Δp = 0, no inversion cloud
    let rh: Vec<f64> = geo
        .sigma
        .iter()
        .map(|&s| if s >= 0.6 { 0.97 } else { 0.3 })
        .collect();
    for convective in [ConvectiveCloud::Column, ConvectiveCloud::Anvil] {
        let s = DiagnosticClouds {
            convective,
            ..slingo()
        };
        let rhs = DiagnosticClouds {
            convective,
            ..scheme()
        };
        let (m_rh, _) = slingo_run(&rhs, &theta, &rh, 0.05, false);
        let c6 = ((0.97 - 0.8) / 0.2f64).powi(2);
        for (omega, f) in [
            (0.05, 0.0),
            (0.0, 0.0),
            (-0.05, 0.5),
            (-0.1, 1.0),
            (-0.3, 1.0),
        ] {
            let (m, diag) = slingo_run(&s, &theta, &rh, omega, false);
            assert_eq!(diag[1], 0.0, "no inversion cloud");
            assert!(diag[2].abs() < 1e-12, "neutral column");
            for k in 0..26 {
                let sg = geo.sigma[k];
                let want = if k == 25 {
                    0.0
                } else if sg >= 0.8 {
                    f * c6
                } else {
                    m_rh[k]
                };
                assert!(
                    (m[k] - want).abs() < 1e-12,
                    "{convective:?} ω {omega} k {k}: {} vs {want}",
                    m[k]
                );
            }
        }
        // eq. (5): convective cloud dries the environment, RH_e = RH (1 − C_c)
        let (m, _) = slingo_run(&s, &theta, &rh, -0.2, true);
        let cc = s.convective(20.0 / 86400.0);
        if convective == ConvectiveCloud::Column {
            let k = 23; // σ 0.93, convective layer
            let cs = DiagnosticClouds::slingo_rh(0.97 * (1.0 - cc), 0.8);
            let want = 1.0 - (1.0 - cs) * (1.0 - cc);
            assert!((m[k] - want).abs() < 1e-12, "{} vs {want}", m[k]);
        }
    }
    // the Sundqvist law under the ω gate
    let s = DiagnosticClouds {
        low_rh: LowRh::Sundqvist,
        ..slingo()
    };
    let (m_rh, _) = slingo_run(&scheme(), &theta, &rh, 0.0, false);
    let (m, _) = slingo_run(&s, &theta, &rh, -0.05, false);
    assert!((m[23] - 0.5 * m_rh[23]).abs() < 1e-12 && m_rh[23] > 0.0);
}

/// Eqs. (8)–(9): a 15 K jump in θ between σ 0.787 and 0.867 (80 hPa) gives
/// C' = 6.67 · 15/80 − 0.667 at the base (σ 0.867), scaled by (RH_base − 0.6)/0.2, under
/// subsidence; the diagnostics carry C_inv and Δθ/Δp; a dry base gives none.
#[test]
fn slingo_inversion_cloud() {
    let geo = Geometry::l26();
    let (k_top, k_base) = (21, 22);
    assert!((geo.sigma[k_top] - 0.787).abs() < 1e-3 && (geo.sigma[k_base] - 0.867).abs() < 1e-3);
    // free troposphere mildly stable (≈ −0.04 K/hPa), well mixed below the inversion
    let theta: Vec<f64> = (0..26)
        .map(|k| {
            if k <= k_top {
                310.0 + 0.04 * (geo.p[k_top] - geo.p[k]) / 100.0
            } else {
                295.0
            }
        })
        .collect();
    let dthdp = (theta[k_top] - theta[k_base]) / ((geo.p[k_top] - geo.p[k_base]) / 100.0);
    let c8 = -6.67 * dthdp - 0.667;
    assert!(c8 > 0.4 && c8 < 0.8, "{c8}");
    for convective in [ConvectiveCloud::Column, ConvectiveCloud::Anvil] {
        for cre_split in [false, true] {
            let s = DiagnosticClouds {
                convective,
                cre_split,
                ..slingo()
            };
            let d0 = if cre_split { 3 } else { 1 };
            for (rhb, f) in [(0.9, 1.0), (0.8, 1.0), (0.7, 0.5), (0.6, 0.0), (0.5, 0.0)] {
                let rh: Vec<f64> = (0..26)
                    .map(|k| if k == k_base { rhb } else { 0.3 })
                    .collect();
                let (m, diag) = slingo_run(&s, &theta, &rh, 0.05, false);
                let want = c8 * f;
                assert!((diag[d0] - want).abs() < 1e-9, "{} vs {want}", diag[d0]);
                assert!((diag[d0 + 1] - dthdp).abs() < 1e-12);
                for k in 0..26 {
                    let w = if k == k_base { want } else { 0.0 };
                    assert!((m[k] - w).abs() < 1e-9, "k {k}: {} vs {w}", m[k]);
                    if cre_split {
                        assert!((m[78 + k] - w).abs() < 1e-9, "stratiform block");
                    }
                }
                if want > 0.0 {
                    assert!(m[26 + k_base] + m[52 + k_base] > 0.0, "in-cloud water");
                }
            }
            // inversion cloud and RH cloud in one layer: the larger
            let rh: Vec<f64> = (0..26)
                .map(|k| if k == k_base { 0.99 } else { 0.3 })
                .collect();
            let (m, _) = slingo_run(&s, &theta, &rh, -0.2, false);
            let c6 = DiagnosticClouds::slingo_rh(0.99, 0.8);
            assert!((m[k_base] - c6.max(c8)).abs() < 1e-9);
        }
    }
    // a surface inversion (lowest pair) is not used; a strong one saturates at 1
    let mut th2 = vec![300.0; 26];
    th2[25] = 280.0;
    let (_, diag) = slingo_run(&slingo(), &th2, &[0.9; 26], 0.05, false);
    assert!(diag[1] == 0.0 && diag[2].abs() < 1e-12, "{diag:?}");
    let mut th3 = vec![300.0; 26];
    th3[k_top] = 340.0;
    let (m, diag) = slingo_run(&slingo(), &th3, &[0.9; 26], 0.05, false);
    assert_eq!(diag[1], 1.0);
    assert_eq!(m[k_base], 1.0);
}

/// Random columns, ω, convection and options: fractions and memory in range, no
/// tendencies, Slingo cloud only where allowed.
#[test]
fn slingo_random_columns_in_range() {
    let geo = Geometry::l26();
    let mut seed = 12345u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for case in 0..400 {
        let theta: Vec<f64> = (0..26)
            .map(|k| 285.0 + 40.0 * (1.0 - geo.sigma[k]) + 10.0 * rnd())
            .collect();
        let rh: Vec<f64> = (0..26).map(|_| 1.1 * rnd()).collect();
        let s = DiagnosticClouds {
            convective: if case % 2 == 0 {
                ConvectiveCloud::Column
            } else {
                ConvectiveCloud::Anvil
            },
            cre_split: case % 3 == 0,
            low_rh: if case % 5 == 0 {
                LowRh::Sundqvist
            } else {
                LowRh::Slingo
            },
            ..slingo()
        };
        let (m, diag) = slingo_run(&s, &theta, &rh, 0.4 * rnd() - 0.2, case % 4 < 2);
        let nb = if s.cre_split { 4 } else { 3 };
        assert_eq!(m.len(), nb * 26);
        for k in 0..26 {
            assert!((0.0..=1.0).contains(&m[k]), "case {case} k {k}: {}", m[k]);
            assert!(m[26 + k] >= 0.0 && m[52 + k] >= 0.0);
            if s.cre_split {
                assert!((0.0..=m[k] + 1e-15).contains(&m[78 + k]));
            }
        }
        assert_eq!(
            m[25], 0.0,
            "lowest layer: no stratiform and no convective cloud"
        );
        let i = if s.cre_split { 3 } else { 1 };
        assert!((0.0..=1.0).contains(&diag[i]), "{}", diag[i]);
    }
}

/// Slingo (1987) anvil cover: zero up to his C_c = 0.4 (P = 3.4 mm/day), 2 (C_c − 0.3)
/// above, 1 at his cap (P ≥ 85 mm/day).
#[test]
fn anvil_cover_follows_slingo() {
    let mmd = 1.0 / 86400.0;
    assert_eq!(DiagnosticClouds::anvil(0.0), 0.0);
    assert_eq!(DiagnosticClouds::anvil(3.3 * mmd), 0.0);
    assert!(DiagnosticClouds::anvil(3.6 * mmd) > 0.2);
    // Table 1: C_c = 0.6 at 17 mm/day -> C_H = 0.6
    assert!((DiagnosticClouds::anvil(17.0 * mmd) - 0.6).abs() < 0.01);
    assert!((DiagnosticClouds::anvil(85.0 * mmd) - 1.0).abs() < 1e-3);
    assert_eq!(DiagnosticClouds::anvil(500.0 * mmd), 1.0);
}

/// "anvil" geometry in a dry column (no stratiform cloud): deep convection gives the core
/// share over the depth, the shallow remainder at σ ≥ 0.8, an anvil in the top layer with
/// thin cloud, the convective water path capped; shallow convection fills its depth with
/// C_c and has no anvil; no tendencies; "column" is unchanged by the new knobs.
#[test]
fn anvil_geometry_column() {
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let ctx = geo.ctx(&planet, None, 1200.0);
    let s = DiagnosticClouds {
        convective: ConvectiveCloud::Anvil,
        cre_split: true,
        conv_water_path: 0.2,
        ..scheme()
    };
    let p = 30.0 / 86400.0;
    let run = |s: &DiagnosticClouds, top: usize| {
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = (300.0 - 80.0 * (1.0 - ctx.sigma[k])).max(200.0);
            st.q[k] = 0.3 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        st.conv_precip = p;
        st.conv_top = top;
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0; 3];
        s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        (sfc.cloud, diag)
    };
    let cc = s.convective(p);
    let ca = DiagnosticClouds::anvil(p);
    assert!(ca > 0.5);
    // deep: top in the upper troposphere
    let top = (0..26).find(|&k| ctx.sigma[k] >= 0.2).unwrap();
    let (m, d) = run(&s, top);
    assert!(m[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
    assert!(m[78..].iter().all(|&c| c == 0.0)); // no stratiform cloud
    for k in 0..26 {
        let sg = ctx.sigma[k];
        if k < top || k == 25 {
            assert_eq!(m[k], 0.0, "{k}");
        } else if k == top {
            let c = 1.0 - (1.0 - 0.25 * cc) * (1.0 - ca);
            assert!((m[k] - c).abs() < 1e-12, "anvil layer {k}: {}", m[k]);
            // thin: in-cloud water close to 0.035 g/m³ of the layer, far below the core's
            let w = m[26 + k] + m[52 + k];
            assert!(w < 0.05, "anvil water {w}");
        } else if sg >= 0.8 {
            let c = 1.0 - (1.0 - 0.25 * cc) * (1.0 - 0.75 * cc);
            assert!((m[k] - c).abs() < 1e-12, "{k}");
        } else {
            assert!((m[k] - 0.25 * cc).abs() < 1e-12, "{k}");
        }
    }
    assert!((d[1] - cc).abs() < 1e-12);
    assert!((d[2] - 0.2).abs() < 1e-9, "capped convective path {}", d[2]);
    // shallow: top at σ ≈ 0.6, C_c over the depth, no anvil
    let top_s = (0..26).find(|&k| ctx.sigma[k] >= 0.6).unwrap();
    let (m2, _) = run(&s, top_s);
    for k in top_s..25 {
        assert!((m2[k] - cc).abs() < 1e-12, "{k}");
    }
    assert_eq!(m2[top_s - 1], 0.0);
    // "column" ignores the anvil knobs
    let col = DiagnosticClouds {
        conv_water_path: 0.2,
        core_share: 0.5,
        ..scheme()
    };
    assert_eq!(run(&col, top).0, run(&scheme(), top).0);
}

/// Freeze-dry against its source (CAM `cldfrc`, corrected 2026-11-26): it scales the RH cloud
/// before the stratus is combined by maximum, so a stratus above the reduced RH cloud is kept
/// whole; the bound is a pressure, so a column with p_s = 700 hPa (all levels above 750 hPa)
/// is not reduced at all.
#[test]
fn freeze_dry_rh_cloud_only_and_by_pressure() {
    let s = scheme();
    let geo = Geometry::l26();
    let planet = crate::planet::Planet::earth();
    let mut ctx = geo.ctx(&planet, None, 1200.0);
    let (q, qs) = (1e-3, 1e-3 / 0.99);
    let k = 22;
    let rh_cloud = DiagnosticClouds::stratiform(0.99, s.rh_crit(ctx.sigma[k]));
    let fd = s.freeze_dry(q, ctx.p[k]);
    assert!(fd < 0.5 && ctx.p[k] > 75000.0);
    assert!((s.layer_cloud(&ctx, qs, q, k, 0.0, None) - rh_cloud * fd).abs() < 1e-12);
    // a stratus larger than the reduced RH cloud is not reduced (gated low cloud mode)
    let lts = DiagnosticClouds {
        low_cloud: LowCloud::SlingoLts,
        low_rh: LowRh::Sundqvist,
        ..scheme()
    };
    let c = lts.layer_cloud(&ctx, qs, q, k, 0.0, Some((k, 0.6, 20.0)));
    assert_eq!(c, 0.6);
    // high ground: p = σ · 700 hPa < 750 hPa at every level, no reduction
    let p: Vec<f64> = ctx.sigma.iter().map(|s| s * 70000.0).collect();
    ctx.p = &p;
    assert_eq!(s.freeze_dry(q, ctx.p[25]), 1.0);
    assert!((s.layer_cloud(&ctx, qs, q, k, 0.0, None) - rh_cloud).abs() < 1e-12);
}
