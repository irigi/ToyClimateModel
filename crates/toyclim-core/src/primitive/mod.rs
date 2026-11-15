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
use crate::physics::ColumnPhysics;
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
    pub condensable: Option<crate::physics::moist::Condensable>,
    /// Orbit and star (insolation); None → no insolation passed to physics.
    pub orbit: Option<Orbit>,
    /// Surface model; None → schemes use their own prescribed surface temperature.
    pub surface: Option<Surface>,
    /// Latest net surface heat flux \[W m⁻²\] (diagnostic; empty before the first step).
    pub surface_heat: Vec<Real>,
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
    precip: Vec<Real>,
    evap: Vec<Real>,
    /// Wall time per phase (see [`PHASES`]) when profiling is on.
    timers: Option<[f64; 9]>,
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
        physics: Vec<Box<dyn ColumnPhysics>>,
    ) -> Self {
        let (l, n) = (levels.nlev, tr.ngrid());
        let ntr = init.state.tracers.len();
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
            precip: vec![0.0; n],
            evap: vec![0.0; n],
            timers: None,
            prev: init.state.clone(),
            curr: init.state,
            coriolis_row,
            steps: 0,
            last_courant: 0.0,
            mass_target: 0.0,
            condensable: None,
            orbit: None,
            surface: None,
            surface_heat: Vec::new(),
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

    /// Shift ln p_s by a constant so the global mass equals `mass_target`.
    fn fix_mass(&mut self) {
        if self.mass_target <= 0.0 {
            return;
        }
        let m = self.mass_of(&self.curr.lnps);
        let shift = (self.mass_target / m).ln() * std::f64::consts::SQRT_2;
        self.curr.lnps[0].re += shift as Real;
    }

    /// Run the column physics on the n−1 level and advance the surface with its fluxes.
    fn physics_step(&mut self) {
        let cx = DriverCtx {
            tr: &self.tr,
            planet: &self.planet,
            phis_grid: &self.phis_grid,
            surface: self.surface.as_ref(),
            orbit: self.orbit.as_ref(),
            time: self.steps as f64 * self.settings.dt,
            dt: if self.steps == 0 {
                self.settings.dt
            } else {
                2.0 * self.settings.dt
            },
        };
        self.physics.run(&cx, &self.prev);
        let tend = &self.physics.tend;
        self.surface_heat.clear();
        self.surface_heat.extend_from_slice(&tend.surface_heat);
        if let Some(sfc) = self.surface.as_mut() {
            // precipitation and evaporation for the land bucket [kg m⁻² s⁻¹]
            let reg = &self.physics.registry;
            self.precip.fill(0.0);
            for &id in reg.precip_parts() {
                let g = &self.physics.diag[id.0];
                self.precip.iter_mut().zip(g).for_each(|(a, b)| *a += *b);
            }
            match reg.evap() {
                Some(id) => self.evap.copy_from_slice(&self.physics.diag[id.0]),
                None => self.evap.fill(0.0),
            }
            sfc.step(
                self.settings.dt,
                &tend.surface_heat,
                &self.precip,
                &self.evap,
            );
        }
    }
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
        let (lay, radius) = (&self.tr.layout, self.tr.radius);
        if first {
            let minus = &self.curr;
            self.si
                .update(lay, radius, minus, &self.ntend, 0.5 * dt, true, &mut next);
        } else {
            let minus = &self.prev;
            self.si
                .update(lay, radius, minus, &self.ntend, dt, false, &mut next);
        }
        lap.add(&mut t.as_mut(), 5);
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
        }
        self.spare = Some(std::mem::replace(&mut self.prev, curr));
        lap.add(&mut t.as_mut(), 6);
        if self.settings.mass_fixer {
            self.fix_mass();
        }
        let ds = &self.levels.dsigma;
        tracers::fix_negative(&self.tr, &self.curr.lnps, ds, &mut self.curr.tracers);
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
