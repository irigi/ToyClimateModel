//! Sequential column physics driver (SPEC.md §9.3.3).
//!
//! For every column (latitude rows in parallel) the driver copies the n−1 state into a
//! [`ColumnState`], fills the surface tiles, runs the schemes in order, and collects
//! - the tendencies (final − initial)/Δt_p for the dynamics,
//! - the schemes' diagnostics (registry with typed ids),
//! - what the surface needs afterwards ([`Exchange`] slots: new tile temperatures, u*,
//!   precipitation, evaporation, and the energy-budget fluxes at the top and the surface).
//!
//! Buffers are allocated once; each rayon job owns its column scratch and [`Workspace`].

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::condensation::Condensable;
use crate::planet::Planet;
use crate::primitive::state::{Grid3, Grids, PeState};
use crate::primitive::vertical::SigmaLevels;
use crate::spectral::Transform;
use crate::surface::{Surface, MAX_TILES};
use crate::system::orbit::Orbit;
use crate::Real;
use rayon::prelude::*;
use std::ops::Range;

/// Typed handle of a registered diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagId(pub usize);

/// The diagnostics the schemes declare, in scheme order. Built once; consumers look
/// diagnostics up by [`DiagId`].
#[derive(Debug, Clone, Default)]
pub struct DiagRegistry {
    names: Vec<&'static str>,
    /// Slots of each scheme.
    ranges: Vec<Range<usize>>,
    /// Components of the total precipitation (names `precip_*`), in registry order.
    precip_parts: Vec<DiagId>,
    /// Outgoing longwave radiation (corrected by the surface solve's extra emission).
    olr: Option<DiagId>,
}

impl DiagRegistry {
    pub fn new(schemes: &[Box<dyn ColumnScheme>]) -> Self {
        let mut r = DiagRegistry::default();
        for s in schemes {
            let start = r.names.len();
            r.names.extend_from_slice(s.diagnostics());
            r.ranges.push(start..r.names.len());
        }
        r.precip_parts = (0..r.names.len())
            .filter(|&i| r.names[i].starts_with("precip_"))
            .map(DiagId)
            .collect();
        r.olr = r.id("olr");
        r
    }
    /// Number of diagnostics.
    pub fn len(&self) -> usize {
        self.names.len()
    }
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
    /// All names, in slot order.
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }
    /// The first diagnostic with this name.
    pub fn id(&self, name: &str) -> Option<DiagId> {
        self.names.iter().position(|n| *n == name).map(DiagId)
    }
    pub fn name(&self, id: DiagId) -> &'static str {
        self.names[id.0]
    }
    /// Components of the total precipitation.
    pub fn precip_parts(&self) -> &[DiagId] {
        &self.precip_parts
    }
}

/// Per-point exchange slots written by the driver (`[point][slot]`, f64).
pub struct Exchange;

impl Exchange {
    /// New temperature of tile i is slot `TS + i`.
    pub const TS: usize = 0;
    pub const USTAR: usize = MAX_TILES;
    /// Rain + snow and evaporation \[kg m⁻² s⁻¹\].
    pub const PRECIP: usize = MAX_TILES + 1;
    pub const EVAP: usize = MAX_TILES + 2;
    /// Net downward flux at the top of the atmosphere and net flux into the surface
    /// \[W m⁻²\]; their difference is the atmosphere's energy source.
    pub const TOA_NET: usize = MAX_TILES + 3;
    pub const SFC_NET: usize = MAX_TILES + 4;
    pub const N: usize = MAX_TILES + 5;
}

/// Physics tendencies per level: cos φ-weighted (du, dv), dT and tracer tendencies.
pub struct PhysTend {
    pub du: Grid3,
    pub dv: Grid3,
    pub dt: Grid3,
    /// One `[lev][point]` field per tracer.
    pub dq: Vec<Grid3>,
}

/// Read-only inputs of one physics call.
pub struct DriverCtx<'a> {
    pub tr: &'a Transform,
    pub planet: &'a Planet,
    pub cond: Option<&'a Condensable>,
    /// Surface geopotential on the grid \[m² s⁻²\].
    pub phis_grid: &'a [Real],
    pub surface: Option<&'a Surface>,
    pub orbit: Option<&'a Orbit>,
    /// Model time \[s\].
    pub time: f64,
    /// Length of the step the tendencies are applied over \[s\].
    pub dt: f64,
}

/// One row's output slices: (row, ((tendencies, diagnostics), exchange)).
type RowJob<'a> = (usize, ((&'a mut [Real], &'a mut [Real]), &'a mut [f64]));

