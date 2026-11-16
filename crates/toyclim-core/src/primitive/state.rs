//! Storage of the primitive-equation model: flat `[lev][point]` containers, the spectral
//! state of one time level, and its grid-point representation.

use crate::spectral::{Complex, Transform};
use crate::Real;
use rayon::prelude::*;
use std::ops::{Index, IndexMut};

pub(crate) const CZERO: Complex = Complex { re: 0.0, im: 0.0 };

/// A field with `nlev` levels of `n` values each, stored contiguously as `[lev][point]`.
/// `x[k]` is the slice of level k, so kernels index it as `x[k][p]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Field3<T> {
    data: Vec<T>,
    n: usize,
}

impl<T: Copy> Field3<T> {
    /// `nlev` levels of `n` copies of `value`.
    pub fn filled(nlev: usize, n: usize, value: T) -> Self {
        Field3 {
            data: vec![value; nlev * n],
            n,
        }
    }
    /// Concatenate per-level vectors (all of the same length).
    pub fn from_levels(levels: Vec<Vec<T>>) -> Self {
        let n = levels.first().map_or(0, |l| l.len());
        debug_assert!(levels.iter().all(|l| l.len() == n));
        Field3 {
            data: levels.concat(),
            n,
        }
    }
    /// Number of levels.
    pub fn nlev(&self) -> usize {
        self.data.len().checked_div(self.n).unwrap_or(0)
    }
    /// Values per level.
    pub fn level_len(&self) -> usize {
        self.n
    }
    /// All values, level after level.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }
    /// Iterator over the levels.
    pub fn levels(&self) -> std::slice::Chunks<'_, T> {
        self.data.chunks(self.n.max(1))
    }
    /// Mutable iterator over the levels.
    pub fn levels_mut(&mut self) -> std::slice::ChunksMut<'_, T> {
        self.data.chunks_mut(self.n.max(1))
    }
    /// Set every value.
    pub fn fill(&mut self, value: T) {
        self.data.fill(value);
    }
}

impl<T> Index<usize> for Field3<T> {
    type Output = [T];
    fn index(&self, k: usize) -> &[T] {
        &self.data[k * self.n..(k + 1) * self.n]
    }
}

impl<T> IndexMut<usize> for Field3<T> {
    fn index_mut(&mut self, k: usize) -> &mut [T] {
        &mut self.data[k * self.n..(k + 1) * self.n]
    }
}

/// Spectral coefficients per level.
pub type Spec3 = Field3<Complex>;
/// Grid-point values per level.
pub type Grid3 = Field3<Real>;

/// One time level of the spectral state.
#[derive(Debug, Clone, PartialEq)]
pub struct PeState {
    pub vor: Spec3,
    pub div: Spec3,
    /// T' = T − T̄ \[K\].
    pub tmp: Spec3,
    /// ln(p_s / 1 Pa).
    pub lnps: Vec<Complex>,
    /// Tracers, one `[lev][spec]` field each (e.g. specific humidity q \[kg/kg\]).
    pub tracers: Vec<Spec3>,
}

impl PeState {
    pub fn zeros(nlev: usize, nspec: usize) -> Self {
        Self::zeros_with_tracers(nlev, nspec, 0)
    }
    pub fn zeros_with_tracers(nlev: usize, nspec: usize, ntracers: usize) -> Self {
        let z = Spec3::filled(nlev, nspec, CZERO);
        PeState {
            vor: z.clone(),
            div: z.clone(),
            tracers: vec![z.clone(); ntracers],
            tmp: z,
            lnps: vec![CZERO; nspec],
        }
    }
    /// Same number of levels, coefficients and tracers as `other`.
    pub fn same_shape(&self, other: &PeState) -> bool {
        self.vor.as_slice().len() == other.vor.as_slice().len()
            && self.vor.level_len() == other.vor.level_len()
            && self.tracers.len() == other.tracers.len()
    }
    pub(crate) fn is_finite(&self) -> bool {
        let ok = |v: &[Complex]| v.iter().all(|c| c.re.is_finite() && c.im.is_finite());
        ok(self.vor.as_slice())
            && ok(self.div.as_slice())
            && ok(self.tmp.as_slice())
            && self.tracers.iter().all(|t| ok(t.as_slice()))
            && ok(&self.lnps)
    }
}

/// Initial condition from a test case.
pub struct PeInitial {
    pub state: PeState,
    /// Surface geopotential on the grid \[m² s⁻²\].
    pub phis_grid: Vec<Real>,
}

