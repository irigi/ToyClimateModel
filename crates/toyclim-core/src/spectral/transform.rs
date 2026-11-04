//! Spectral transforms on the Gaussian grid (SPEC.md §4.3–4.4).
//!
//! Grid fields are `[lat][lon]` row-major with row 0 the northernmost latitude.
//! Spectral fields use the [`Layout`] packing. The Legendre step is parallel over m and
//! exploits the equatorial symmetry P̄_ℓ^m(−μ) = (−1)^{ℓ+m} P̄_ℓ^m(μ),
//! H_ℓ^m(−μ) = −(−1)^{ℓ+m} H_ℓ^m(μ). All sums run in a fixed order, so results are
//! bitwise independent of the thread count.

use super::fft::RowFft;
use super::legendre::pbar_and_h;
use super::packing::{Complex, Layout};
use crate::grid::GaussianGrid;
use crate::Real;
use rayon::prelude::*;

const ZERO: Complex = Complex { re: 0.0, im: 0.0 };

/// out_i += (i even ? e : o) · t_i.
#[inline]
fn pair_axpy(out: &mut [Complex], e: Complex, o: Complex, t: &[Real]) {
    let mut oc = out.chunks_exact_mut(2);
    let tc = t.chunks_exact(2);
    let tr = tc.remainder();
    for (x, y) in (&mut oc).zip(tc) {
        x[0].re += e.re * y[0];
        x[0].im += e.im * y[0];
        x[1].re += o.re * y[1];
        x[1].im += o.im * y[1];
    }
    if let (Some(x), Some(y)) = (oc.into_remainder().first_mut(), tr.first()) {
        x.re += e.re * y;
        x.im += e.im * y;
    }
}

/// Precomputed tables and plans for one truncation, grid and planet radius.
pub struct Transform {
    pub layout: Layout,
    pub grid: GaussianGrid,
    /// Planet radius a \[m\] used by the vector operators.
    pub radius: Real,
    nhalf: usize,
    /// P̄ per m: `[j][ℓ−m]` for the northern rows j < nhalf.
    p: Vec<Vec<Real>>,
    /// H per m, same layout.
    h: Vec<Vec<Real>>,
    /// P̄ and H per m transposed to `[ℓ−m][j]` (latitude innermost, for synthesis).
    pt: Vec<Vec<Real>>,
    ht: Vec<Vec<Real>>,
    /// Gaussian weight per northern row.
    w: Vec<Real>,
    /// Gaussian weight / (a (1 − μ²)) per northern row.
    w_div: Vec<Real>,
    /// 1/(1 − μ²) per row (all rows).
    pub inv_cos2: Vec<Real>,
    fft: RowFft,
}

/// Fourier coefficients `[lat][m]`, m = 0..=M.
pub type Fourier = Vec<Complex>;

impl Transform {
    /// Build tables for truncation T`mmax` on its alias-free Gaussian grid.
    pub fn new(mmax: usize, radius: f64) -> Self {
        Self::with_grid(mmax, GaussianGrid::for_truncation(mmax), radius)
    }

