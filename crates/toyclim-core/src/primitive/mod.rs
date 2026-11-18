//! Primitive equations in σ coordinates (SPEC.md §5.2–5.3).
//!
//! Prognostic spectral variables per level k (0 = top): vorticity ζ_k, divergence δ_k,
//! temperature deviation T'_k = T_k − T̄; plus ln p_s and tracers. Lorenz staggering with
//! the Simmons & Burridge (1981) energy- and angular-momentum-conserving discretization.
//! Semi-implicit leapfrog (Hoskins & Simmons 1975), RAW filter, implicit hyperdiffusion.
//! Column physics is evaluated at the n−1 level (stable for damping terms).
//!
//! This module knows nothing about what the surface or the physics is: it hands the
//! physics driver a state and receives total tendencies (SPEC.md §9.3.1).

pub mod cases;
pub mod implicit;
pub mod levels;
mod output;
mod restart;
pub(crate) mod semi_lagrangian;
pub mod setup;
pub mod state;
pub mod tendencies;
pub mod tracers;
pub mod vertical;

pub use levels::PRESSURE_FIELDS;
pub use setup::from_config;
pub use state::{Field3, Grid3, PeInitial, PeState, Spec3};

use crate::grid::GaussianGrid;
use crate::model::{GridField, Model, RestartArray};
use crate::physics::driver::{DriverCtx, PhysicsDriver};
use crate::physics::ColumnScheme;
use crate::planet::Planet;
use crate::spectral::ops::hyperdiffusion_factors;
use crate::spectral::{Complex, Transform};
use crate::surface::Surface;
use crate::system::orbit::Orbit;
use crate::time::leapfrog::{scale_by, RawFilter};
use crate::Real;
use implicit::SemiImplicit;
use tendencies::{ColumnConsts, DynCtx, DynWork};
use vertical::SigmaLevels;

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
    /// Return the kinetic energy removed by hyperdiffusion as a uniform heating
    /// (SPEC.md §9.4.3; off for Part I cases).
    pub energy_fixer: bool,
}

/// Time integrals of global physics fluxes since the start of the run \[J\] or \[kg\],
/// accumulated every step (restart state).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Budget {
    /// ∫∫ net downward flux at the top of the atmosphere dA dt.
    pub toa_net: f64,
    /// ∫∫ net flux into the surface dA dt.
    pub sfc_net: f64,
    /// ∫∫ precipitation and evaporation dA dt.
    pub precip: f64,
    pub evap: f64,
    /// Kinetic energy removed by hyperdiffusion (returned as heat when the energy fixer is
    /// on).
    pub diffusion_ke: f64,
    /// Water mass moved by the negative-humidity fixer (SPEC.md §9.8 monitor) \[kg\].
    pub fixer: f64,
    /// Surface: runoff and water into the ocean reservoir \[kg\]; energy gained by
    /// discarding frozen mass above the caps and prescribed ocean heat convergence \[J\].
    pub runoff: f64,
    pub ocean_uptake: f64,
    pub ice_cap: f64,
    pub qflux: f64,
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
    /// Column physics and its diagnostics.
    pub physics: PhysicsDriver,
    /// The condensable of tracer 0, if the model is moist.
    pub condensable: Option<crate::physics::Condensable>,
    /// Accumulated global fluxes (energy and water budgets).
    pub budget: Budget,
    /// Orbit and star (insolation); None → no insolation passed to physics.
    pub orbit: Option<Orbit>,
    /// Surface model; None → schemes use their own prescribed surface temperature.
    pub surface: Option<Surface>,
    pub last_courant: f64,
    /// Target global mass for the mass fixer \[kg\] (set from the initial state).
    pub mass_target: f64,
    si: SemiImplicit,
    consts: ColumnConsts,
    coriolis_row: Vec<Real>,
    dsigma: Vec<Real>,
    diff_leap: Vec<Real>,
    diff_first: Vec<Real>,
    steps: u64,
    /// Reused buffers: dynamics workspace, explicit tendencies, the recycled state.
    work: DynWork,
    ntend: PeState,
    spare: Option<PeState>,
    /// Scratch of the semi-implicit solve.
    isol: Vec<Complex>,
    /// ∫ f² dA / Σ_m w_m |f_ℓm|² of the transform's spectral normalisation \[m²\].
    ke_norm: f64,
    /// Wall time per phase (see [`PHASES`]) when profiling is on.
    timers: Option<[f64; 9]>,
    /// Semi-Lagrangian transport of the grid-point tracers (`PeState::gq`), if used.
    sl: Option<semi_lagrangian::SemiLagrangian>,
    /// Global mass of each grid-point tracer after physics, before transport (target of
    /// the mass fixer) \[Pa m²\].
    sl_target: Vec<f64>,
    /// Consecutive steps with specific humidity above 0.1 somewhere (SPEC.md §9.4.4).
    q_high_steps: u64,
}