/// Grid-point fields of one state.
pub(crate) struct Grids {
    /// cos φ-weighted winds U, V per level.
    pub u: Grid3,
    pub v: Grid3,
    pub vor: Grid3,
    pub div: Grid3,
    /// T' per level.
    pub t: Grid3,
    pub lnps: Vec<Real>,
    /// cos φ-weighted gradient of ln p_s.
    pub px: Vec<Real>,
    pub py: Vec<Real>,
    /// Tracers, one `[lev][point]` field each.
    pub q: Vec<Grid3>,
}

/// Map over levels, in parallel when more than one thread is available. Each level is
/// computed identically either way (deterministic).
pub(crate) fn map_levels<T: Send>(nlev: usize, f: impl Fn(usize) -> T + Sync + Send) -> Vec<T> {
    if rayon::current_num_threads() > 1 {
        (0..nlev).into_par_iter().map(f).collect()
    } else {
        (0..nlev).map(f).collect()
    }
}

impl Grids {
    /// Zero-filled buffers for `nlev` levels, `n` points and `ntracers` tracers.
    pub fn new(nlev: usize, n: usize, ntracers: usize) -> Self {
        let z = Grid3::filled(nlev, n, 0.0);
        Grids {
            u: z.clone(),
            v: z.clone(),
            vor: z.clone(),
            div: z.clone(),
            q: vec![z.clone(); ntracers],
            t: z,
            lnps: vec![0.0; n],
            px: vec![0.0; n],
            py: vec![0.0; n],
        }
    }

    /// Synthesize all grid-point fields of `s` into these buffers.
    pub fn fill(&mut self, tr: &Transform, s: &PeState) {
        self.fill_with(tr, s, true);
    }

    /// Synthesize the grid-point fields of `s`; vorticity and divergence only if
    /// `vor_div` (the physics does not need them). Levels are done in parallel, each
    /// writing straight into its slice of the flat buffers.
    pub fn fill_with(&mut self, tr: &Transform, s: &PeState, vor_div: bool) {
        let l = s.vor.nlev();
        let n = tr.ngrid();
        self.q
            .resize_with(s.tracers.len(), || Grid3::filled(l, n, 0.0));
        let Grids {
            u,
            v,
            vor,
            div,
            t,
            lnps,
            px,
            py,
            q,
        } = self;
        let dyn_levels = |k: usize,
                          u: &mut [Real],
                          v: &mut [Real],
                          z: &mut [Real],
                          d: &mut [Real],
                          t: &mut [Real]| {
            let (gu, gv) = tr.uv_from_vort_div(&s.vor[k], &s.div[k]);
            u.copy_from_slice(&gu);
            v.copy_from_slice(&gv);
            if vor_div {
                z.copy_from_slice(&tr.synthesis(&s.vor[k]));
                d.copy_from_slice(&tr.synthesis(&s.div[k]));
            }
            t.copy_from_slice(&tr.synthesis(&s.tmp[k]));
        };
        let surface = || {
            let (gx, gy) = tr.grad(&s.lnps);
            (gx, gy, tr.synthesis(&s.lnps))
        };
        let tracers = |q: &mut Vec<Grid3>| {
            for (gq, tq) in q.iter_mut().zip(&s.tracers) {
                gq.as_mut_slice()
                    .par_chunks_mut(n)
                    .enumerate()
                    .for_each(|(k, x)| x.copy_from_slice(&tr.synthesis(&tq[k])));
            }
        };
        let ((gx, gy, gl), ()) = rayon::join(surface, || {
            u.as_mut_slice()
                .par_chunks_mut(n)
                .zip(v.as_mut_slice().par_chunks_mut(n))
                .zip(vor.as_mut_slice().par_chunks_mut(n))
                .zip(div.as_mut_slice().par_chunks_mut(n))
                .zip(t.as_mut_slice().par_chunks_mut(n))
                .enumerate()
                .for_each(|(k, ((((u, v), z), d), t))| dyn_levels(k, u, v, z, d, t));
            tracers(q);
        });
        px.copy_from_slice(&gx);
        py.copy_from_slice(&gy);
        lnps.copy_from_slice(&gl);
    }
}

/// Synthesize all grid-point fields of `s` into new buffers.
pub(crate) fn grids(tr: &Transform, s: &PeState) -> Grids {
    let mut g = Grids::new(s.vor.nlev(), tr.ngrid(), s.tracers.len());
    g.fill(tr, s);
    g
}