    /// Build tables for truncation T`mmax` on a given grid (N_lat even, N_lon > 2M).
    pub fn with_grid(mmax: usize, grid: GaussianGrid, radius: f64) -> Self {
        assert!(grid.nlat.is_multiple_of(2), "N_lat must be even");
        let nhalf = grid.nlat / 2;
        let layout = Layout::new(mmax);
        let (p, h): (Vec<Vec<Real>>, Vec<Vec<Real>>) = (0..=mmax)
            .into_par_iter()
            .map(|m| {
                let nl = mmax + 1 - m;
                let mut pm = Vec::with_capacity(nhalf * nl);
                let mut hm = Vec::with_capacity(nhalf * nl);
                for j in 0..nhalf {
                    let (pc, hc) = pbar_and_h(m, mmax, grid.mu[j]);
                    pm.extend(pc.iter().map(|&x| x as Real));
                    hm.extend(hc.iter().map(|&x| x as Real));
                }
                (pm, hm)
            })
            .unzip();
        let transpose = |tab: &Vec<Vec<Real>>| -> Vec<Vec<Real>> {
            tab.iter()
                .enumerate()
                .map(|(m, t)| {
                    let nl = mmax + 1 - m;
                    let mut o = vec![0.0 as Real; nl * nhalf];
                    for j in 0..nhalf {
                        for i in 0..nl {
                            o[i * nhalf + j] = t[j * nl + i];
                        }
                    }
                    o
                })
                .collect()
        };
        let pt = transpose(&p);
        let ht = transpose(&h);
        let w = grid.weights[..nhalf].iter().map(|&x| x as Real).collect();
        let w_div = (0..nhalf)
            .map(|j| (grid.weights[j] / (radius * (1.0 - grid.mu[j] * grid.mu[j]))) as Real)
            .collect();
        let inv_cos2 = grid
            .mu
            .iter()
            .map(|m| (1.0 / (1.0 - m * m)) as Real)
            .collect();
        let fft = RowFft::new(grid.nlon, mmax);
        Transform {
            layout,
            grid,
            radius: radius as Real,
            nhalf,
            p,
            h,
            pt,
            ht,
            w,
            w_div,
            inv_cos2,
            fft,
        }
    }

    pub fn mmax(&self) -> usize {
        self.layout.mmax
    }
    pub fn nlat(&self) -> usize {
        self.grid.nlat
    }
    pub fn nlon(&self) -> usize {
        self.grid.nlon
    }
    /// Number of grid points.
    pub fn ngrid(&self) -> usize {
        self.grid.nlat * self.grid.nlon
    }
    /// A zero grid field.
    pub fn grid_zeros(&self) -> Vec<Real> {
        vec![0.0; self.ngrid()]
    }
    /// A zero spectral field.
    pub fn spec_zeros(&self) -> Vec<Complex> {
        self.layout.zeros()
    }

    // ---------------------------------------------------------------- Legendre

    /// Map over m = 0..=M, in parallel only for grids large enough to profit. Each m is
    /// computed identically either way, so results do not depend on the thread count.
    fn map_m<T: Send>(&self, f: impl Fn(usize) -> T + Sync + Send) -> Vec<T> {
        let m = self.mmax();
        if self.ngrid() >= super::fft::PARALLEL_MIN_POINTS && rayon::current_num_threads() > 1 {
            (0..=m).into_par_iter().map(f).collect()
        } else {
            (0..=m).map(f).collect()
        }
    }

    /// F_m(μ_j) = Σ_ℓ (a_ℓ^m P̄_ℓ^m(μ_j) + b_ℓ^m H_ℓ^m(μ_j)).
    fn legendre_synthesis(&self, a: Option<&[Complex]>, b: Option<&[Complex]>) -> Fourier {
        let mmax = self.mmax();
        let nm = mmax + 1;
        let nlat = self.nlat();
        let nhalf = self.nhalf;
        let cols: Vec<Vec<Complex>> = self.map_m(|m| {
            let r = self.layout.m_range(m);
            let nl = r.len();
            // sym/anti parts per northern row, real and imaginary separately so the
            // inner loops over latitude vectorize (no reductions).
            let mut sr = vec![0.0 as Real; nhalf];
            let mut si = vec![0.0 as Real; nhalf];
            let mut ar = vec![0.0 as Real; nhalf];
            let mut ai = vec![0.0 as Real; nhalf];
            let mut accumulate = |coef: &[Complex], tab: &[Real], even_to_sym: bool| {
                for i in 0..nl {
                    let c = coef[i];
                    if c.re == 0.0 && c.im == 0.0 {
                        continue;
                    }
                    let row = &tab[i * nhalf..(i + 1) * nhalf];
                    let to_sym = (i % 2 == 0) == even_to_sym;
                    let (dr, di) = if to_sym {
                        (&mut sr, &mut si)
                    } else {
                        (&mut ar, &mut ai)
                    };
                    for ((x, y), &t) in dr.iter_mut().zip(di.iter_mut()).zip(row) {
                        *x += c.re * t;
                        *y += c.im * t;
                    }
                }
            };
            if let Some(a) = a {
                accumulate(&a[r.clone()], &self.pt[m], true);
            }
            if let Some(b) = b {
                accumulate(&b[r.clone()], &self.ht[m], false);
            }
            let mut col = vec![ZERO; nlat];
            for j in 0..nhalf {
                let (sy, an) = (Complex::new(sr[j], si[j]), Complex::new(ar[j], ai[j]));
                col[j] = sy + an;
                col[nlat - 1 - j] = sy - an;
            }
            col
        });
        let mut f = vec![ZERO; nlat * nm];
        for (m, col) in cols.iter().enumerate() {
            for j in 0..nlat {
                f[j * nm + m] = col[j];
            }
        }
        f
    }

