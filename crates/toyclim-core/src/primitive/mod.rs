//! Dry primitive equations in σ coordinates (SPEC.md §5.2–5.3).
//!
//! Prognostic spectral variables per level k (0 = top): vorticity ζ_k, divergence δ_k,
//! temperature deviation T'_k = T_k − T̄; plus ln p_s. Lorenz staggering with the
//! Simmons & Burridge (1981) energy- and angular-momentum-conserving discretization.
//! Semi-implicit leapfrog (Hoskins & Simmons 1975), RAW filter, implicit hyperdiffusion.
//! Column physics is evaluated at the n−1 level (stable for damping terms).

pub mod cases;
pub mod vertical;

use crate::grid::GaussianGrid;
use crate::model::{GridField, Model, RestartArray};
use crate::physics::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::spectral::ops::{hyperdiffusion_factors, laplacian};
use crate::spectral::{Complex, Transform};
use crate::time::leapfrog::{scale_by, RawFilter};
use crate::Real;
use rayon::prelude::*;
use vertical::SigmaLevels;

const CZERO: Complex = Complex { re: 0.0, im: 0.0 };

/// 2D fields interpolated to pressure levels.
pub const PRESSURE_FIELDS: &[&str] = &["t850", "u850", "v850", "t500", "phi500", "u250", "vor850"];

/// One time level of the spectral state.
#[derive(Debug, Clone)]
pub struct PeState {
    pub vor: Vec<Vec<Complex>>,
    pub div: Vec<Vec<Complex>>,
    /// T' = T − T̄ \[K\].
    pub tmp: Vec<Vec<Complex>>,
    /// ln(p_s / 1 Pa).
    pub lnps: Vec<Complex>,
    /// Tracers `[tracer][lev]` (e.g. specific humidity q \[kg/kg\]).
    pub tracers: Vec<Vec<Vec<Complex>>>,
}

impl PeState {
    pub fn zeros(nlev: usize, nspec: usize) -> Self {
        Self::zeros_with_tracers(nlev, nspec, 0)
    }
    pub fn zeros_with_tracers(nlev: usize, nspec: usize, ntracers: usize) -> Self {
        let z = vec![CZERO; nspec];
        PeState {
            vor: vec![z.clone(); nlev],
            div: vec![z.clone(); nlev],
            tmp: vec![z.clone(); nlev],
            tracers: vec![vec![z.clone(); nlev]; ntracers],
            lnps: z,
        }
    }
    fn is_finite(&self) -> bool {
        let ok = |v: &[Complex]| v.iter().all(|c| c.re.is_finite() && c.im.is_finite());
        self.vor.iter().all(|v| ok(v))
            && self.div.iter().all(|v| ok(v))
            && self.tmp.iter().all(|v| ok(v))
            && self.tracers.iter().flatten().all(|v| ok(v))
            && ok(&self.lnps)
    }
}

/// Initial condition from a test case.
pub struct PeInitial {
    pub state: PeState,
    /// Surface geopotential on the grid \[m² s⁻²\].
    pub phis_grid: Vec<Real>,
}

/// Numerical settings.
#[derive(Debug, Clone)]
pub struct PeSettings {
    pub dt: f64,
    pub diffusion_order: u32,
    pub diffusion_tau: f64,
    pub filter: RawFilter,
    /// Restore the global dry mass after every step (shift of the ℓ = 0 coefficient of
    /// ln p_s). ln p_s models conserve ∫ ln p_s, not ∫ p_s, without it.
    pub mass_fixer: bool,
}

/// Grid-point fields of one state.
struct Grids {
    /// cos φ-weighted winds U, V per level.
    u: Vec<Vec<Real>>,
    v: Vec<Vec<Real>>,
    vor: Vec<Vec<Real>>,
    div: Vec<Vec<Real>>,
    /// T' per level.
    t: Vec<Vec<Real>>,
    lnps: Vec<Real>,
    /// cos φ-weighted gradient of ln p_s.
    px: Vec<Real>,
    py: Vec<Real>,
    /// Tracers `[tracer][lev]`.
    q: Vec<Vec<Vec<Real>>>,
}

/// Physics tendencies per level: cos φ-weighted (du, dv), dT and tracer tendencies.
struct PhysTend {
    du: Vec<Vec<Real>>,
    dv: Vec<Vec<Real>>,
    dt: Vec<Vec<Real>>,
    /// `[tracer][lev][point]`.
    dq: Vec<Vec<Vec<Real>>>,
}

