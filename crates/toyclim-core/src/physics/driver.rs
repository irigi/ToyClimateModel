//! Column physics driver: calls the schemes for every column, in parallel over latitude
//! rows, with buffers that are allocated once (SPEC.md §9.3.3), and owns the diagnostics
//! registry.
//!
//! Still the Part I contract (§5.4): schemes add tendencies evaluated from the same input
//! state (parallel split). The sequential driver of §9.3.3 replaces this in P1.

use super::column::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::primitive::state::{Grid3, Grids, PeState};
use crate::primitive::vertical::SigmaLevels;
use crate::spectral::Transform;
use crate::surface::Surface;
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
    /// Surface evaporation (the last diagnostic named `evap`).
    evap: Option<DiagId>,
}

impl DiagRegistry {
    pub fn new(schemes: &[Box<dyn ColumnPhysics>]) -> Self {
        let mut r = DiagRegistry::default();
        for s in schemes {
            let start = r.names.len();
            r.names.extend(s.diagnostic_names());
            r.ranges.push(start..r.names.len());
        }
        r.precip_parts = (0..r.names.len())
            .filter(|&i| r.names[i].starts_with("precip_"))
            .map(DiagId)
            .collect();
        r.evap = r.names.iter().rposition(|n| *n == "evap").map(DiagId);
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
    pub fn evap(&self) -> Option<DiagId> {
        self.evap
    }
}

/// Physics tendencies per level: cos φ-weighted (du, dv), dT and tracer tendencies.
pub struct PhysTend {
    pub du: Grid3,
    pub dv: Grid3,
    pub dt: Grid3,
    /// One `[lev][point]` field per tracer.
    pub dq: Vec<Grid3>,
    /// Net downward heat flux into the surface per point \[W m⁻²\].
    pub surface_heat: Vec<Real>,
}

/// Read-only inputs of one physics call.
pub struct DriverCtx<'a> {
    pub tr: &'a Transform,
    pub planet: &'a Planet,
    /// Surface geopotential on the grid \[m² s⁻²\].
    pub phis_grid: &'a [Real],
    pub surface: Option<&'a Surface>,
    pub orbit: Option<&'a Orbit>,
    /// Model time \[s\].
    pub time: f64,
    /// Length of the step the tendencies are applied over \[s\].
    pub dt: f64,
}

/// One latitude row's output slices: (row, ((column outputs, diagnostics), surface heat)).
type RowJob<'a> = (usize, ((&'a mut [Real], &'a mut [Real]), &'a mut [Real]));

/// Per-job column scratch.
struct ColumnScratch {
    u: Vec<Real>,
    v: Vec<Real>,
    t: Vec<Real>,
    q: Vec<Real>,
}

/// The column physics of a model and everything it needs between calls.
pub struct PhysicsDriver {
    pub schemes: Vec<Box<dyn ColumnPhysics>>,
    pub registry: DiagRegistry,
    /// Latest diagnostics, `[diagnostic][point]`.
    pub diag: Grid3,
    /// Latest tendencies.
    pub tend: PhysTend,
    sigma: Vec<Real>,
    sigma_half: Vec<Real>,
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
        schemes: Vec<Box<dyn ColumnPhysics>>,
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
                surface_heat: vec![0.0; n],
            },
            sigma: levels.full.iter().map(|&x| x as Real).collect(),
            sigma_half: levels.half.iter().map(|&x| x as Real).collect(),
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

    /// Evaluate all schemes on state `s`; results in [`Self::tend`] and [`Self::diag`].
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
        let surface = cx.surface;
        let row = |sc: &mut ColumnScratch, (j, ((out, diag), sheat)): RowJob| {
            out.fill(0.0);
            diag.fill(0.0);
            sheat.fill(0.0);
            let coslat = (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt() as Real;
            for i in 0..nlon {
                let p = j * nlon + i;
                for k in 0..l {
                    sc.u[k] = g.u[k][p] / coslat;
                    sc.v[k] = g.v[k][p] / coslat;
                    sc.t[k] = g.t[k][p] + tref;
                }
                for (it, tq) in g.q.iter().enumerate() {
                    for k in 0..l {
                        sc.q[it * l + k] = tq[k][p];
                    }
                }
                let col = ColumnIn {
                    lat: tr.grid.lat[j] as Real,
                    lon: tr.grid.lon[i] as Real,
                    index: p,
                    sigma,
                    sigma_half,
                    ps: g.lnps[p].exp(),
                    u: &sc.u,
                    v: &sc.v,
                    t: &sc.t,
                    tracers: &sc.q,
                    phis: cx.phis_grid[p],
                    t_surface: surface.map(|s| s.ts[p]),
                    surface_albedo: surface.and_then(|s| {
                        let a = s.albedo_at(p);
                        (a >= 0.0).then_some(a)
                    }),
                    wetness: surface.map_or(1.0, |s| s.wetness(p)),
                    insolation: insol[p],
                    cos_zenith: coszen[p],
                    time: cx.time,
                    dt: cx.dt,
                };
                let o = &mut out[i * w..(i + 1) * w];
                let (du, rest) = o.split_at_mut(l);
                let (dv, rest) = rest.split_at_mut(l);
                let (dtt, dqq) = rest.split_at_mut(l);
                for (ph, r) in schemes.iter().zip(ranges) {
                    let mut co = ColumnOut {
                        du: &mut *du,
                        dv: &mut *dv,
                        dt: &mut *dtt,
                        dtracers: &mut *dqq,
                        diag: &mut diag[i * ndiag + r.start..i * ndiag + r.end],
                        surface_heat: &mut sheat[i],
                    };
                    ph.tendencies(cx.planet, &col, &mut co);
                }
                for k in 0..l {
                    du[k] *= coslat;
                    dv[k] *= coslat;
                }
            }
        };
        let jobs = self
            .row_out
            .par_chunks_mut(nlon * w)
            .zip(self.row_diag.par_chunks_mut(nlon * dslot))
            .zip(self.tend.surface_heat.par_chunks_mut(nlon))
            .enumerate();
        let scratch = || ColumnScratch {
            u: vec![0.0; l],
            v: vec![0.0; l],
            t: vec![0.0; l],
            q: vec![0.0; ntr * l],
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
                    dst[p] = row_diag[p * ndiag + d];
                }
            });
    }
}
