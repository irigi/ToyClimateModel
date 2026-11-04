//! M1 acceptance tests 3–6 (SPEC.md §8).

use super::ops::{inv_laplacian, laplacian, solid_body_psi};
use super::{Complex, Layout, Transform};
use crate::util::SplitMix64;
use crate::Real;

const A: f64 = 6.37122e6;

fn random_spec(lay: &Layout, seed: u64) -> Vec<Complex> {
    let mut rng = SplitMix64(seed);
    (0..lay.len())
        .map(|k| {
            let im = if lay.m_of[k] == 0 {
                0.0
            } else {
                rng.symmetric()
            };
            Complex::new(rng.symmetric() as Real, im as Real)
        })
        .collect()
}

fn max_rel_err(a: &[Complex], b: &[Complex]) -> f64 {
    let scale = b.iter().map(|c| c.norm() as f64).fold(0.0, f64::max);
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).norm() as f64)
        .fold(0.0, f64::max)
        / scale
}

/// Test 3: analysis(synthesis(f)) = f.
#[test]
fn round_trip() {
    for t in [21, 42, 85, 170] {
        let tr = Transform::new(t, A);
        let f = random_spec(&tr.layout, t as u64);
        let g = tr.synthesis(&f);
        let f2 = tr.analysis(&g);
        let e = max_rel_err(&f2, &f);
        eprintln!("M1.3 round trip T{t}: {e:.2e}");
        assert!(e < 1e-12, "T{t}: {e:e}");
    }
}

/// Test 4: ∇² of a synthesized Y_ℓ^m via the gradient and its divergence.
#[test]
fn laplacian_via_grad_div() {
    let tr = Transform::new(42, A);
    let lay = &tr.layout;
    for (l, m) in [(1, 0), (5, 3), (20, 0), (30, 17), (42, 42), (41, 40)] {
        let mut f = lay.zeros();
        f[lay.idx(l, m)] = Complex::new(1.0, if m > 0 { 0.5 } else { 0.0 });
        let (gx, gy) = tr.grad(&f);
        let lap = tr.div(&gx, &gy);
        let exact = laplacian(lay, &f, A as Real);
        let e = max_rel_err(&lap, &exact);
        eprintln!("M1.4 laplacian l={l} m={m}: {e:.2e}");
        assert!(e < 1e-10, "l={l} m={m}: {e:e}");
    }
}

/// Test 5: ψ, χ → U, V → curl, div gives ∇²ψ, ∇²χ.
#[test]
fn wind_round_trip() {
    for t in [21, 42, 85] {
        let tr = Transform::new(t, A);
        let lay = &tr.layout;
        let mut psi = random_spec(lay, 7 + t as u64);
        let mut chi = random_spec(lay, 11 + t as u64);
        psi[0] = Complex::new(0.0, 0.0);
        chi[0] = Complex::new(0.0, 0.0);
        let (u, v) = tr.uv_from_psi_chi(&psi, &chi);
        let (d, c) = tr.div_curl(&u, &v);
        let ed = max_rel_err(&d, &laplacian(lay, &chi, A as Real));
        let ec = max_rel_err(&c, &laplacian(lay, &psi, A as Real));
        eprintln!("M1.5 wind T{t}: div {ed:.2e} curl {ec:.2e}");
        assert!(ed < 1e-10 && ec < 1e-10, "T{t}: div {ed:e} curl {ec:e}");
        // and back through the inverse Laplacian
        let (u2, v2) = tr.uv_from_vort_div(&c, &d);
        let eu = u
            .iter()
            .zip(&u2)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, Real::max);
        let umax = u.iter().map(|x| x.abs()).fold(0.0, Real::max);
        assert!((eu / umax) < 1e-10);
        let _ = (v2, inv_laplacian(lay, &c, A as Real));
    }
}

/// Test 6: ψ = −a u₀ sin φ gives u = u₀ cos φ, v = 0.
#[test]
fn solid_body_rotation() {
    let tr = Transform::new(42, A);
    let u0 = 38.61;
    let psi = solid_body_psi(&tr.layout, u0, A as Real);
    let (u, v) = tr.uv_from_psi_chi(&psi, &tr.spec_zeros());
    let nlon = tr.nlon();
    let mut worst = 0.0f64;
    for j in 0..tr.nlat() {
        let cos2 = 1.0 - tr.grid.mu[j] * tr.grid.mu[j];
        for k in 0..nlon {
            // U = u cos φ = u₀ cos² φ
            let e_u = (u[j * nlon + k] as f64 - u0 as f64 * cos2).abs();
            worst = worst.max(e_u).max((v[j * nlon + k] as f64).abs());
        }
    }
    let rel = worst / u0 as f64;
    eprintln!("M1.6 solid body: {rel:.2e}");
    assert!(rel < 1e-12, "{worst:e}");
}

/// Determinism: identical results for 1 and N threads.
#[test]
fn thread_count_independent() {
    let tr = Transform::new(42, A);
    let f = random_spec(&tr.layout, 99);
    let pool1 = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let pool4 = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let run = || {
        let (u, v) = tr.uv_from_psi_chi(&f, &f);
        tr.div_curl(&u, &v)
    };
    let a = pool1.install(run);
    let b = pool4.install(run);
    assert!(a
        .0
        .iter()
        .zip(&b.0)
        .all(|(x, y)| x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits()));
    assert!(a
        .1
        .iter()
        .zip(&b.1)
        .all(|(x, y)| x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits()));
}