/// Vertical constants used by the column kernel.
struct ColumnConsts {
    ds: Vec<Real>,
    sh: Vec<Real>,
    alpha: Vec<Real>,
    lnr: Vec<Real>,
    r: Real,
    kappa: Real,
    tref: Real,
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
    mut row: RowOut,
) -> f64 {
    let r0 = j * nlon;
    let rng = r0..r0 + nlon;
    let px = &g.px[rng.clone()];
    let py = &g.py[rng.clone()];
    // vg_k = v·∇ln p_s, dd_k = D_k, cum_k = Σ_{i≤k} Δσ_i D_i
    let mut vg = vec![0.0 as Real; l * nlon];
    let mut dd = vec![0.0 as Real; l * nlon];
    let mut cum = vec![0.0 as Real; l * nlon];
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
    // σ̇ at half levels 0..=l (zero at top and bottom)
    let mut sdot = vec![0.0 as Real; (l + 1) * nlon];
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
    let va = |x: &[Vec<Real>], k: usize, i: usize| -> Real {
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

/// The primitive-equation model.
pub struct Primitive {
    pub tr: Transform,
    pub planet: Planet,
    pub settings: PeSettings,
    pub levels: SigmaLevels,
    pub phis_grid: Vec<Real>,
    pub phis: Vec<Complex>,
    pub prev: PeState,
    pub curr: PeState,
    pub physics: Vec<Box<dyn ColumnPhysics>>,
    /// Latest diagnostics written by the physics schemes: (name, grid).
    pub physics_diag: Vec<(String, Vec<Real>)>,
    coriolis_row: Vec<Real>,
    g: Vec<Real>,
    tau: Vec<Real>,
    /// S_ℓ⁻¹ for the leapfrog step and for the first (half-length) step, per ℓ, L×L.
    sinv_leap: Vec<Vec<Real>>,
    sinv_first: Vec<Vec<Real>>,
    diff_leap: Vec<Real>,
    diff_first: Vec<Real>,
    steps: u64,
    pub last_courant: f64,
    /// Target global mass for the mass fixer \[kg\] (set from the initial state).
    pub mass_target: f64,
    /// The condensable of tracer 0, if the model is moist.
    pub condensable: Option<crate::physics::moist::Condensable>,
}

impl Primitive {
    pub fn new(
        tr: Transform,
        planet: Planet,
        settings: PeSettings,
        levels: SigmaLevels,
        init: PeInitial,
        physics: Vec<Box<dyn ColumnPhysics>>,
    ) -> Self {
        let l = levels.nlev;
        let r = planet.gas_constant;
        let kappa = planet.kappa();
        let tref = planet.t_ref;
        let g = levels.g_matrix(r);
        let tau = levels.tau_matrix(kappa, tref);
        let b = levels.b_matrix(r, kappa, tref);
        let a2 = planet.radius * planet.radius;
        let make_sinv = |dt: f64| -> Vec<Vec<Real>> {
            (0..=tr.mmax())
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
        let sinv_leap = make_sinv(settings.dt);
        let sinv_first = make_sinv(0.5 * settings.dt);
        let lay = &tr.layout;
        let diff_leap = hyperdiffusion_factors(
            lay,
            settings.diffusion_order,
            settings.diffusion_tau,
            2.0 * settings.dt,
        );
        let diff_first = hyperdiffusion_factors(
            lay,
            settings.diffusion_order,
            settings.diffusion_tau,
            settings.dt,
        );
        let coriolis_row = tr
            .grid
            .mu
            .iter()
            .map(|m| (2.0 * planet.rotation_rate * m) as Real)
            .collect();
        let phis = tr.analysis(&init.phis_grid);
        let physics_diag = physics
            .iter()
            .flat_map(|p| p.diagnostic_names())
            .map(|n| (n.to_string(), tr.grid_zeros()))
            .collect();
        Primitive {
            phis,
            phis_grid: init.phis_grid,
            prev: init.state.clone(),
            curr: init.state,
            physics,
            physics_diag,
            coriolis_row,
            g: g.iter().map(|&x| x as Real).collect(),
            tau: tau.iter().map(|&x| x as Real).collect(),
            sinv_leap,
            sinv_first,
            diff_leap,
            diff_first,
            steps: 0,
            last_courant: 0.0,
            mass_target: 0.0,
            condensable: None,
            levels,
            tr,
            planet,
            settings,
        }
    }

    pub fn nlev(&self) -> usize {
        self.levels.nlev
    }

    /// Global mass ∫ p_s/g dA of a ln p_s field \[kg\].
    pub fn mass_of(&self, lnps: &[Complex]) -> f64 {
        let ps: Vec<Real> = self.tr.synthesis(lnps).iter().map(|x| x.exp()).collect();
        self.tr.integrate(&ps) / self.planet.gravity
    }

    /// Negative-value fixer for tracers: in grid space, set negative values to zero and
    /// scale the positive values so that the global tracer mass ∫Σ_k q Δσ_k p_s/g dA is
    /// unchanged; then transform back. Applied to the new time level after each step.
    fn fix_tracers(&mut self) {
        if self.curr.tracers.is_empty() {
            return;
        }
        let tr = &self.tr;
        let l = self.nlev();
        let ps: Vec<Real> = tr
            .synthesis(&self.curr.lnps)
            .iter()
            .map(|x| x.exp())
            .collect();
        let ds = self.levels.dsigma.clone();
        for it in 0..self.curr.tracers.len() {
            let mut grids: Vec<Vec<Real>> =
                self.map_levels(|k| tr.synthesis(&self.curr.tracers[it][k]));
            if !grids.iter().flatten().any(|&x| x < 0.0) {
                continue;
            }
            let column_mass = |g: &Vec<Vec<Real>>| -> f64 {
                let mut w = vec![0.0 as Real; tr.ngrid()];
                for k in 0..l {
                    for (p, x) in w.iter_mut().enumerate() {
                        *x += g[k][p] * ds[k] as Real * ps[p];
                    }
                }
                tr.integrate(&w)
            };
            let before = column_mass(&grids);
            for g in grids.iter_mut() {
                for x in g.iter_mut() {
                    if *x < 0.0 {
                        *x = 0.0;
                    }
                }
            }
            let after = column_mass(&grids);
            let scale = if after > 0.0 && before > 0.0 {
                (before / after) as Real
            } else {
                0.0
            };
            for g in grids.iter_mut() {
                for x in g.iter_mut() {
                    *x *= scale;
                }
            }
            let spec: Vec<Vec<Complex>> = self.map_levels(|k| tr.analysis(&grids[k]));
            self.curr.tracers[it] = spec;
        }
    }

    /// Shift ln p_s by a constant so the global mass equals `mass_target`.
    fn fix_mass(&mut self) {
        if self.mass_target <= 0.0 {
            return;
        }
        let m = self.mass_of(&self.curr.lnps);
        let shift = (self.mass_target / m).ln() * std::f64::consts::SQRT_2;
        self.curr.lnps[0].re += shift as Real;
    }

    /// Map over levels, in parallel when more than one thread is available. Each level is
    /// computed identically either way (deterministic).
    fn map_levels<T: Send>(&self, f: impl Fn(usize) -> T + Sync + Send) -> Vec<T> {
        if rayon::current_num_threads() > 1 {
            (0..self.nlev()).into_par_iter().map(f).collect()
        } else {
            (0..self.nlev()).map(f).collect()
        }
    }

    fn grids(&self, s: &PeState) -> Grids {
        let tr = &self.tr;
        let per: Vec<_> = self.map_levels(|k| {
            let (u, v) = tr.uv_from_vort_div(&s.vor[k], &s.div[k]);
            (
                u,
                v,
                tr.synthesis(&s.vor[k]),
                tr.synthesis(&s.div[k]),
                tr.synthesis(&s.tmp[k]),
            )
        });
        let (px, py) = tr.grad(&s.lnps);
        let q: Vec<Vec<Vec<Real>>> = s
            .tracers
            .iter()
            .map(|tq| self.map_levels(|k| tr.synthesis(&tq[k])))
            .collect();
        let mut g = Grids {
            q,
            u: Vec::new(),
            v: Vec::new(),
            vor: Vec::new(),
            div: Vec::new(),
            t: Vec::new(),
            lnps: tr.synthesis(&s.lnps),
            px,
            py,
        };
        for (u, v, z, d, t) in per {
            g.u.push(u);
            g.v.push(v);
            g.vor.push(z);
            g.div.push(d);
            g.t.push(t);
        }
        g
    }

    /// Physics tendencies from `s` (the n−1 level): cos φ-weighted (du, dv) and dT per level.
    fn physics_tendencies(&mut self, s: &PeState) -> Option<PhysTend> {
        if self.physics.is_empty() {
            return None;
        }
        let g = self.grids(s);
        let tr = &self.tr;
        let (nlat, nlon, l) = (tr.nlat(), tr.nlon(), self.nlev());
        let sigma: Vec<Real> = self.levels.full.iter().map(|&x| x as Real).collect();
        let sigma_half: Vec<Real> = self.levels.half.iter().map(|&x| x as Real).collect();
        let ndiag: usize = self.physics_diag.len();
        let ntr = s.tracers.len();
        let w = (3 + ntr) * l;
        let tref = self.planet.t_ref as Real;
        let time = self.time();
        let dt2 = if self.steps == 0 {
            self.settings.dt
        } else {
            2.0 * self.settings.dt
        };
        let planet = &self.planet;
        let physics = &self.physics;
        // Per row: [du | dv | dT] for each column, plus diagnostics.
        let rows: Vec<(Vec<Real>, Vec<Real>)> = (0..nlat)
            .into_par_iter()
            .map(|j| {
                let coslat = (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt() as Real;
                let mut out = vec![0.0 as Real; nlon * w];
                let mut diag = vec![0.0 as Real; nlon * ndiag];
                let (mut u, mut v, mut t) = (vec![0.0; l], vec![0.0; l], vec![0.0; l]);
                let mut qc = vec![0.0 as Real; ntr * l];
                for i in 0..nlon {
                    let p = j * nlon + i;
                    for k in 0..l {
                        u[k] = g.u[k][p] / coslat;
                        v[k] = g.v[k][p] / coslat;
                        t[k] = g.t[k][p] + tref;
                    }
                    for (it, tq) in g.q.iter().enumerate() {
                        for k in 0..l {
                            qc[it * l + k] = tq[k][p];
                        }
                    }
                    let col = ColumnIn {
                        lat: tr.grid.lat[j] as Real,
                        lon: tr.grid.lon[i] as Real,
                        index: p,
                        sigma: &sigma,
                        sigma_half: &sigma_half,
                        ps: g.lnps[p].exp(),
                        u: &u,
                        v: &v,
                        t: &t,
                        tracers: &qc,
                        phis: self.phis_grid[p],
                        time,
                        dt: dt2,
                    };
                    let o = &mut out[i * w..(i + 1) * w];
                    let (du, rest) = o.split_at_mut(l);
                    let (dv, rest) = rest.split_at_mut(l);
                    let (dtt, dqq) = rest.split_at_mut(l);
                    let mut off = 0;
                    for ph in physics.iter() {
                        let nd = ph.diagnostic_names().len();
                        let mut co = ColumnOut {
                            du: &mut *du,
                            dv: &mut *dv,
                            dt: &mut *dtt,
                            dtracers: &mut *dqq,
                            diag: &mut diag[i * ndiag + off..i * ndiag + off + nd],
                        };
                        ph.tendencies(planet, &col, &mut co);
                        off += nd;
                    }
                    for k in 0..l {
                        du[k] *= coslat;
                        dv[k] *= coslat;
                    }
                }
                (out, diag)
            })
            .collect();
        let npts = nlat * nlon;
        let mut du = vec![vec![0.0 as Real; npts]; l];
        let mut dv = vec![vec![0.0 as Real; npts]; l];
        let mut dtt = vec![vec![0.0 as Real; npts]; l];
        let mut dq = vec![vec![vec![0.0 as Real; npts]; l]; ntr];
        for (j, (out, diag)) in rows.iter().enumerate() {
            for i in 0..nlon {
                let p = j * nlon + i;
                for k in 0..l {
                    du[k][p] = out[i * w + k];
                    dv[k][p] = out[i * w + l + k];
                    dtt[k][p] = out[i * w + 2 * l + k];
                }
                for (it, dqt) in dq.iter_mut().enumerate() {
                    for k in 0..l {
                        dqt[k][p] = out[i * w + (3 + it) * l + k];
                    }
                }
                for (d, (_, grid)) in self.physics_diag.iter_mut().enumerate() {
                    grid[p] = diag[i * ndiag + d];
                }
            }
        }
        Some(PhysTend {
            du,
            dv,
            dt: dtt,
            dq,
        })
    }

    /// Explicit tendencies N at the current level (and max Courant number).
    fn tendencies(&self, s: &PeState, phys: Option<&PhysTend>) -> (PeState, f64) {
        let tr = &self.tr;
        let lev = &self.levels;
        let l = lev.nlev;
        let n = tr.ngrid();
        let nlon = tr.nlon();
        let r = self.planet.gas_constant as Real;
        let kappa = self.planet.kappa() as Real;
        let tref = self.planet.t_ref as Real;
        let g = self.grids(s);

        let mut qu = vec![vec![0.0 as Real; n]; l];
        let mut qv = vec![vec![0.0 as Real; n]; l];
        let mut ke = vec![vec![0.0 as Real; n]; l];
        let mut tu = vec![vec![0.0 as Real; n]; l];
        let mut tv = vec![vec![0.0 as Real; n]; l];
        let mut trest = vec![vec![0.0 as Real; n]; l];
        let mut np = vec![0.0 as Real; n];
        let mut sdl = vec![vec![0.0 as Real; n]; l];
        let consts = ColumnConsts {
            ds: lev.dsigma.iter().map(|&x| x as Real).collect(),
            sh: lev.half.iter().map(|&x| x as Real).collect(),
            alpha: lev.alpha.iter().map(|&x| x as Real).collect(),
            lnr: lev.lnratio.iter().map(|&x| x as Real).collect(),
            r,
            kappa,
            tref,
        };
        let cfl = self.settings.dt / (tr.radius as f64 * 2.0 * std::f64::consts::PI / nlon as f64);
        // Split every output into per-row slices so rows can be processed in parallel.
        let mut rows: Vec<RowOut> = (0..tr.nlat()).map(|_| RowOut::default()).collect();
        for (dst, src) in [
            (0usize, &mut qu),
            (1, &mut qv),
            (2, &mut ke),
            (3, &mut tu),
            (4, &mut tv),
            (5, &mut trest),
            (6, &mut sdl),
        ] {
            for lvl in src.iter_mut() {
                for (j, chunk) in lvl.chunks_mut(nlon).enumerate() {
                    rows[j].fields[dst].push(chunk);
                }
            }
        }
        for (j, chunk) in np.chunks_mut(nlon).enumerate() {
            rows[j].np = Some(chunk);
        }
        let row_kernel = |(j, row): (usize, RowOut)| -> f64 {
            column_row(
                &g,
                &consts,
                j,
                nlon,
                l,
                tr.inv_cos2[j],
                self.coriolis_row[j],
                row,
            ) * cfl
        };
        let courant = if rayon::current_num_threads() > 1 {
            rows.into_par_iter()
                .enumerate()
                .map(row_kernel)
                .reduce(|| 0.0, f64::max)
        } else {
            rows.into_iter()
                .enumerate()
                .map(row_kernel)
                .fold(0.0, f64::max)
        };
        if let Some(ph) = phys {
            for k in 0..l {
                for p in 0..n {
                    qu[k][p] -= ph.du[k][p];
                    qv[k][p] -= ph.dv[k][p];
                    trest[k][p] += ph.dt[k][p];
                }
            }
        }
        // Tracers: −∇·(v q) + q δ − σ̇ ∂q/∂σ + physics.
        let ds: Vec<Real> = lev.dsigma.iter().map(|&x| x as Real).collect();
        let tracer_tend: Vec<Vec<Vec<Complex>>> =
            g.q.iter()
                .enumerate()
                .map(|(it, q)| {
                    self.map_levels(|k| {
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
                    })
                })
                .collect();
        let per: Vec<_> = self.map_levels(|k| {
            let (dq, cq) = tr.div_curl(&qu[k], &qv[k]);
            let mut kes = tr.analysis(&ke[k]);
            for (e, ps) in kes.iter_mut().zip(&self.phis) {
                *e += *ps;
            }
            let lap = laplacian(&tr.layout, &kes, tr.radius);
            let nvor: Vec<Complex> = cq.iter().map(|c| -c).collect();
            let ndiv: Vec<Complex> = dq.iter().zip(&lap).map(|(d, lp)| -d - lp).collect();
            let fl = tr.div(&tu[k], &tv[k]);
            let rest = tr.analysis(&trest[k]);
            let mut ntmp: Vec<Complex> = rest.iter().zip(&fl).map(|(a, b)| a - b).collect();
            // add back the linear part: N_T = full + τ δ
            for (j, dj) in s.div.iter().enumerate().take(k + 1) {
                let t = self.tau[k * l + j];
                if t != 0.0 {
                    for (x, y) in ntmp.iter_mut().zip(dj) {
                        *x += y * t;
                    }
                }
            }
            (nvor, ndiv, ntmp)
        });
        let mut out = PeState {
            vor: Vec::with_capacity(l),
            div: Vec::with_capacity(l),
            tmp: Vec::with_capacity(l),
            lnps: tr.analysis(&np),
            tracers: tracer_tend,
        };
        for (a, b, c) in per {
            out.vor.push(a);
            out.div.push(b);
            out.tmp.push(c);
        }
        (out, courant)
    }

    /// Semi-implicit update from `minus` with centre-level tendencies `n`, half-step `dt`.
    fn implicit_update(
        &self,
        minus: &PeState,
        n: &PeState,
        dt: Real,
        sinv: &[Vec<Real>],
    ) -> PeState {
        let lay = &self.tr.layout;
        let l = self.nlev();
        let a2 = self.tr.radius * self.tr.radius;
        let rt = (self.planet.gas_constant * self.planet.t_ref) as Real;
        let w: Vec<Real> = self.levels.dsigma.iter().map(|&x| x as Real).collect();
        let mut out = PeState::zeros_with_tracers(l, lay.len(), minus.tracers.len());
        for (it, tq) in out.tracers.iter_mut().enumerate() {
            for k in 0..l {
                for (idx, x) in tq[k].iter_mut().enumerate() {
                    *x = minus.tracers[it][k][idx] + n.tracers[it][k][idx] * (2.0 * dt);
                }
            }
        }
        let mut tt = vec![CZERO; l];
        let mut rhs = vec![CZERO; l];
        let mut x = vec![CZERO; l];
        for idx in 0..lay.len() {
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
                out.div[k][idx] = x[k] * 2.0 - minus.div[k][idx];
                let mut tx = CZERO;
                for j in 0..=k {
                    tx += x[j] * self.tau[k * l + j];
                }
                out.tmp[k][idx] = minus.tmp[k][idx] + (n.tmp[k][idx] - tx) * (2.0 * dt);
                out.vor[k][idx] = minus.vor[k][idx] + n.vor[k][idx] * (2.0 * dt);
            }
            out.lnps[idx] = minus.lnps[idx] + (n.lnps[idx] - wx) * (2.0 * dt);
        }
        out
    }

    /// Mass \[kg\], total energy ∫(c_p T + Φ_s + E) dm \[J\], axial angular momentum
    /// \[kg m² s⁻¹\], max |u| \[m/s\] and kinetic energy ∫E dm \[J\].
    pub fn conservation(&self) -> [f64; 5] {
        let tr = &self.tr;
        let g = self.grids(&self.curr);
        let n = tr.ngrid();
        let nlon = tr.nlon();
        let grav = self.planet.gravity;
        let a = self.planet.radius;
        let om = self.planet.rotation_rate;
        let cp = self.planet.cp;
        let tref = self.planet.t_ref;
        let (mut mass, mut en, mut am) = (
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
            vec![0.0 as Real; n],
        );
        let mut kin_e = vec![0.0 as Real; n];
        let mut umax = 0.0f64;
        for j in 0..tr.nlat() {
            let c2 = 1.0 - tr.grid.mu[j] * tr.grid.mu[j];
            for p in j * nlon..(j + 1) * nlon {
                let ps = (g.lnps[p] as f64).exp();
                let dm = ps / grav;
                let (mut e, mut m, mut ke) = (self.phis_grid[p] as f64, 0.0, 0.0);
                for k in 0..self.nlev() {
                    let ds = self.levels.dsigma[k];
                    let (u, v) = (g.u[k][p] as f64, g.v[k][p] as f64);
                    let kin = 0.5 * (u * u + v * v) / c2;
                    e += ds * (cp * (g.t[k][p] as f64 + tref) + kin);
                    ke += ds * kin;
                    m += ds * (a * u + om * a * a * c2);
                    umax = umax.max(u.abs() / c2.sqrt());
                }
                mass[p] = dm as Real;
                en[p] = (dm * e) as Real;
                am[p] = (dm * m) as Real;
                kin_e[p] = (dm * ke) as Real;
            }
        }
        [
            tr.integrate(&mass),
            tr.integrate(&en),
            tr.integrate(&am),
            umax,
            tr.integrate(&kin_e),
        ]
    }

    /// Interpolate a 3D field (`[lev][lat][lon]`) to pressure `p` \[Pa\], linear in ln p
    /// between full levels; constant extrapolation outside the column.
    fn to_pressure(&self, field: &[Real], p: f64) -> Vec<Real> {
        let tr = &self.tr;
        let n = tr.ngrid();
        let l = self.nlev();
        let ps = tr.synthesis(&self.curr.lnps);
        let lnsig: Vec<f64> = self.levels.full.iter().map(|s| s.ln()).collect();
        (0..n)
            .map(|q| {
                let target = p.ln() - ps[q] as f64; // ln σ of the target
                if target <= lnsig[0] {
                    return field[q];
                }
                if target >= lnsig[l - 1] {
                    return field[(l - 1) * n + q];
                }
                let k = lnsig.partition_point(|&x| x < target) - 1;
                let w = (target - lnsig[k]) / (lnsig[k + 1] - lnsig[k]);
                (field[k * n + q] as f64 * (1.0 - w) + field[(k + 1) * n + q] as f64 * w) as Real
            })
            .collect()
    }

    fn level_fields(&self, name: &str) -> Option<Vec<Real>> {
        let tr = &self.tr;
        let l = self.nlev();
        let nlon = tr.nlon();
        let n = tr.ngrid();
        let tref = self.planet.t_ref as Real;
        let unweight = |x: &mut [Real]| {
            for j in 0..tr.nlat() {
                let c = (1.0 / (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt()) as Real;
                x[j * nlon..(j + 1) * nlon].iter_mut().for_each(|v| *v *= c);
            }
        };
        let mut out = Vec::with_capacity(n * l);
        match name {
            "u" | "v" => {
                for k in 0..l {
                    let (mut u, mut v) = tr.uv_from_vort_div(&self.curr.vor[k], &self.curr.div[k]);
                    let x = if name == "u" { &mut u } else { &mut v };
                    unweight(x);
                    out.extend_from_slice(x);
                }
            }
            "vor" => (0..l).for_each(|k| out.extend(tr.synthesis(&self.curr.vor[k]))),
            "div" => (0..l).for_each(|k| out.extend(tr.synthesis(&self.curr.div[k]))),
            "t" => (0..l)
                .for_each(|k| out.extend(tr.synthesis(&self.curr.tmp[k]).iter().map(|t| t + tref))),
            "q" => {
                let tq = self.curr.tracers.first()?;
                (0..l).for_each(|k| out.extend(tr.synthesis(&tq[k])));
            }
            "rh" => {
                let cond = self.condensable.as_ref()?;
                let tq = self.curr.tracers.first()?;
                let ps: Vec<Real> = tr
                    .synthesis(&self.curr.lnps)
                    .iter()
                    .map(|x| x.exp())
                    .collect();
                for k in 0..l {
                    let q = tr.synthesis(&tq[k]);
                    let t = tr.synthesis(&self.curr.tmp[k]);
                    for p in 0..n {
                        let pr = self.levels.full[k] * ps[p] as f64;
                        let qs = cond.qsat(&self.planet, (t[p] + tref) as f64, pr);
                        out.push((q[p] as f64 / qs) as Real);
                    }
                }
            }
            "phi" => {
                // geopotential Φ_k = Φ_s + Σ_j G_kj T_j (full T)
                let t: Vec<Vec<Real>> = (0..l)
                    .map(|k| {
                        tr.synthesis(&self.curr.tmp[k])
                            .iter()
                            .map(|x| x + tref)
                            .collect()
                    })
                    .collect();
                for k in 0..l {
                    for p in 0..n {
                        let mut v = self.phis_grid[p];
                        for j in k..l {
                            v += self.g[k * l + j] * t[j][p];
                        }
                        out.push(v);
                    }
                }
            }
            _ => return None,
        }
        Some(out)
    }
}

impl Model for Primitive {
    fn kind(&self) -> &'static str {
        "primitive"
    }

    fn step(&mut self) {
        let first = self.steps == 0;
        let phys = if self.physics.is_empty() {
            None
        } else {
            let prev_state = self.prev.clone();
            self.physics_tendencies(&prev_state)
        };
        let (n, courant) = self.tendencies(&self.curr, phys.as_ref());
        self.last_courant = courant;
        let dt = self.settings.dt as Real;
        let mut next = if first {
            self.implicit_update(&self.curr, &n, 0.5 * dt, &self.sinv_first)
        } else {
            self.implicit_update(&self.prev, &n, dt, &self.sinv_leap)
        };
        let diff = if first {
            &self.diff_first
        } else {
            &self.diff_leap
        };
        for k in 0..self.nlev() {
            scale_by(&mut next.vor[k], diff);
            scale_by(&mut next.div[k], diff);
            scale_by(&mut next.tmp[k], diff);
            for tq in next.tracers.iter_mut() {
                scale_by(&mut tq[k], diff);
            }
        }
        let mut curr = std::mem::replace(&mut self.curr, next);
        if !first {
            let f = self.settings.filter;
            for k in 0..self.nlev() {
                f.apply(&self.prev.vor[k], &mut curr.vor[k], &mut self.curr.vor[k]);
                f.apply(&self.prev.div[k], &mut curr.div[k], &mut self.curr.div[k]);
                f.apply(&self.prev.tmp[k], &mut curr.tmp[k], &mut self.curr.tmp[k]);
            }
            f.apply(&self.prev.lnps, &mut curr.lnps, &mut self.curr.lnps);
            for it in 0..curr.tracers.len() {
                for k in 0..self.nlev() {
                    f.apply(
                        &self.prev.tracers[it][k],
                        &mut curr.tracers[it][k],
                        &mut self.curr.tracers[it][k],
                    );
                }
            }
        }
        self.prev = curr;
        if self.settings.mass_fixer {
            self.fix_mass();
        }
        self.fix_tracers();
        self.steps += 1;
    }

    fn time(&self) -> f64 {
        self.steps as f64 * self.settings.dt
    }
    fn step_count(&self) -> u64 {
        self.steps
    }
    fn dt(&self) -> f64 {
        self.settings.dt
    }
    fn grid(&self) -> &GaussianGrid {
        &self.tr.grid
    }
    fn sigma(&self) -> Vec<f64> {
        self.levels.full.clone()
    }
    fn field_names(&self) -> Vec<&'static str> {
        let mut v = vec!["u", "v", "t", "vor", "div", "phi", "ps", "phis"];
        v.extend_from_slice(PRESSURE_FIELDS);
        if !self.curr.tracers.is_empty() {
            v.extend_from_slice(&["q", "rh", "pw"]);
        }
        if self.physics.iter().any(|p| {
            p.diagnostic_names()
                .iter()
                .any(|n| n.starts_with("precip_"))
        }) {
            v.push("precip");
        }
        v.extend(self.physics.iter().flat_map(|p| p.diagnostic_names()));
        v
    }
    fn field(&self, name: &str) -> Option<GridField> {
        match name {
            "ps" => Some(GridField {
                nlev: 1,
                data: self
                    .tr
                    .synthesis(&self.curr.lnps)
                    .iter()
                    .map(|x| x.exp())
                    .collect(),
            }),
            "phis" => Some(GridField {
                nlev: 1,
                data: self.phis_grid.clone(),
            }),
            // precipitable water Σ q Δσ p_s / g [kg m⁻² = mm]
            "pw" => {
                let tq = self.curr.tracers.first()?;
                let tr = &self.tr;
                let ps: Vec<Real> = tr
                    .synthesis(&self.curr.lnps)
                    .iter()
                    .map(|x| x.exp())
                    .collect();
                let mut w = vec![0.0 as Real; tr.ngrid()];
                for k in 0..self.nlev() {
                    let q = tr.synthesis(&tq[k]);
                    let ds = self.levels.dsigma[k] as Real;
                    for p in 0..w.len() {
                        w[p] += q[p] * ds * ps[p] / self.planet.gravity as Real;
                    }
                }
                Some(GridField { nlev: 1, data: w })
            }
            // Pressure-level fields: <name><hPa>, e.g. t850, u850, v250, phi500.
            _ if PRESSURE_FIELDS.contains(&name) => {
                let split = name.find(|c: char| c.is_ascii_digit())?;
                let (base, hpa) = name.split_at(split);
                let hpa: f64 = hpa.parse().ok()?;
                let f3 = self.level_fields(base)?;
                Some(GridField {
                    nlev: 1,
                    data: self.to_pressure(&f3, hpa * 100.0),
                })
            }
            // total precipitation = sum of all precip_* diagnostics
            "precip" if !self.physics_diag.iter().any(|(n, _)| n == "precip") => {
                let mut acc: Option<Vec<Real>> = None;
                for (n, g) in &self.physics_diag {
                    if n.starts_with("precip_") {
                        match acc.as_mut() {
                            None => acc = Some(g.clone()),
                            Some(a) => a.iter_mut().zip(g).for_each(|(x, y)| *x += *y),
                        }
                    }
                }
                acc.map(|data| GridField { nlev: 1, data })
            }
            _ => {
                if let Some((_, g)) = self.physics_diag.iter().find(|(n, _)| n == name) {
                    return Some(GridField {
                        nlev: 1,
                        data: g.clone(),
                    });
                }
                self.level_fields(name).map(|data| GridField {
                    nlev: self.nlev(),
                    data,
                })
            }
        }
    }
    fn diagnostics(&self) -> Vec<(&'static str, f64)> {
        let [mass, energy, am, umax, ke] = self.conservation();
        vec![
            ("mass", mass),
            ("energy", energy),
            ("angular_momentum", am),
            ("kinetic_energy", ke),
            ("max_u", umax),
            ("max_courant", self.last_courant),
        ]
    }
    fn is_finite(&self) -> bool {
        self.curr.is_finite() && self.prev.is_finite()
    }
    fn restart_state(&self) -> (Vec<RestartArray>, Vec<(String, f64)>) {
        let mut a = Vec::new();
        for (lvl, s) in [("prev", &self.prev), ("curr", &self.curr)] {
            for k in 0..self.nlev() {
                a.push((format!("{lvl}_vor_{k:03}"), s.vor[k].clone()));
                a.push((format!("{lvl}_div_{k:03}"), s.div[k].clone()));
                a.push((format!("{lvl}_tmp_{k:03}"), s.tmp[k].clone()));
                for (it, tq) in s.tracers.iter().enumerate() {
                    a.push((format!("{lvl}_tracer{it}_{k:03}"), tq[k].clone()));
                }
            }
            a.push((format!("{lvl}_lnps"), s.lnps.clone()));
        }
        (
            a,
            vec![
                ("steps".to_string(), self.steps as f64),
                ("mass_target".to_string(), self.mass_target),
            ],
        )
    }
    fn load_restart_state(
        &mut self,
        arrays: &[RestartArray],
        scalars: &[(String, f64)],
    ) -> Result<(), String> {
        let len = self.tr.layout.len();
        let get = |name: &str| -> Result<Vec<Complex>, String> {
            let a = arrays
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| format!("restart lacks {name}"))?;
            if a.1.len() != len {
                return Err(format!("{name}: wrong truncation"));
            }
            Ok(a.1.clone())
        };
        for lvl in ["prev", "curr"] {
            let ntr = self.curr.tracers.len();
            let mut s = PeState::zeros_with_tracers(self.nlev(), len, ntr);
            for k in 0..self.nlev() {
                s.vor[k] = get(&format!("{lvl}_vor_{k:03}"))?;
                s.div[k] = get(&format!("{lvl}_div_{k:03}"))?;
                s.tmp[k] = get(&format!("{lvl}_tmp_{k:03}"))?;
                for it in 0..ntr {
                    s.tracers[it][k] = get(&format!("{lvl}_tracer{it}_{k:03}"))?;
                }
            }
            s.lnps = get(&format!("{lvl}_lnps"))?;
            if lvl == "prev" {
                self.prev = s;
            } else {
                self.curr = s;
            }
        }
        for (k, v) in scalars {
            match k.as_str() {
                "steps" => self.steps = *v as u64,
                "mass_target" => self.mass_target = *v,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Build the primitive-equation model from a run configuration.
pub fn from_config(cfg: &crate::config::Config) -> Result<Primitive, String> {
    let planet = cfg.planet.resolve().map_err(|e| e.to_string())?;
    let levels = match (&cfg.sigma_half, cfg.levels) {
        (Some(h), _) => SigmaLevels::from_half(h.clone()),
        (None, Some(n)) => SigmaLevels::equal(n),
        (None, None) => return Err("primitive model needs `levels` or `sigma_half`".into()),
    };
    let tr = Transform::new(cfg.truncation, planet.radius);
    let init = cases::build(&tr, &planet, &levels, &cfg.initial_condition)?;
    let moisture = match &cfg.moisture {
        Some(m) => Some(m.resolve().map_err(|e| e.to_string())?),
        None => None,
    };
    let physics = crate::physics::build(cfg.forcing.as_ref(), &planet, moisture.as_ref())?;
    let mut init = init;
    if let Some(m) = &cfg.moisture {
        let q = cases::initial_humidity(&tr, &planet, &levels, &m.initial)?;
        init.state.tracers = vec![q];
    }
    let settings = PeSettings {
        dt: cfg.dt.seconds(),
        diffusion_order: cfg.diffusion.order,
        diffusion_tau: cfg.diffusion.tau.seconds(),
        filter: RawFilter {
            nu: cfg.time_filter.nu as Real,
            alpha: cfg.time_filter.alpha as Real,
        },
        mass_fixer: cfg.mass_fixer.unwrap_or(true),
    };
    let mut m = Primitive::new(tr, planet, settings, levels, init, physics);
    m.mass_target = m.mass_of(&m.curr.lnps);
    m.condensable = moisture;
    Ok(m)
}

#[cfg(test)]
mod tests;
