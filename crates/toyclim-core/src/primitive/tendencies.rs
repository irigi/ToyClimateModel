//! Explicit (nonlinear) dynamics tendencies of the primitive equations (SPEC.md §5.2):
//! the grid-point kernel per latitude row and the spectral assembly.

use super::implicit::SemiImplicit;
use super::state::{map_levels, Grid3, Grids, PeState};
use super::tracers;
use super::vertical::SigmaLevels;
use crate::physics::driver::PhysTend;
use crate::planet::Planet;
use crate::spectral::ops::laplacian;
use crate::spectral::{Complex, Transform};
use crate::Real;
use rayon::prelude::*;

/// Vertical and thermodynamic constants used by the column kernel.
pub(crate) struct ColumnConsts {
    ds: Vec<Real>,
    sh: Vec<Real>,
    alpha: Vec<Real>,
    lnr: Vec<Real>,
    r: Real,
    kappa: Real,
    tref: Real,
}

impl ColumnConsts {
    pub fn new(levels: &SigmaLevels, planet: &Planet) -> Self {
        ColumnConsts {
            ds: levels.dsigma.iter().map(|&x| x as Real).collect(),
            sh: levels.half.iter().map(|&x| x as Real).collect(),
            alpha: levels.alpha.iter().map(|&x| x as Real).collect(),
            lnr: levels.lnratio.iter().map(|&x| x as Real).collect(),
            r: planet.gas_constant as Real,
            kappa: planet.kappa() as Real,
            tref: planet.t_ref as Real,
        }
    }
}

/// Grid-point buffers of the explicit tendencies, reused every step.
pub(crate) struct DynWork {
    /// Grid fields of the current state.
    pub grids: Grids,
    qu: Grid3,
    qv: Grid3,
    ke: Grid3,
    tu: Grid3,
    tv: Grid3,
    trest: Grid3,
    /// σ̇ at the half level below each full level, σ̇_{k+1/2}.
    sdl: Grid3,
    np: Vec<Real>,
}

impl DynWork {
    pub fn new(nlev: usize, n: usize, ntracers: usize) -> Self {
        let z = Grid3::filled(nlev, n, 0.0);
        DynWork {
            grids: Grids::new(nlev, n, ntracers),
            qu: z.clone(),
            qv: z.clone(),
            ke: z.clone(),
            tu: z.clone(),
            tv: z.clone(),
            trest: z.clone(),
            sdl: z,
            np: vec![0.0; n],
        }
    }
}

/// Per-row scratch of the column kernel (one per rayon job).
struct RowScratch {
    /// v·∇ln p_s per level.
    vg: Vec<Real>,
    /// D_k.
    dd: Vec<Real>,
    /// Σ_{i≤k} Δσ_i D_i.
    cum: Vec<Real>,
    /// σ̇ at half levels 0..=L (top and bottom stay zero).
    sdot: Vec<Real>,
}

impl RowScratch {
    fn new(l: usize, nlon: usize) -> Self {
        RowScratch {
            vg: vec![0.0; l * nlon],
            dd: vec![0.0; l * nlon],
            cum: vec![0.0; l * nlon],
            sdot: vec![0.0; (l + 1) * nlon],
        }
    }
}

/// Mutable per-row output slices: [qu, qv, ke, tu, tv, trest, σ̇_{k+1/2}] × levels, and N_p.
#[derive(Default)]
struct RowOut<'a> {
    fields: [Vec<&'a mut [Real]>; 7],
    np: Option<&'a mut [Real]>,
}

