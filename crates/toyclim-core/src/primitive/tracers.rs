//! Tracer transport in the primitive-equation model: flux-form advection tendency and the
//! negative-value fixer.

use super::state::{map_levels, Grid3, Grids, Spec3};
use crate::physics::driver::PhysTend;
use crate::spectral::{Complex, Transform};
use crate::Real;
use rayon::prelude::*;

/// Tendency of tracer `it`: −∇·(v q) + q δ − σ̇ ∂q/∂σ + physics, written to `dst`.
/// `sdl[k]` is σ̇ at the half level below full level k.
pub(crate) fn tendency(
    tr: &Transform,
    g: &Grids,
    it: usize,
    sdl: &Grid3,
    ds: &[Real],
    phys: Option<&PhysTend>,
    dst: &mut Spec3,
) {
    let n = tr.ngrid();
    let q = &g.q[it];
    let l = q.nlev();
    let per: Vec<Vec<Complex>> = map_levels(l, |k| {
        let (mut fu, mut fv, mut rest) = (
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
        );
        for p in 0..n {
            fu[p] = g.u[k][p] * q[k][p];
            fv[p] = g.v[k][p] * q[k][p];
            let below = if k + 1 < l {
                sdl[k][p] * (q[k + 1][p] - q[k][p])
            } else {
                0.0
            };
            let above = if k > 0 {
                sdl[k - 1][p] * (q[k][p] - q[k - 1][p])
            } else {
                0.0
            };
            rest[p] = q[k][p] * g.div[k][p] - (below + above) / (2.0 * ds[k]);
            if let Some(ph) = phys {
                rest[p] += ph.dq[it][k][p];
            }
        }
        let fl = tr.div(&fu, &fv);
        tr.analysis(&rest)
            .iter()
            .zip(&fl)
            .map(|(a, b)| a - b)
            .collect()
    });
    for (k, x) in per.into_iter().enumerate() {
        dst[k].copy_from_slice(&x);
    }
}

/// Negative-value fixer: in grid space, set negative values to zero and scale the positive
/// values so that the global tracer mass ∫Σ_k q Δσ_k p_s/g dA is unchanged; then transform
/// back. Applied to the new time level after each step. Tracers without negative values
/// are left untouched.
pub(crate) fn fix_negative(tr: &Transform, lnps: &[Complex], ds: &[f64], tracers: &mut [Spec3]) {
    if tracers.is_empty() {
        return;
    }
    let n = tr.ngrid();
    let ps: Vec<Real> = tr.synthesis(lnps).iter().map(|x| x.exp()).collect();
    for tq in tracers.iter_mut() {
        let l = tq.nlev();
        let mut grids: Vec<Vec<Real>> = map_levels(l, |k| tr.synthesis(&tq[k]));
        if !grids.par_iter().any(|g| g.iter().any(|&x| x < 0.0)) {
            continue;
        }
        // Σ_k q Δσ_k p_s per point (levels summed in order), then the global integral.
        let column_mass = |g: &Vec<Vec<Real>>| -> f64 {
            let mut w = vec![0.0 as Real; n];
            w.par_chunks_mut(tr.nlon())
                .enumerate()
                .for_each(|(j, row)| {
                    let p0 = j * row.len();
                    for k in 0..l {
                        for (i, x) in row.iter_mut().enumerate() {
                            *x += g[k][p0 + i] * ds[k] as Real * ps[p0 + i];
                        }
                    }
                });
            tr.integrate(&w)
        };
        let before = column_mass(&grids);
        grids.par_iter_mut().for_each(|g| {
            for x in g.iter_mut() {
                if *x < 0.0 {
                    *x = 0.0;
                }
            }
        });
        let after = column_mass(&grids);
        let scale = if after > 0.0 && before > 0.0 {
            (before / after) as Real
        } else {
            0.0
        };
        grids.par_iter_mut().for_each(|g| {
            for x in g.iter_mut() {
                *x *= scale;
            }
        });
        let nspec = tq.level_len();
        tq.as_mut_slice()
            .par_chunks_mut(nspec)
            .enumerate()
            .for_each(|(k, x)| x.copy_from_slice(&tr.analysis(&grids[k])));
    }
}
