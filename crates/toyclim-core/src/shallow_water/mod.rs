//! Shallow-water model on the sphere (SPEC.md §5.1).
//!
//! Prognostic spectral variables: relative vorticity ζ, divergence δ and the
//! geopotential deviation φ' = φ − φ̄ (φ = g h, h the layer depth). Leapfrog with
//! semi-implicit gravity waves, RAW filter and implicit hyperdiffusion.

pub mod cases;

use crate::grid::GaussianGrid;
use crate::model::{GridField, Model, RestartArray};
use crate::planet::Planet;
use crate::spectral::ops::{hyperdiffusion_factors, laplacian};
use crate::spectral::{Complex, Transform};
use crate::time::leapfrog::{scale_by, RawFilter};
use crate::Real;

/// One time level of the spectral state.
#[derive(Debug, Clone)]
pub struct SwState {
    pub vor: Vec<Complex>,
    pub div: Vec<Complex>,
    /// φ' = g h − φ̄ \[m² s⁻²\].
    pub phi: Vec<Complex>,
}

impl SwState {
    fn fields(&self) -> [&Vec<Complex>; 3] {
        [&self.vor, &self.div, &self.phi]
    }
    fn is_finite(&self) -> bool {
        self.fields()
            .iter()
            .all(|f| f.iter().all(|c| c.re.is_finite() && c.im.is_finite()))
    }
}

/// Initial condition produced by a test case.
pub struct SwInitial {
    pub state: SwState,
    /// Mean geopotential φ̄ \[m² s⁻²\].
    pub phibar: Real,
    /// Surface geopotential g h_s on the grid \[m² s⁻²\].
    pub phis_grid: Vec<Real>,
    /// Coriolis parameter on the grid, if not 2Ω sin φ (Williamson test 2 with α ≠ 0
    /// rotates it with the flow).
    pub coriolis: Option<Vec<Real>>,
}

/// Numerical settings.
#[derive(Debug, Clone)]
pub struct SwSettings {
    pub dt: f64,
    pub diffusion_order: u32,
    /// Hyperdiffusion e-folding time at ℓ = M \[s\]; ≤ 0 disables it.
    pub diffusion_tau: f64,
    pub filter: RawFilter,
}

/// The shallow-water model.
pub struct ShallowWater {
    pub tr: Transform,
    pub planet: Planet,
    pub settings: SwSettings,
    pub phibar: Real,
    /// Surface geopotential, grid and spectral (truncated).
    pub phis_grid: Vec<Real>,
    pub phis: Vec<Complex>,
    pub prev: SwState,
    pub curr: SwState,
    /// Coriolis parameter f on the grid.
    coriolis: Vec<Real>,
    diff_leapfrog: Vec<Real>,
    diff_first: Vec<Real>,
    steps: u64,
    /// Max Courant number of the last step.
    pub last_courant: f64,
}

/// Grid-point quantities derived from one state.
struct Derived {
    u: Vec<Real>,
    v: Vec<Real>,
    vor: Vec<Real>,
    phi: Vec<Real>,
}