#[allow(clippy::too_many_arguments)]
/// Grid-point part of the explicit tendencies for one latitude row (all levels):
/// D_k, σ̇, ω/p, vertical advection, Q_k components, kinetic energy, temperature fluxes.
/// Loops run over longitude innermost (contiguous). Returns max |u|/cos φ (for Courant).
fn column_row(
    g: &Grids,
    c: &ColumnConsts,
    j: usize,
    nlon: usize,
    l: usize,
    ic2: Real,
    f: Real,
    sc: &mut RowScratch,
    mut row: RowOut,
) -> f64 {
    let r0 = j * nlon;
    let rng = r0..r0 + nlon;
    let px = &g.px[rng.clone()];
    let py = &g.py[rng.clone()];
    let RowScratch { vg, dd, cum, sdot } = sc;
    for k in 0..l {
        let (u, v, dv) = (
            &g.u[k][rng.clone()],
            &g.v[k][rng.clone()],
            &g.div[k][rng.clone()],
        );
        let vgk = &mut vg[k * nlon..(k + 1) * nlon];
        for i in 0..nlon {
            vgk[i] = (u[i] * px[i] + v[i] * py[i]) * ic2;
        }
        for i in 0..nlon {
            dd[k * nlon + i] = dv[i] + vg[k * nlon + i];
            let prevc = if k > 0 { cum[(k - 1) * nlon + i] } else { 0.0 };
            cum[k * nlon + i] = prevc + c.ds[k] * dd[k * nlon + i];
        }
    }
    for k in 0..l.saturating_sub(1) {
        for i in 0..nlon {
            sdot[(k + 1) * nlon + i] = c.sh[k + 1] * cum[(l - 1) * nlon + i] - cum[k * nlon + i];
        }
    }
    if let Some(np) = row.np.as_deref_mut() {
        for i in 0..nlon {
            let mut s = 0.0;
            for k in 0..l {
                s -= c.ds[k] * vg[k * nlon + i];
            }
            np[i] = s;
        }
    }
    let va = |x: &Grid3, k: usize, i: usize| -> Real {
        let p = r0 + i;
        let below = if k + 1 < l {
            sdot[(k + 1) * nlon + i] * (x[k + 1][p] - x[k][p])
        } else {
            0.0
        };
        let above = if k > 0 {
            sdot[k * nlon + i] * (x[k][p] - x[k - 1][p])
        } else {
            0.0
        };
        (below + above) / (2.0 * c.ds[k])
    };
    let mut umax: Real = 0.0;
    let [qu, qv, ke, tu, tv, trest, sdl] = &mut row.fields;
    for k in 0..l {
        sdl[k].copy_from_slice(&sdot[(k + 1) * nlon..(k + 2) * nlon]);
    }
    for k in 0..l {
        let (u, v, t, z, dv) = (
            &g.u[k][rng.clone()],
            &g.v[k][rng.clone()],
            &g.t[k][rng.clone()],
            &g.vor[k][rng.clone()],
            &g.div[k][rng.clone()],
        );
        for i in 0..nlon {
            let above = if k > 0 {
                c.lnr[k] * cum[(k - 1) * nlon + i]
            } else {
                0.0
            };
            let wp = vg[k * nlon + i] - (above + c.alpha[k] * c.ds[k] * dd[k * nlon + i]) / c.ds[k];
            let eta = z[i] + f;
            qu[k][i] = -eta * v[i] + va(&g.u, k, i) + c.r * t[i] * px[i];
            qv[k][i] = eta * u[i] + va(&g.v, k, i) + c.r * t[i] * py[i];
            ke[k][i] = 0.5 * (u[i] * u[i] + v[i] * v[i]) * ic2;
            tu[k][i] = u[i] * t[i];
            tv[k][i] = v[i] * t[i];
            trest[k][i] = t[i] * dv[i] - va(&g.t, k, i) + c.kappa * (c.tref + t[i]) * wp;
            umax = umax.max(u[i].abs() * ic2);
        }
    }
    umax as f64
}

/// Read-only inputs of the explicit tendencies.
pub(crate) struct DynCtx<'a> {
    pub tr: &'a Transform,
    pub consts: &'a ColumnConsts,
    pub si: &'a SemiImplicit,
    pub coriolis_row: &'a [Real],
    /// Spectral surface geopotential.
    pub phis: &'a [Complex],
    pub dsigma: &'a [Real],
    pub dt: f64,
}