/// Phases reported by [`Model::profile`].
pub const PHASES: [&str; 9] = [
    "physics",
    "dyn_synthesis",
    "dyn_grid_kernel",
    "dyn_tracers",
    "dyn_analysis",
    "implicit",
    "diffusion_filter",
    "fixers",
    "other",
];

impl Primitive {
    pub fn new(
        tr: Transform,
        planet: Planet,
        settings: PeSettings,
        levels: SigmaLevels,
        init: PeInitial,
        physics: Vec<Box<dyn ColumnScheme>>,
    ) -> Self {
        let (l, n) = (levels.nlev, tr.ngrid());
        let ntr = init.state.ntracers();
        let lay = &tr.layout;
        let (order, tau) = (settings.diffusion_order, settings.diffusion_tau);
        let coriolis_row = tr
            .grid
            .mu
            .iter()
            .map(|m| (2.0 * planet.rotation_rate * m) as Real)
            .collect();
        Primitive {
            si: SemiImplicit::new(&planet, &levels, tr.mmax(), settings.dt),
            consts: ColumnConsts::new(&levels, &planet),
            diff_leap: hyperdiffusion_factors(lay, order, tau, 2.0 * settings.dt),
            diff_first: hyperdiffusion_factors(lay, order, tau, settings.dt),
            dsigma: levels.dsigma.iter().map(|&x| x as Real).collect(),
            phis: tr.analysis(&init.phis_grid),
            phis_grid: init.phis_grid,
            physics: PhysicsDriver::new(physics, &tr, &levels, ntr),
            work: DynWork::new(l, n, ntr),
            ntend: PeState::zeros_with_tracers(l, lay.len(), ntr),
            spare: None,
            timers: None,
            sl: (!init.state.gq.is_empty())
                .then(|| semi_lagrangian::SemiLagrangian::new(&tr.grid, tr.radius as f64)),
            sl_target: Vec::new(),
            q_high_steps: 0,
            isol: Vec::new(),
            ke_norm: harmonic_norm(&tr),
            prev: init.state.clone(),
            curr: init.state,
            coriolis_row,
            steps: 0,
            last_courant: 0.0,
            mass_target: 0.0,
            condensable: None,
            budget: Budget::default(),
            orbit: None,
            surface: None,
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

    /// ∫ Σ_k q_k Δσ_k p_s dA of a grid-point tracer \[Pa m²\] (= g × mass).
    fn grid_tracer_mass(&self, q: &Grid3, lnps: &[Complex]) -> f64 {
        let ps: Vec<Real> = self.tr.synthesis(lnps).iter().map(|x| x.exp()).collect();
        let mut w = vec![0.0 as Real; ps.len()];
        for k in 0..self.nlev() {
            let ds = self.levels.dsigma[k] as Real;
            for (x, (qq, p)) in w.iter_mut().zip(q[k].iter().zip(&ps)) {
                *x += qq * ds * p;
            }
        }
        self.tr.integrate(&w)
    }

    /// Semi-Lagrangian step of the grid-point tracers into `next`: Q* = q^{base} + τ·P
    /// (physics), carried to n+1 by the level-n winds of the dynamics workspace. Records
    /// the mass of Q* for the fixer; returns the water mass removed where Q* < 0 (round-off
    /// of the physics) \[Pa m²\].
    fn sl_step(&mut self, first: bool, next: &mut PeState) -> f64 {
        let Some(sl) = &self.sl else {
            return 0.0;
        };
        let (base, tau) = if first {
            (&self.curr, self.settings.dt)
        } else {
            (&self.prev, 2.0 * self.settings.dt)
        };
        let phys = (!self.physics.is_empty()).then_some(&self.physics.tend);
        let mut targets = Vec::with_capacity(base.gq.len());
        let mut clipped0 = 0.0;
        for it in 0..base.gq.len() {
            let mut qs = base.gq[it].clone();
            if let Some(ph) = phys {
                for (x, y) in qs.as_mut_slice().iter_mut().zip(ph.dq[it].as_slice()) {
                    *x += tau as Real * y;
                }
            }
            let before = self.grid_tracer_mass(&qs, &base.lnps);
            qs.as_mut_slice().iter_mut().for_each(|x| *x = x.max(0.0));
            let after = self.grid_tracer_mass(&qs, &base.lnps);
            if it == 0 {
                clipped0 = after - before;
            }
            targets.push(after);
            semi_lagrangian::transport(
                sl,
                &qs,
                &self.work.grids.u,
                &self.work.grids.v,
                &self.work.sdl,
                &self.levels.full,
                &self.levels.dsigma,
                tau,
                &mut next.gq[it],
            );
        }
        self.sl_target = targets;
        clipped0
    }

    /// Multiplicative global mass fixer of the grid-point tracers of the new state;
    /// returns the absolute mass correction of tracer 0 \[Pa m²\].
    fn sl_fix_mass(&mut self) -> f64 {
        let mut moved0 = 0.0;
        for it in 0..self.sl_target.len() {
            let m = self.grid_tracer_mass(&self.curr.gq[it], &self.curr.lnps);
            let target = self.sl_target[it];
            if m > 0.0 {
                let scale = (target / m) as Real;
                self.curr.gq[it]
                    .as_mut_slice()
                    .iter_mut()
                    .for_each(|x| *x *= scale);
            }
            if it == 0 {
                moved0 = (target - m).abs();
            }
        }
        moved0
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

    /// Run the column physics on the n−1 level, advance the surface with its fluxes and
    /// accumulate the global budgets.
    fn physics_step(&mut self) {
        let dtp = if self.steps == 0 {
            self.settings.dt
        } else {
            2.0 * self.settings.dt
        };
        if let Some(sfc) = self.surface.as_mut() {
            sfc.prescribe(self.steps as f64 * self.settings.dt);
        }
        let cx = DriverCtx {
            tr: &self.tr,
            planet: &self.planet,
            cond: self.condensable.as_ref(),
            phis_grid: &self.phis_grid,
            surface: self.surface.as_ref(),
            orbit: self.orbit.as_ref(),
            time: self.steps as f64 * self.settings.dt,
            dt: dtp,
        };
        self.physics.run(&cx, &self.prev);
        let dt = self.settings.dt;
        if let Some(sfc) = self.surface.as_mut() {
            sfc.apply_physics(&self.physics.exchange, dt / dtp, dt);
        }
        if let Some(sfc) = &self.surface {
            let int = |x: &[f64]| {
                let g: Vec<Real> = x.iter().map(|&v| v as Real).collect();
                self.tr.integrate(&g) * dt
            };
            let f = &sfc.fluxes;
            self.budget.runoff += int(&f.runoff);
            self.budget.ocean_uptake += int(&f.ocean_uptake);
            self.budget.ice_cap += int(&f.ice_cap);
            self.budget.qflux += int(&f.qflux);
        }
        use crate::physics::driver::Exchange as X;
        let mut sums = [0.0f64; 4];
        let ex = &self.physics.exchange;
        if !ex.is_empty() {
            let nlon = self.tr.nlon();
            for j in 0..self.tr.nlat() {
                let w = self.tr.grid.weights[j];
                let mut row = [0.0f64; 4];
                for p in j * nlon..(j + 1) * nlon {
                    let e = &ex[p * X::N..(p + 1) * X::N];
                    row[0] += e[X::TOA_NET];
                    row[1] += e[X::SFC_NET];
                    row[2] += e[X::PRECIP];
                    row[3] += e[X::EVAP];
                }
                for (s, r) in sums.iter_mut().zip(row) {
                    *s += w * r;
                }
            }
        }
        // ∫ x dA = a² · 2π/nlon · Σ_j w_j Σ_i x
        let a = self.planet.radius;
        let area = a * a * std::f64::consts::TAU / self.tr.nlon() as f64 * dt;
        self.budget.toa_net += sums[0] * area;
        self.budget.sfc_net += sums[1] * area;
        self.budget.precip += sums[2] * area;
        self.budget.evap += sums[3] * area;
    }

    /// Whether the hyperdiffusion's kinetic-energy loss is tracked (world physics) or
    /// returned as heat (energy fixer).
    fn track_diffusion(&self) -> bool {
        self.settings.energy_fixer || self.surface.is_some()
    }

    /// Global kinetic energy of the spectral winds of `s` with p_s = p₀ \[J\]:
    /// ½ p₀/g Σ_k Δσ_k · C a² Σ_{ℓ,m} w_m (|ζ|² + |δ|²)/(ℓ(ℓ+1)), C = [`harmonic_norm`]
    /// (from ∫|∇ψ|² dA = ∫ ψ(−∇²ψ) dA and ζ = −ℓ(ℓ+1)ψ/a²).
    fn spectral_ke(&self, s: &PeState) -> f64 {
        let lay = &self.tr.layout;
        let a2 = (self.tr.radius as f64).powi(2);
        let mut ke = 0.0;
        for k in 0..self.nlev() {
            let mut lev = 0.0;
            for (i, &l) in lay.l_of.iter().enumerate() {
                if l == 0 {
                    continue;
                }
                let w = if lay.m_of[i] == 0 { 1.0 } else { 2.0 };
                let (z, d) = (s.vor[k][i], s.div[k][i]);
                lev += w * (z.norm_sqr() + d.norm_sqr()) as f64 / (l * (l + 1)) as f64;
            }
            ke += self.levels.dsigma[k] * lev;
        }
        0.5 * a2 * ke * self.planet.p0 / self.planet.gravity * self.ke_norm
    }
}

/// ∫ f² dA / Σ_m w_m |f_ℓm|² (w = 1 for m = 0, 2 otherwise) of the transform, measured on
/// one harmonic \[m²\].
fn harmonic_norm(tr: &Transform) -> f64 {
    let lay = &tr.layout;
    let mut c = lay.zeros();
    c[lay.idx(2, 1)] = Complex::new(1.0, 0.5);
    let f = tr.synthesis(&c);
    let f2: Vec<Real> = f.iter().map(|x| x * x).collect();
    tr.integrate(&f2) / (2.0 * 1.25)
}

impl Model for Primitive {
    fn kind(&self) -> &'static str {
        "primitive"
    }

    fn step(&mut self) {
        let first = self.steps == 0;
        let t_start = std::time::Instant::now();
        let mut t = self.timers.map(|_| [0.0f64; 9]);
        let mut lap = tendencies::Lap::new();
        if !self.physics.is_empty() {
            self.physics_step();
            // the physics saw the grid humidity of the n−1 level
            let q_max = self.physics.max_humidity();
            if q_max > 0.1 {
                self.q_high_steps += 1;
            } else {
                self.q_high_steps = 0;
            }
        }
        lap.add(&mut t.as_mut(), 0);
        let phys = (!self.physics.is_empty()).then_some(&self.physics.tend);
        let cx = DynCtx {
            tr: &self.tr,
            consts: &self.consts,
            si: &self.si,
            coriolis_row: &self.coriolis_row,
            phis: &self.phis,
            dsigma: &self.dsigma,
            dt: self.settings.dt,
        };
        let mut dyn_t = t.map(|_| [0.0f64; 4]);
        self.last_courant = tendencies::explicit(
            &cx,
            &self.curr,
            phys,
            &mut self.work,
            &mut self.ntend,
            dyn_t.as_mut(),
        );
        lap = tendencies::Lap::new();
        let dt = self.settings.dt as Real;
        let mut next = match self.spare.take() {
            Some(s) if s.same_shape(&self.curr) => s,
            _ => self.curr.clone(),
        };
        let clipped = self.sl_step(first, &mut next);
        let (lay, radius) = (&self.tr.layout, self.tr.radius);
        if first {
            let minus = &self.curr;
            self.si.update(
                lay,
                radius,
                minus,
                &self.ntend,
                0.5 * dt,
                true,
                &mut next,
                &mut self.isol,
            );
        } else {
            let minus = &self.prev;
            self.si.update(
                lay,
                radius,
                minus,
                &self.ntend,
                dt,
                false,
                &mut next,
                &mut self.isol,
            );
        }
        lap.add(&mut t.as_mut(), 5);
        let diff = if first {
            &self.diff_first
        } else {
            &self.diff_leap
        };
        let ke_before = if self.track_diffusion() {
            self.spectral_ke(&next)
        } else {
            0.0
        };
        for k in 0..self.nlev() {
            scale_by(&mut next.vor[k], diff);
            scale_by(&mut next.div[k], diff);
            scale_by(&mut next.tmp[k], diff);
            for tq in next.tracers.iter_mut() {
                scale_by(&mut tq[k], diff);
            }
        }
        if self.track_diffusion() {
            let removed = ke_before - self.spectral_ke(&next);
            self.budget.diffusion_ke += removed;
            if self.settings.energy_fixer {
                // uniform heating c_p ΔT M = removed kinetic energy (ℓ = 0 coefficient)
                let mass = self.planet.p0 / self.planet.gravity
                    * 4.0
                    * std::f64::consts::PI
                    * self.planet.radius
                    * self.planet.radius;
                let dtemp = removed / (self.planet.cp * mass) * std::f64::consts::SQRT_2;
                for k in 0..self.nlev() {
                    next.tmp[k][0].re += dtemp as Real;
                }
            }
        }
        let mut curr = std::mem::replace(&mut self.curr, next);
        if !first {
            // water mass of the grid-point tracers before the filter (the filters of ln p_s
            // and of q both change ∫ q dp; restored below)
            let gq_mass: Vec<f64> = curr
                .gq
                .iter()
                .map(|q| self.grid_tracer_mass(q, &curr.lnps))
                .collect();
            let f = self.settings.filter;
            let (p, c) = (&self.prev, &mut self.curr);
            f.apply(
                p.vor.as_slice(),
                curr.vor.as_mut_slice(),
                c.vor.as_mut_slice(),
            );
            f.apply(
                p.div.as_slice(),
                curr.div.as_mut_slice(),
                c.div.as_mut_slice(),
            );
            f.apply(
                p.tmp.as_slice(),
                curr.tmp.as_mut_slice(),
                c.tmp.as_mut_slice(),
            );
            f.apply(&p.lnps, &mut curr.lnps, &mut c.lnps);
            for it in 0..curr.tracers.len() {
                f.apply(
                    p.tracers[it].as_slice(),
                    curr.tracers[it].as_mut_slice(),
                    c.tracers[it].as_mut_slice(),
                );
            }
            // grid-point tracers: classic Robert–Asselin (α = 1, x^{n+1} untouched), whose
            // weights are all non-negative, so the filter cannot create negative values
            let h = 0.5 * f.nu;
            for it in 0..curr.gq.len() {
                let (pq, nq) = (p.gq[it].as_slice(), c.gq[it].as_slice());
                for ((x, a), b) in curr.gq[it].as_mut_slice().iter_mut().zip(pq).zip(nq) {
                    *x += h * (*a - 2.0 * *x + *b);
                }
            }
            // The filter is not mass-conserving: it changes the global tracer mass by the
            // time second difference of the mass, which the physics' 2Δt noise feeds (P4
            // diagnosis: 1.8e-3 mm/day of water at T21 with semigray radiation, the whole
            // water-closure residual). Restore the pre-filter mass multiplicatively; the
            // local smoothing stays.
            for (it, &m0) in gq_mass.iter().enumerate() {
                let m1 = self.grid_tracer_mass(&curr.gq[it], &curr.lnps);
                if m1 > 0.0 {
                    let scale = (m0 / m1) as Real;
                    curr.gq[it]
                        .as_mut_slice()
                        .iter_mut()
                        .for_each(|x| *x *= scale);
                }
            }
        }
        self.spare = Some(std::mem::replace(&mut self.prev, curr));
        lap.add(&mut t.as_mut(), 6);
        if self.settings.mass_fixer {
            self.fix_mass();
        }
        let ds = &self.levels.dsigma;
        let moved = tracers::fix_negative(&self.tr, &self.curr.lnps, ds, &mut self.curr.tracers);
        let moved_sl = self.sl_fix_mass();
        self.budget.fixer += (moved + moved_sl + clipped) / self.planet.gravity;
        lap.add(&mut t.as_mut(), 7);
        self.steps += 1;
        if let (Some(acc), Some(t), Some(d)) = (self.timers.as_mut(), t, dyn_t) {
            let parts = [t[0], d[0], d[1], d[2], d[3], t[5], t[6], t[7]];
            for (a, x) in acc.iter_mut().zip(parts) {
                *a += x;
            }
            acc[8] += t_start.elapsed().as_secs_f64() - parts.iter().sum::<f64>();
        }
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
        self.output_field_names()
    }
    fn field(&self, name: &str) -> Option<GridField> {
        self.output_field(name)
    }
    fn diagnostics(&self) -> Vec<(&'static str, f64)> {
        self.output_diagnostics()
    }
    fn is_finite(&self) -> bool {
        self.curr.is_finite() && self.prev.is_finite()
    }
    fn failure(&self) -> Option<String> {
        (self.q_high_steps > 100).then(|| {
            format!(
                "specific humidity above 0.1 kg/kg for {} consecutive steps: the atmosphere is \
                 outside the dilute envelope this model supports (SPEC.md §9.4.4)",
                self.q_high_steps
            )
        })
    }
    fn set_profiling(&mut self, on: bool) {
        self.timers = on.then_some([0.0; 9]);
    }
    fn profile(&self) -> Vec<(&'static str, f64)> {
        self.timers
            .map(|t| PHASES.iter().copied().zip(t).collect())
            .unwrap_or_default()
    }
    fn restart_state(&self) -> (Vec<RestartArray>, Vec<(String, f64)>) {
        self.save_restart()
    }
    fn load_restart_state(
        &mut self,
        arrays: &[RestartArray],
        scalars: &[(String, f64)],
    ) -> Result<(), String> {
        self.load_restart(arrays, scalars)
    }
}

#[cfg(test)]
mod tests;
