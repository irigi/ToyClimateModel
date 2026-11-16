//! Semi-implicit treatment of the gravity-wave terms (SPEC.md §5.3, Hoskins & Simmons 1975).

use super::state::{PeState, CZERO};
use super::vertical::SigmaLevels;
use crate::planet::Planet;
use crate::spectral::{Complex, Layout};
use crate::Real;
use rayon::prelude::*;

/// Matrices of the linearised system and the inverses S_ℓ⁻¹ per total wavenumber ℓ.
pub struct SemiImplicit {
    /// Hydrostatic matrix G (L×L, row-major).
    pub g: Vec<Real>,
    /// Linear temperature-tendency matrix τ (L×L, row-major).
    pub tau: Vec<Real>,
    /// S_ℓ⁻¹ for the leapfrog step (half-step Δt) per ℓ.
    sinv_leap: Vec<Vec<Real>>,
    /// S_ℓ⁻¹ for the first, forward step (half-step Δt/2) per ℓ.
    sinv_first: Vec<Vec<Real>>,
    /// R T̄.
    rt: Real,
    /// Δσ_k.
    w: Vec<Real>,
}

impl SemiImplicit {
    /// Precompute everything for time step `dt` \[s\] and truncation `mmax`.
    pub fn new(planet: &Planet, levels: &SigmaLevels, mmax: usize, dt: f64) -> Self {
        let l = levels.nlev;
        let r = planet.gas_constant;
        let kappa = planet.kappa();
        let tref = planet.t_ref;
        let g = levels.g_matrix(r);
        let tau = levels.tau_matrix(kappa, tref);
        let b = levels.b_matrix(r, kappa, tref);
        let a2 = planet.radius * planet.radius;
        let make_sinv = |dt: f64| -> Vec<Vec<Real>> {
            (0..=mmax)
                .map(|ell| {
                    let lam = (ell * (ell + 1)) as f64 / a2;
                    let mut s = vec![0.0; l * l];
                    for i in 0..l {
                        for j in 0..l {
                            s[i * l + j] =
                                dt * dt * lam * b[i * l + j] + if i == j { 1.0 } else { 0.0 };
                        }
                    }
                    crate::util::invert(l, &s)
                        .expect("semi-implicit matrix is singular")
                        .iter()
                        .map(|&x| x as Real)
                        .collect()
                })
                .collect()
        };
        SemiImplicit {
            g: g.iter().map(|&x| x as Real).collect(),
            tau: tau.iter().map(|&x| x as Real).collect(),
            sinv_leap: make_sinv(dt),
            sinv_first: make_sinv(0.5 * dt),
            rt: (planet.gas_constant * planet.t_ref) as Real,
            w: levels.dsigma.iter().map(|&x| x as Real).collect(),
        }
    }

    /// Semi-implicit update from `minus` with centre-level tendencies `n` into `out`, with
    /// half-step `dt` (Δt for leapfrog steps, Δt/2 for the first step, chosen by `first`).
    /// Every coefficient of `out` is overwritten. The coupled (δ, T', ln p_s) solve runs in
    /// parallel over spectral coefficients into `sol` (`[coefficient][δ | T' | ln p_s]`),
    /// then is copied to `out`; the arithmetic per coefficient does not depend on the
    /// thread count.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        lay: &Layout,
        radius: Real,
        minus: &PeState,
        n: &PeState,
        dt: Real,
        first: bool,
        out: &mut PeState,
        sol: &mut Vec<Complex>,
    ) {
        let l = self.w.len();
        let nspec = lay.len();
        let a2 = radius * radius;
        let sinv = if first {
            &self.sinv_first
        } else {
            &self.sinv_leap
        };
        let (rt, w) = (self.rt, &self.w);
        // Explicit parts: vorticity and tracers, elementwise.
        let explicit = |o: &mut [Complex], m: &[Complex], t: &[Complex]| {
            for ((x, a), b) in o.iter_mut().zip(m).zip(t) {
                *x = a + b * (2.0 * dt);
            }
        };
        out.vor
            .as_mut_slice()
            .par_chunks_mut(nspec)
            .zip(minus.vor.as_slice().par_chunks(nspec))
            .zip(n.vor.as_slice().par_chunks(nspec))
            .for_each(|((o, m), t)| explicit(o, m, t));
        for (it, tq) in out.tracers.iter_mut().enumerate() {
            tq.as_mut_slice()
                .par_chunks_mut(nspec)
                .zip(minus.tracers[it].as_slice().par_chunks(nspec))
                .zip(n.tracers[it].as_slice().par_chunks(nspec))
                .for_each(|((o, m), t)| explicit(o, m, t));
        }
        // Coupled solve per coefficient.
        let stride = 2 * l + 1;
        sol.resize(nspec * stride, CZERO);
        let solve = |(tt, rhs, x): &mut (Vec<Complex>, Vec<Complex>, Vec<Complex>),
                     (idx, o): (usize, &mut [Complex])| {
            let ell = lay.l_of[idx];
            let lam = (ell * (ell + 1)) as Real / a2;
            let si = &sinv[ell];
            for k in 0..l {
                tt[k] = minus.tmp[k][idx] + n.tmp[k][idx] * dt;
            }
            let pp = minus.lnps[idx] + n.lnps[idx] * dt;
            for k in 0..l {
                let mut gt = pp * rt;
                for j in k..l {
                    gt += tt[j] * self.g[k * l + j];
                }
                rhs[k] = minus.div[k][idx] + (n.div[k][idx] + gt * lam) * dt;
            }
            for k in 0..l {
                let mut acc = CZERO;
                for j in 0..l {
                    acc += rhs[j] * si[k * l + j];
                }
                x[k] = acc;
            }
            let mut wx = CZERO;
            for k in 0..l {
                wx += x[k] * w[k];
            }
            for k in 0..l {
                o[k] = x[k] * 2.0 - minus.div[k][idx];
                let mut tx = CZERO;
                for j in 0..=k {
                    tx += x[j] * self.tau[k * l + j];
                }
                o[l + k] = minus.tmp[k][idx] + (n.tmp[k][idx] - tx) * (2.0 * dt);
            }
            o[2 * l] = minus.lnps[idx] + (n.lnps[idx] - wx) * (2.0 * dt);
        };
        sol.par_chunks_mut(stride)
            .enumerate()
            .with_min_len(16)
            .for_each_init(|| (vec![CZERO; l], vec![CZERO; l], vec![CZERO; l]), solve);
        let sol = &*sol;
        out.div
            .as_mut_slice()
            .par_chunks_mut(nspec)
            .zip(out.tmp.as_mut_slice().par_chunks_mut(nspec))
            .enumerate()
            .for_each(|(k, (d, t))| {
                for idx in 0..nspec {
                    d[idx] = sol[idx * stride + k];
                    t[idx] = sol[idx * stride + l + k];
                }
            });
        for (idx, x) in out.lnps.iter_mut().enumerate() {
            *x = sol[idx * stride + 2 * l];
        }
    }
}
