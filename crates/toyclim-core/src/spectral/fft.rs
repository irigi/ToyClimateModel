//! Real FFT per latitude row (SPEC.md §4.3).
//!
//! Forward: F_m(μ_j) = (1/N_lon) Σ_k f(λ_k, μ_j) e^{−imλ_k}, m = 0..=M.
//! Inverse: f(λ_k) = Σ_{m=−M}^{M} F_m e^{imλ_k} (with F_{−m} = conj F_m).
//! Rows are processed in parallel; each row is independent, so the result does not
//! depend on the thread count.

use super::packing::Complex;
use crate::Real;
use rayon::prelude::*;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// FFT plans for one grid size.
pub struct RowFft {
    pub nlon: usize,
    pub mmax: usize,
    fwd: Arc<dyn RealToComplex<Real>>,
    inv: Arc<dyn ComplexToReal<Real>>,
}

impl RowFft {
    pub fn new(nlon: usize, mmax: usize) -> Self {
        assert!(2 * mmax < nlon, "need N_lon > 2M");
        let mut planner = RealFftPlanner::<Real>::new();
        RowFft {
            nlon,
            mmax,
            fwd: planner.plan_fft_forward(nlon),
            inv: planner.plan_fft_inverse(nlon),
        }
    }

    /// Grid `[lat][lon]` → Fourier `[lat][m]` (m = 0..=M).
    pub fn forward(&self, grid: &[Real], fourier: &mut [Complex]) {
        let (nlon, nm) = (self.nlon, self.mmax + 1);
        assert_eq!(grid.len() % nlon, 0);
        assert_eq!(fourier.len(), grid.len() / nlon * nm);
        let scale = 1.0 / nlon as Real;
        grid.par_chunks(nlon)
            .zip(fourier.par_chunks_mut(nm))
            .with_min_len(4)
            .for_each_init(
                || {
                    (
                        self.fwd.make_input_vec(),
                        self.fwd.make_output_vec(),
                        self.fwd.make_scratch_vec(),
                    )
                },
                |(inp, out, scratch), (row, frow)| {
                    inp.copy_from_slice(row);
                    self.fwd
                        .process_with_scratch(inp, out, scratch)
                        .expect("fft size");
                    for (f, o) in frow.iter_mut().zip(out.iter()) {
                        *f = *o * scale;
                    }
                },
            );
    }

    /// Fourier `[lat][m]` → grid `[lat][lon]`.
    pub fn inverse(&self, fourier: &[Complex], grid: &mut [Real]) {
        let (nlon, nm) = (self.nlon, self.mmax + 1);
        assert_eq!(fourier.len() % nm, 0);
        assert_eq!(grid.len(), fourier.len() / nm * nlon);
        fourier
            .par_chunks(nm)
            .zip(grid.par_chunks_mut(nlon))
            .with_min_len(4)
            .for_each_init(
                || (self.inv.make_input_vec(), self.inv.make_scratch_vec()),
                |(spec, scratch), (frow, row)| {
                    spec.iter_mut().for_each(|c| *c = Complex::new(0.0, 0.0));
                    spec[..nm].copy_from_slice(frow);
                    spec[0].im = 0.0;
                    if nlon.is_multiple_of(2) {
                        spec[nlon / 2].im = 0.0;
                    }
                    self.inv
                        .process_with_scratch(spec, row, scratch)
                        .expect("fft size");
                },
            );
    }
}