    /// f_ℓ^m = Σ_j s_j [X_m(μ_j) P̄_ℓ^m(μ_j) + Y_m(μ_j) H_ℓ^m(μ_j)], s_j a symmetric row scale.
    fn legendre_analysis(
        &self,
        x: Option<&Fourier>,
        y: Option<&Fourier>,
        scale: &[Real],
    ) -> Vec<Complex> {
        let mmax = self.mmax();
        let nm = mmax + 1;
        let nlat = self.nlat();
        let nhalf = self.nhalf;
        let blocks: Vec<Vec<Complex>> = self.map_m(|m| {
            let nl = mmax + 1 - m;
            let (pm, hm) = (&self.p[m], &self.h[m]);
            let mut out = vec![ZERO; nl];
            for j in 0..nhalf {
                let s = nlat - 1 - j;
                let pj = &pm[j * nl..(j + 1) * nl];
                let hj = &hm[j * nl..(j + 1) * nl];
                let sc = scale[j];
                if let Some(x) = x {
                    let (xn, xs) = (x[j * nm + m], x[s * nm + m]);
                    pair_axpy(&mut out, (xn + xs) * sc, (xn - xs) * sc, pj);
                }
                if let Some(y) = y {
                    let (yn, ys) = (y[j * nm + m], y[s * nm + m]);
                    pair_axpy(&mut out, (yn - ys) * sc, (yn + ys) * sc, hj);
                }
            }
            out
        });
        blocks.concat()
    }

    /// Multiply each Fourier coefficient by i m.
    fn times_im(&self, f: &mut Fourier) {
        let nm = self.mmax() + 1;
        for row in f.chunks_mut(nm) {
            for (m, c) in row.iter_mut().enumerate() {
                *c = Complex::new(-c.im, c.re) * (m as Real);
            }
        }
    }

    fn spec_times_im(&self, f: &[Complex], scale: Real) -> Vec<Complex> {
        f.iter()
            .zip(&self.layout.m_of)
            .map(|(c, &m)| Complex::new(-c.im, c.re) * (m as Real * scale))
            .collect()
    }

    fn to_grid(&self, f: &Fourier) -> Vec<Real> {
        let mut g = self.grid_zeros();
        self.fft.inverse(f, &mut g);
        g
    }

    /// Grid → Fourier `[lat][m]`.
    pub fn fourier(&self, grid: &[Real]) -> Fourier {
        let mut f = vec![ZERO; self.nlat() * (self.mmax() + 1)];
        self.fft.forward(grid, &mut f);
        f
    }

    // ---------------------------------------------------------------- public

    /// Spectral → grid.
    pub fn synthesis(&self, spec: &[Complex]) -> Vec<Real> {
        self.to_grid(&self.legendre_synthesis(Some(spec), None))
    }

    /// Grid → spectral.
    pub fn analysis(&self, grid: &[Real]) -> Vec<Complex> {
        let f = self.fourier(grid);
        let mut out = self.legendre_analysis(Some(&f), None, &self.w);
        // Real field: the m = 0 coefficients are real.
        for c in &mut out[self.layout.m_range(0)] {
            c.im = 0.0;
        }
        out
    }