/// Column scratch of one job.
struct ColumnScratch {
    st: ColumnState,
    sfc: SurfaceExchange,
    work: Workspace,
    p: Vec<f64>,
    dp: Vec<f64>,
    ph: Vec<f64>,
    diag: Vec<f64>,
}

/// The column physics of a model and everything it needs between calls.
pub struct PhysicsDriver {
    pub schemes: Vec<Box<dyn ColumnScheme>>,
    pub registry: DiagRegistry,
    /// Latest diagnostics, `[diagnostic][point]`.
    pub diag: Grid3,
    /// Latest tendencies.
    pub tend: PhysTend,
    /// Latest exchange slots `[point][Exchange::N]`.
    pub exchange: Vec<f64>,
    sigma: Vec<f64>,
    sigma_half: Vec<f64>,
    ntracers: usize,
    /// Grid fields of the state the physics sees.
    grids: Grids,
    /// Per-row column outputs `[row][lon][du | dv | dT | dq]`.
    row_out: Vec<Real>,
    /// Per-row diagnostics `[row][lon][diag]` (at least one slot per column).
    row_diag: Vec<Real>,
    insol: Vec<Real>,
    coszen: Vec<Real>,
}

impl PhysicsDriver {
    pub fn new(
        schemes: Vec<Box<dyn ColumnScheme>>,
        tr: &Transform,
        levels: &SigmaLevels,
        ntracers: usize,
    ) -> Self {
        let (n, l) = (tr.ngrid(), levels.nlev);
        let registry = DiagRegistry::new(&schemes);
        let z = Grid3::filled(l, n, 0.0);
        let active = !schemes.is_empty();
        let w = (3 + ntracers) * l;
        PhysicsDriver {
            diag: Grid3::filled(registry.len(), n, 0.0),
            tend: PhysTend {
                du: z.clone(),
                dv: z.clone(),
                dt: z.clone(),
                dq: vec![z; ntracers],
            },
            exchange: vec![0.0; if active { n * Exchange::N } else { 0 }],
            sigma: levels.full.clone(),
            sigma_half: levels.half.clone(),
            ntracers,
            grids: Grids::new(if active { l } else { 0 }, n, 0),
            row_out: vec![0.0; if active { n * w } else { 0 }],
            row_diag: vec![0.0; if active { n * registry.len().max(1) } else { 0 }],
            insol: vec![0.0; n],
            coszen: vec![0.0; n],
            registry,
            schemes,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.schemes.is_empty()
    }

    /// Exchange slots of point `p`.
    pub fn exchange_at(&self, p: usize) -> &[f64] {
        &self.exchange[p * Exchange::N..(p + 1) * Exchange::N]
    }

    /// Evaluate all schemes on state `s`; results in [`Self::tend`], [`Self::diag`] and
    /// [`Self::exchange`].
    pub fn run(&mut self, cx: &DriverCtx, s: &PeState) {
        let tr = cx.tr;
        let (nlat, nlon) = (tr.nlat(), tr.nlon());
        let l = self.sigma.len();
        let ntr = self.ntracers;
        let w = (3 + ntr) * l;
        let ndiag = self.registry.len();
        let dslot = ndiag.max(1);
        let tref = cx.planet.t_ref as Real;
        self.grids.fill_with(tr, s, false);
        if let Some(orbit) = cx.orbit {
            let sun = orbit.sun(cx.time);
            self.insol
                .par_chunks_mut(nlon)
                .zip(self.coszen.par_chunks_mut(nlon))
                .enumerate()
                .for_each(|(j, (ins, cz))| {
                    for i in 0..nlon {
                        let (la, lo) = (tr.grid.lat[j], tr.grid.lon[i]);
                        ins[i] = orbit.insolation(&sun, la, lo) as Real;
                        cz[i] = orbit.cos_zenith(&sun, la, lo) as Real;
                    }
                });
        }
        let g = &self.grids;
        let (sigma, sigma_half) = (&self.sigma, &self.sigma_half);
        let (insol, coszen) = (&self.insol, &self.coszen);
        let (schemes, ranges) = (&self.schemes, &self.registry.ranges);
        let olr = self.registry.olr;
        let surface = cx.surface;
        let dtp = cx.dt;
        let row = |sc: &mut ColumnScratch, (j, ((out, diag), exch)): RowJob| {
            let coslat = (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt() as Real;
            for i in 0..nlon {
                let p = j * nlon + i;
                let st = &mut sc.st;
                st.begin(dtp);
                for k in 0..l {
                    st.u[k] = (g.u[k][p] / coslat) as f64;
                    st.v[k] = (g.v[k][p] / coslat) as f64;
                    st.t[k] = (g.t[k][p] + tref) as f64;
                }
                for (it, tq) in g.q.iter().enumerate() {
                    for k in 0..l {
                        st.q[it * l + k] = tq[k][p] as f64;
                    }
                }
                let ps = g.lnps[p].exp() as f64;
                for k in 0..l {
                    sc.p[k] = sigma[k] * ps;
                    sc.dp[k] = (sigma_half[k + 1] - sigma_half[k]) * ps;
                    sc.ph[k] = sigma_half[k] * ps;
                }
                sc.ph[l] = ps;
                let sfc = &mut sc.sfc;
                sfc.begin();
                sfc.tiles.clear();
                if let Some(sf) = surface {
                    sf.tiles_at(p, &mut sfc.tiles);
                    sfc.ustar_prev = sf.ustar[p] as f64;
                }
                let ctx = ColumnCtx {
                    lat: tr.grid.lat[j] as Real as f64,
                    lon: tr.grid.lon[i] as Real as f64,
                    index: p,
                    sigma,
                    sigma_half,
                    ps,
                    p: &sc.p,
                    dp: &sc.dp,
                    p_half: &sc.ph,
                    phis: cx.phis_grid[p] as f64,
                    time: cx.time,
                    dt: dtp,
                    planet: cx.planet,
                    cond: cx.cond,
                    insolation: insol[p] as f64,
                    cos_zenith: coszen[p] as f64,
                };
                sc.diag.fill(0.0);
                for (ph, r) in schemes.iter().zip(ranges) {
                    ph.apply(&ctx, st, sfc, &mut sc.diag[r.clone()], &mut sc.work);
                }
                // the surface's extra linearised emission leaves at the top
                if let Some(id) = olr {
                    sc.diag[id.0] += sfc.lw_up_change;
                }
                let toa_net = sfc.toa_net - sfc.lw_up_change;
                let o = &mut out[i * w..(i + 1) * w];
                for k in 0..l {
                    o[k] = st.du[k] as Real * coslat;
                    o[l + k] = st.dv[k] as Real * coslat;
                    o[2 * l + k] = st.dt[k] as Real;
                }
                for (x, &y) in o[3 * l..].iter_mut().zip(&st.dq) {
                    *x = y as Real;
                }
                for (x, &y) in diag[i * dslot..i * dslot + ndiag].iter_mut().zip(&sc.diag) {
                    *x = y as Real;
                }
                let e = &mut exch[i * Exchange::N..(i + 1) * Exchange::N];
                for (it, t) in sfc.tiles.iter().take(MAX_TILES).enumerate() {
                    e[Exchange::TS + it] = t.ts_new;
                }
                e[Exchange::USTAR] = sfc.ustar;
                e[Exchange::PRECIP] = sfc.rain + sfc.snow;
                e[Exchange::EVAP] = sfc.evap;
                e[Exchange::TOA_NET] = toa_net;
                e[Exchange::SFC_NET] = sfc.sfc_net;
            }
        };
        let jobs = self
            .row_out
            .par_chunks_mut(nlon * w)
            .zip(self.row_diag.par_chunks_mut(nlon * dslot))
            .zip(self.exchange.par_chunks_mut(nlon * Exchange::N))
            .enumerate();
        let scratch = || ColumnScratch {
            st: ColumnState::new(l, ntr),
            sfc: SurfaceExchange {
                tiles: Vec::with_capacity(MAX_TILES),
                ..Default::default()
            },
            work: Workspace::new(l),
            p: vec![0.0; l],
            dp: vec![0.0; l],
            ph: vec![0.0; l + 1],
            diag: vec![0.0; ndiag],
        };
        jobs.for_each_init(scratch, row);
        // Scatter the column-major buffers (`[point][du | dv | dT | dq]`) to
        // `[lev][point]`, in parallel over levels.
        let n = nlat * nlon;
        let row_out = &self.row_out;
        let gather = |off: usize, dst: &mut [Real]| {
            for p in 0..n {
                dst[p] = row_out[p * w + off];
            }
        };
        let t = &mut self.tend;
        let mut dsts: Vec<(usize, &mut [Real])> = Vec::with_capacity((3 + ntr) * l);
        for (f, field) in [&mut t.du, &mut t.dv, &mut t.dt]
            .into_iter()
            .chain(t.dq.iter_mut())
            .enumerate()
        {
            for (k, lvl) in field.levels_mut().enumerate() {
                dsts.push((f * l + k, lvl));
            }
        }
        dsts.into_par_iter().for_each(|(off, dst)| gather(off, dst));
        let row_diag = &self.row_diag;
        self.diag
            .levels_mut()
            .collect::<Vec<_>>()
            .into_par_iter()
            .enumerate()
            .for_each(|(d, dst)| {
                for p in 0..n {
                    dst[p] = row_diag[p * dslot + d];
                }
            });
    }
}