impl ShallowWater {
    pub fn new(tr: Transform, planet: Planet, settings: SwSettings, init: SwInitial) -> Self {
        let phis = tr.analysis(&init.phis_grid);
        let nlon = tr.nlon();
        let coriolis = init.coriolis.unwrap_or_else(|| {
            let mut c = tr.grid_zeros();
            for j in 0..tr.nlat() {
                let f = (2.0 * planet.rotation_rate * tr.grid.mu[j]) as Real;
                c[j * nlon..(j + 1) * nlon].fill(f);
            }
            c
        });
        let lay = &tr.layout;
        let diff_leapfrog = hyperdiffusion_factors(
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
        ShallowWater {
            phis,
            phis_grid: init.phis_grid,
            phibar: init.phibar,
            prev: init.state.clone(),
            curr: init.state,
            coriolis,
            diff_leapfrog,
            diff_first,
            steps: 0,
            last_courant: 0.0,
            tr,
            planet,
            settings,
        }
    }

    fn derived(&self, s: &SwState) -> Derived {
        let (u, v) = self.tr.uv_from_vort_div(&s.vor, &s.div);
        Derived {
            u,
            v,
            vor: self.tr.synthesis(&s.vor),
            phi: self.tr.synthesis(&s.phi),
        }
    }

    /// Explicit tendencies (N_ζ, N_δ, N_φ) at the current level; also returns the max
    /// Courant number.
    fn tendencies(&self, s: &SwState) -> (SwState, f64) {
        let tr = &self.tr;
        let d = self.derived(s);
        let n = tr.ngrid();
        let mut au = vec![0.0; n];
        let mut av = vec![0.0; n];
        let mut ke = vec![0.0; n];
        let mut pu = vec![0.0; n];
        let mut pv = vec![0.0; n];
        let nlon = tr.nlon();
        let mut courant = 0.0f64;
        let dlam = 2.0 * std::f64::consts::PI / nlon as f64;
        for j in 0..tr.nlat() {
            let ic2 = tr.inv_cos2[j];
            let cfl = self.settings.dt / (tr.radius as f64 * dlam);
            for k in j * nlon..(j + 1) * nlon {
                let (u, v) = (d.u[k], d.v[k]);
                let eta = d.vor[k] + self.coriolis[k];
                au[k] = eta * u;
                av[k] = eta * v;
                ke[k] = 0.5 * (u * u + v * v) * ic2;
                pu[k] = d.phi[k] * u;
                pv[k] = d.phi[k] * v;
                // |u| / cos φ = |U| / cos² φ
                courant = courant.max((u.abs() * ic2) as f64 * cfl);
            }
        }
        let (div_eta, curl_eta) = tr.div_curl(&au, &av);
        let mut ke_s = tr.analysis(&ke);
        for (e, p) in ke_s.iter_mut().zip(&self.phis) {
            *e += *p;
        }
        let lap = laplacian(&tr.layout, &ke_s, tr.radius);
        let n_vor: Vec<Complex> = div_eta.iter().map(|c| -c).collect();
        let n_div: Vec<Complex> = curl_eta.iter().zip(&lap).map(|(c, l)| c - l).collect();
        let n_phi: Vec<Complex> = tr.div(&pu, &pv).iter().map(|c| -c).collect();
        (
            SwState {
                vor: n_vor,
                div: n_div,
                phi: n_phi,
            },
            courant,
        )
    }

    /// Semi-implicit leapfrog update from `minus` with tendencies at the centre level,
    /// half-step `dt` (the step spans 2·dt).
    fn implicit_update(&self, minus: &SwState, n: &SwState, dt: Real) -> SwState {
        let lay = &self.tr.layout;
        let a2 = self.tr.radius * self.tr.radius;
        let len = lay.len();
        let mut out = SwState {
            vor: vec![Complex::new(0.0, 0.0); len],
            div: vec![Complex::new(0.0, 0.0); len],
            phi: vec![Complex::new(0.0, 0.0); len],
        };
        for k in 0..len {
            let l = lay.l_of[k] as Real;
            let lam = l * (l + 1.0) / a2;
            let rhs = minus.div[k] + (n.div[k] + (minus.phi[k] + n.phi[k] * dt) * lam) * dt;
            let x = rhs / (1.0 + dt * dt * lam * self.phibar);
            out.div[k] = x * 2.0 - minus.div[k];
            out.phi[k] = minus.phi[k] + (n.phi[k] - x * self.phibar) * (2.0 * dt);
            out.vor[k] = minus.vor[k] + n.vor[k] * (2.0 * dt);
        }
        out
    }

    /// Grid fields of a state, for output.
    pub fn grid_field(&self, name: &str) -> Option<Vec<Real>> {
        let tr = &self.tr;
        let nlon = tr.nlon();
        let g = self.planet.gravity as Real;
        let unweight = |x: Vec<Real>| -> Vec<Real> {
            let mut x = x;
            for j in 0..tr.nlat() {
                let c = (1.0 / (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt()) as Real;
                x[j * nlon..(j + 1) * nlon].iter_mut().for_each(|v| *v *= c);
            }
            x
        };
        Some(match name {
            "h" => self
                .tr
                .synthesis(&self.curr.phi)
                .iter()
                .map(|p| (p + self.phibar) / g)
                .collect(),
            // free-surface height h + h_s
            "eta" | "surface" => self
                .tr
                .synthesis(&self.curr.phi)
                .iter()
                .zip(&self.phis_grid)
                .map(|(p, s)| (p + self.phibar + s) / g)
                .collect(),
            "hs" => self.phis_grid.iter().map(|s| s / g).collect(),
            "vor" => self.tr.synthesis(&self.curr.vor),
            "div" => self.tr.synthesis(&self.curr.div),
            "u" | "v" => {
                let (u, v) = tr.uv_from_vort_div(&self.curr.vor, &self.curr.div);
                unweight(if name == "u" { u } else { v })
            }
            "pv" => {
                let vor = self.tr.synthesis(&self.curr.vor);
                let phi = self.tr.synthesis(&self.curr.phi);
                (0..tr.ngrid())
                    .map(|k| (vor[k] + self.coriolis[k]) * g / (phi[k] + self.phibar))
                    .collect()
            }
            _ => return None,
        })
    }

    /// Mass, total energy, potential enstrophy and axial angular momentum (SI, global).
    pub fn conservation(&self) -> [f64; 4] {
        let tr = &self.tr;
        let d = self.derived(&self.curr);
        let n = tr.ngrid();
        let nlon = tr.nlon();
        let g = self.planet.gravity;
        let a = tr.radius as f64;
        let om = self.planet.rotation_rate;
        let mut mass = vec![0.0 as Real; n];
        let mut energy = vec![0.0 as Real; n];
        let mut enstr = vec![0.0 as Real; n];
        let mut am = vec![0.0 as Real; n];
        for j in 0..tr.nlat() {
            let ic2 = tr.inv_cos2[j] as f64;
            let c2 = 1.0 - tr.grid.mu[j] * tr.grid.mu[j];
            for k in j * nlon..(j + 1) * nlon {
                let phi = (d.phi[k] + self.phibar) as f64;
                let (u, v) = (d.u[k] as f64, d.v[k] as f64);
                let ke = 0.5 * (u * u + v * v) * ic2;
                let eta = (d.vor[k] + self.coriolis[k]) as f64;
                let phis = self.phis_grid[k] as f64;
                mass[k] = (phi / g) as Real;
                energy[k] = ((phi * ke + 0.5 * phi * phi + phi * phis) / g) as Real;
                enstr[k] = (g * eta * eta / (2.0 * phi)) as Real;
                am[k] = (phi / g * (a * u + om * a * a * c2)) as Real;
            }
        }
        [
            tr.integrate(&mass),
            tr.integrate(&energy),
            tr.integrate(&enstr),
            tr.integrate(&am),
        ]
    }
}

impl Model for ShallowWater {
    fn kind(&self) -> &'static str {
        "shallow_water"
    }

    fn step(&mut self) {
        let first = self.steps == 0;
        let (n, courant) = self.tendencies(&self.curr);
        self.last_courant = courant;
        let dt = self.settings.dt as Real;
        let mut next = if first {
            self.implicit_update(&self.curr, &n, 0.5 * dt)
        } else {
            self.implicit_update(&self.prev, &n, dt)
        };
        let diff = if first {
            &self.diff_first
        } else {
            &self.diff_leapfrog
        };
        scale_by(&mut next.vor, diff);
        scale_by(&mut next.div, diff);
        scale_by(&mut next.phi, diff);
        let mut curr = std::mem::replace(&mut self.curr, next);
        if !first {
            let f = self.settings.filter;
            f.apply(&self.prev.vor, &mut curr.vor, &mut self.curr.vor);
            f.apply(&self.prev.div, &mut curr.div, &mut self.curr.div);
            f.apply(&self.prev.phi, &mut curr.phi, &mut self.curr.phi);
        }
        self.prev = curr;
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
    fn field_names(&self) -> Vec<&'static str> {
        vec!["h", "eta", "hs", "vor", "div", "u", "v", "pv"]
    }
    fn field(&self, name: &str) -> Option<GridField> {
        self.grid_field(name)
            .map(|data| GridField { nlev: 1, data })
    }
    fn diagnostics(&self) -> Vec<(&'static str, f64)> {
        let [mass, energy, enstrophy, am] = self.conservation();
        vec![
            ("mass", mass),
            ("energy", energy),
            ("enstrophy", enstrophy),
            ("angular_momentum", am),
            ("max_courant", self.last_courant),
        ]
    }
    fn is_finite(&self) -> bool {
        self.curr.is_finite() && self.prev.is_finite()
    }
    fn restart_state(&self) -> (Vec<RestartArray>, Vec<(String, f64)>) {
        let mut a = Vec::new();
        for (lvl, s) in [("prev", &self.prev), ("curr", &self.curr)] {
            a.push((format!("{lvl}_vor"), s.vor.clone()));
            a.push((format!("{lvl}_div"), s.div.clone()));
            a.push((format!("{lvl}_phi"), s.phi.clone()));
        }
        let scalars = vec![
            ("steps".to_string(), self.steps as f64),
            ("phibar".to_string(), self.phibar as f64),
        ];
        (a, scalars)
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
        for (lvl, which) in [("prev", 0), ("curr", 1)] {
            let s = SwState {
                vor: get(&format!("{lvl}_vor"))?,
                div: get(&format!("{lvl}_div"))?,
                phi: get(&format!("{lvl}_phi"))?,
            };
            if which == 0 {
                self.prev = s;
            } else {
                self.curr = s;
            }
        }
        for (k, v) in scalars {
            match k.as_str() {
                "steps" => self.steps = *v as u64,
                "phibar" => self.phibar = *v as Real,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Build a shallow-water model from a run configuration.
pub fn from_config(cfg: &crate::config::Config) -> Result<ShallowWater, String> {
    let planet = cfg.planet.resolve().map_err(|e| e.to_string())?;
    let tr = Transform::new(cfg.truncation, planet.radius);
    let init = cases::build(&tr, &planet, &cfg.initial_condition)?;
    let settings = SwSettings {
        dt: cfg.dt.seconds(),
        diffusion_order: cfg.diffusion.order,
        diffusion_tau: cfg.diffusion.tau.seconds(),
        filter: RawFilter {
            nu: cfg.time_filter.nu as Real,
            alpha: cfg.time_filter.alpha as Real,
        },
    };
    Ok(ShallowWater::new(tr, planet, settings, init))
}

#[cfg(test)]
mod tests;