/// Explicit tendencies N of state `s` (physics tendencies `phys` included) written to
/// `out`; returns the max Courant number. Every coefficient of `out` is overwritten.
/// With `timers`, the wall time of [synthesis, grid kernel, tracers, analysis] is added.
pub(crate) fn explicit(
    cx: &DynCtx,
    s: &PeState,
    phys: Option<&PhysTend>,
    w: &mut DynWork,
    out: &mut PeState,
    mut timers: Option<&mut [f64; 4]>,
) -> f64 {
    let tr = cx.tr;
    let l = s.vor.nlev();
    let n = tr.ngrid();
    let nlon = tr.nlon();
    let mut lap = Lap::new();
    w.grids.fill(tr, s);
    lap.add(&mut timers, 0);
    let cfl = cx.dt / (tr.radius as f64 * 2.0 * std::f64::consts::PI / nlon as f64);
    // Split every output into per-row slices so rows can be processed in parallel.
    let mut rows: Vec<RowOut> = (0..tr.nlat()).map(|_| RowOut::default()).collect();
    for (dst, src) in [
        (0usize, &mut w.qu),
        (1, &mut w.qv),
        (2, &mut w.ke),
        (3, &mut w.tu),
        (4, &mut w.tv),
        (5, &mut w.trest),
        (6, &mut w.sdl),
    ] {
        for lvl in src.levels_mut() {
            for (j, chunk) in lvl.chunks_mut(nlon).enumerate() {
                rows[j].fields[dst].push(chunk);
            }
        }
    }
    for (j, chunk) in w.np.chunks_mut(nlon).enumerate() {
        rows[j].np = Some(chunk);
    }
    let g = &w.grids;
    let row_kernel = |sc: &mut RowScratch, (j, row): (usize, RowOut)| -> f64 {
        column_row(
            g,
            cx.consts,
            j,
            nlon,
            l,
            tr.inv_cos2[j],
            cx.coriolis_row[j],
            sc,
            row,
        ) * cfl
    };
    let courant = if rayon::current_num_threads() > 1 {
        rows.into_par_iter()
            .enumerate()
            .map_init(|| RowScratch::new(l, nlon), row_kernel)
            .reduce(|| 0.0, f64::max)
    } else {
        let mut sc = RowScratch::new(l, nlon);
        rows.into_iter()
            .enumerate()
            .map(|r| row_kernel(&mut sc, r))
            .fold(0.0, f64::max)
    };
    lap.add(&mut timers, 1);
    if let Some(ph) = phys {
        for k in 0..l {
            for p in 0..n {
                w.qu[k][p] -= ph.du[k][p];
                w.qv[k][p] -= ph.dv[k][p];
                w.trest[k][p] += ph.dt[k][p];
            }
        }
    }
    for (it, dst) in out.tracers.iter_mut().enumerate() {
        tracers::tendency(tr, &w.grids, it, &w.sdl, cx.dsigma, phys, dst);
    }
    lap.add(&mut timers, 2);
    let (qu, qv, ke, tu, tv, trest) = (&w.qu, &w.qv, &w.ke, &w.tu, &w.tv, &w.trest);
    let per: Vec<_> = map_levels(l, |k| {
        let (dq, cq) = tr.div_curl(&qu[k], &qv[k]);
        let mut kes = tr.analysis(&ke[k]);
        for (e, ps) in kes.iter_mut().zip(cx.phis) {
            *e += *ps;
        }
        let lap = laplacian(&tr.layout, &kes, tr.radius);
        let nvor: Vec<Complex> = cq.iter().map(|c| -c).collect();
        let ndiv: Vec<Complex> = dq.iter().zip(&lap).map(|(d, lp)| -d - lp).collect();
        let fl = tr.div(&tu[k], &tv[k]);
        let rest = tr.analysis(&trest[k]);
        let mut ntmp: Vec<Complex> = rest.iter().zip(&fl).map(|(a, b)| a - b).collect();
        // add back the linear part: N_T = full + τ δ
        for (j, dj) in s.div.levels().enumerate().take(k + 1) {
            let t = cx.si.tau[k * l + j];
            if t != 0.0 {
                for (x, y) in ntmp.iter_mut().zip(dj) {
                    *x += y * t;
                }
            }
        }
        (nvor, ndiv, ntmp)
    });
    for (k, (a, b, c)) in per.into_iter().enumerate() {
        out.vor[k].copy_from_slice(&a);
        out.div[k].copy_from_slice(&b);
        out.tmp[k].copy_from_slice(&c);
    }
    out.lnps.copy_from_slice(&tr.analysis(&w.np));
    lap.add(&mut timers, 3);
    courant
}

/// Stopwatch adding the time since the last lap to slot `i` of optional timers.
pub(crate) struct Lap(std::time::Instant);

impl Lap {
    pub fn new() -> Self {
        Lap(std::time::Instant::now())
    }
    pub fn add<const N: usize>(&mut self, timers: &mut Option<&mut [f64; N]>, i: usize) {
        if let Some(t) = timers.as_deref_mut() {
            let now = std::time::Instant::now();
            t[i] += (now - self.0).as_secs_f64();
            self.0 = now;
        }
    }
}