    /// cos φ-weighted gradient of a scalar: ((1/a) ∂X/∂λ, (1/a)(1−μ²) ∂X/∂μ) on the grid.
    pub fn grad(&self, spec: &[Complex]) -> (Vec<Real>, Vec<Real>) {
        let ia = 1.0 / self.radius;
        let a = self.spec_times_im(spec, ia);
        let b: Vec<Complex> = spec.iter().map(|c| c * ia).collect();
        (
            self.to_grid(&self.legendre_synthesis(Some(&a), None)),
            self.to_grid(&self.legendre_synthesis(None, Some(&b))),
        )
    }

    /// U = u cos φ, V = v cos φ from streamfunction ψ and velocity potential χ.
    pub fn uv_from_psi_chi(&self, psi: &[Complex], chi: &[Complex]) -> (Vec<Real>, Vec<Real>) {
        let ia = 1.0 / self.radius;
        let im_chi = self.spec_times_im(chi, ia);
        let im_psi = self.spec_times_im(psi, ia);
        let mpsi: Vec<Complex> = psi.iter().map(|c| c * -ia).collect();
        let chi_a: Vec<Complex> = chi.iter().map(|c| c * ia).collect();
        (
            self.to_grid(&self.legendre_synthesis(Some(&im_chi), Some(&mpsi))),
            self.to_grid(&self.legendre_synthesis(Some(&im_psi), Some(&chi_a))),
        )
    }

    /// U, V from vorticity ζ and divergence δ (ψ = ∇⁻²ζ, χ = ∇⁻²δ).
    pub fn uv_from_vort_div(&self, vor: &[Complex], div: &[Complex]) -> (Vec<Real>, Vec<Real>) {
        let psi = super::ops::inv_laplacian(&self.layout, vor, self.radius);
        let chi = super::ops::inv_laplacian(&self.layout, div, self.radius);
        self.uv_from_psi_chi(&psi, &chi)
    }

    /// Divergence D and curl C of a cos φ-weighted vector (A, B) given on the grid:
    /// D = (1/(a(1−μ²))) [∂A/∂λ + (1−μ²) ∂B/∂μ], C = (1/(a(1−μ²))) [∂B/∂λ − (1−μ²) ∂A/∂μ].
    pub fn div_curl(&self, a: &[Real], b: &[Real]) -> (Vec<Complex>, Vec<Complex>) {
        let fa = self.fourier(a);
        let fb = self.fourier(b);
        (self.div_f(&fa, &fb), self.curl_f(&fa, &fb))
    }

    /// Divergence only (see [`Self::div_curl`]).
    pub fn div(&self, a: &[Real], b: &[Real]) -> Vec<Complex> {
        self.div_f(&self.fourier(a), &self.fourier(b))
    }

    /// Curl only (see [`Self::div_curl`]).
    pub fn curl(&self, a: &[Real], b: &[Real]) -> Vec<Complex> {
        self.curl_f(&self.fourier(a), &self.fourier(b))
    }

    fn div_f(&self, fa: &Fourier, fb: &Fourier) -> Vec<Complex> {
        let mut x = fa.clone();
        self.times_im(&mut x);
        let y: Fourier = fb.iter().map(|c| -c).collect();
        let mut out = self.legendre_analysis(Some(&x), Some(&y), &self.w_div);
        for c in &mut out[self.layout.m_range(0)] {
            c.im = 0.0;
        }
        out
    }

    fn curl_f(&self, fa: &Fourier, fb: &Fourier) -> Vec<Complex> {
        let mut x = fb.clone();
        self.times_im(&mut x);
        let mut out = self.legendre_analysis(Some(&x), Some(fa), &self.w_div);
        for c in &mut out[self.layout.m_range(0)] {
            c.im = 0.0;
        }
        out
    }

    /// Global integral ∫X dA \[unit(X)·m²\] of a grid field, summed in a fixed order.
    pub fn integrate(&self, grid: &[Real]) -> f64 {
        let nlon = self.nlon();
        let mut total = 0.0f64;
        for j in 0..self.nlat() {
            let row = &grid[j * nlon..(j + 1) * nlon];
            let mean: f64 = row.iter().map(|&x| x as f64).sum::<f64>() / nlon as f64;
            total += self.grid.weights[j] * mean;
        }
        let a = self.radius as f64;
        2.0 * std::f64::consts::PI * a * a * total
    }
}
