//! Single-column model (SPEC.md §9.5.1): the physics driver's scheme chain and one surface
//! tile on one column with prescribed large-scale conditions (no dynamics).
//!
//! A case is a schema-2 world config with `model = "scm"` and an `[scm]` block:
//!
//! ```toml
//! [scm]
//! lat = 20.0                 # degrees
//! lon = 0.0
//! dt = "1h"                  # physics step Δt_p
//! t_air = 300.0              # initial temperature of the lowest level [K]
//! lapse_rate = 6.5           # K/km up to t_tropopause, isothermal above
//! t_tropopause = 200.0
//! rh = 0.5                   # initial relative humidity (moist cases)
//! wind = [5.0, 0.0]          # [u, v] m/s
//! wind_mode = "geostrophic"  # or "fixed" (wind reset to `wind` after every step)
//! [scm.surface]
//! kind = "land"              # "land" | "ocean"
//! heat_capacity = 2.0e5      # land [J m⁻² K⁻¹]; ocean: mixed_layer_depth [m]
//! fixed = false              # prescribed T_s
//! albedo = 0.3
//! beta = 1.0                 # evaporation efficiency (land)
//! t_initial = 300.0
//! ```
//!
//! "geostrophic": after the physics, the ageostrophic wind (u − u_g, v − v_g) is rotated by
//! the Coriolis angle −f Δt_p (exact inertial rotation; the external pressure gradient is
//! the large-scale forcing). Insolation comes from the config's `[star]`/`[orbit]`.
//! Each step records the energy and water residuals of column + surface against the
//! boundary fluxes the schemes report (§9.5.1 conservation test).

use crate::config::{Config, Duration, ModelKind};
use crate::physics::column::{ColumnCtx, ColumnState, SurfaceExchange, Workspace};
use crate::physics::{Condensable, Schemes};
use crate::planet::Planet;
use crate::primitive::vertical::SigmaLevels;
use crate::surface::{ocean_slab, Tile, TileKind};
use crate::system::orbit::Orbit;

/// Settings of the `[scm]` block.
#[derive(Debug, Clone)]
pub struct ScmSettings {
    pub lat: f64,
    pub lon: f64,
    pub dt: f64,
    pub t_air: f64,
    pub lapse_rate: f64,
    pub t_tropopause: f64,
    pub rh: f64,
    pub wind: [f64; 2],
    pub fixed_wind: bool,
    pub kind: TileKind,
    pub heat_capacity: f64,
    pub albedo: f64,
    pub beta: f64,
    pub t_surface: f64,
}

fn get_num(t: &toml::Table, k: &str, d: f64) -> Result<f64, String> {
    match t.get(k) {
        None => Ok(d),
        Some(v) => v
            .as_float()
            .or_else(|| v.as_integer().map(|i| i as f64))
            .ok_or_else(|| format!("scm.{k} must be a number")),
    }
}

impl ScmSettings {
    pub fn parse(t: &toml::Table) -> Result<Self, String> {
        const KEYS: &[&str] = &[
            "lat",
            "lon",
            "dt",
            "t_air",
            "lapse_rate",
            "t_tropopause",
            "rh",
            "wind",
            "wind_mode",
            "surface",
        ];
        if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
            return Err(format!("unknown key {k:?} in [scm]"));
        }
        let dt = match t.get("dt") {
            Some(toml::Value::String(s)) => s.parse::<Duration>().map_err(|e| e.to_string())?.0,
            _ => get_num(t, "dt", 1800.0)?,
        };
        let wind = match t.get("wind") {
            None => [0.0, 0.0],
            Some(v) => {
                let a = v.as_array().ok_or("scm.wind must be [u, v]")?;
                let x = |i: usize| {
                    a.get(i)
                        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                        .ok_or("scm.wind must be [u, v]")
                };
                [x(0)?, x(1)?]
            }
        };
        let fixed_wind = match t.get("wind_mode").and_then(|v| v.as_str()) {
            None | Some("geostrophic") => false,
            Some("fixed") => true,
            Some(o) => return Err(format!("unknown scm.wind_mode {o:?} (geostrophic, fixed)")),
        };
        let empty = toml::Table::new();
        let s = match t.get("surface") {
            Some(toml::Value::Table(s)) => s,
            None => &empty,
            _ => return Err("[scm.surface] must be a table".into()),
        };
        const SKEYS: &[&str] = &[
            "kind",
            "heat_capacity",
            "mixed_layer_depth",
            "fixed",
            "albedo",
            "beta",
            "t_initial",
        ];
        if let Some(k) = s.keys().find(|k| !SKEYS.contains(&k.as_str())) {
            return Err(format!("unknown key {k:?} in [scm.surface]"));
        }
        let kind = match s.get("kind").and_then(|v| v.as_str()).unwrap_or("ocean") {
            "ocean" => TileKind::Ocean,
            "land" => TileKind::Land,
            o => return Err(format!("unknown scm.surface.kind {o:?} (ocean, land)")),
        };
        let fixed = s.get("fixed").and_then(|v| v.as_bool()).unwrap_or(false);
        let heat_capacity = if fixed {
            f64::INFINITY
        } else if kind == TileKind::Ocean {
            ocean_slab::heat_capacity(get_num(s, "mixed_layer_depth", 50.0)?)
        } else {
            get_num(s, "heat_capacity", 2.0e5)?
        };
        let t_air = get_num(t, "t_air", 300.0)?;
        Ok(ScmSettings {
            lat: get_num(t, "lat", 0.0)?.to_radians(),
            lon: get_num(t, "lon", 0.0)?.to_radians(),
            dt,
            t_air,
            lapse_rate: get_num(t, "lapse_rate", 6.5)? * 1e-3,
            t_tropopause: get_num(t, "t_tropopause", 200.0)?,
            rh: get_num(t, "rh", 0.5)?,
            wind,
            fixed_wind,
            kind,
            heat_capacity,
            albedo: get_num(s, "albedo", 0.3)?,
            beta: get_num(s, "beta", 1.0)?,
            t_surface: get_num(s, "t_initial", t_air)?,
        })
    }
}

/// Column energy change Σ (c_p ΔT + Δ|v|²/2 + L Δq) Δp/g of `s` relative to (t0, u0, v0, q0),
/// level by level as differences \[J m⁻²\].
fn column_energy_change(
    s: &ColumnState,
    dp: &[f64],
    planet: &Planet,
    lv: f64,
    (t0, u0, v0, q0): (&[f64], &[f64], &[f64], &[f64]),
) -> f64 {
    let (cp, g) = (planet.cp, planet.gravity);
    (0..s.nlev)
        .map(|k| {
            let dq = if q0.is_empty() { 0.0 } else { s.q[k] - q0[k] };
            (cp * (s.t[k] - t0[k])
                + 0.5 * (s.u[k] * s.u[k] - u0[k] * u0[k] + s.v[k] * s.v[k] - v0[k] * v0[k])
                + lv * dq)
                * dp[k]
                / g
        })
        .sum()
}

/// Per-step record of the single-column model.
#[derive(Debug, Clone, Default)]
pub struct ScmRecord {
    pub time: f64,
    /// Surface temperature after the step \[K\].
    pub ts: f64,
    /// Diagnostics of the schemes (registry order).
    pub diag: Vec<f64>,
    /// Energy residual of column + surface against the reported boundary fluxes, and water
    /// residual \[W m⁻²\], \[kg m⁻² s⁻¹\].
    pub energy_residual: f64,
    pub water_residual: f64,
}

/// The single-column model.
pub struct Scm {
    pub settings: ScmSettings,
    pub planet: Planet,
    pub cond: Option<Condensable>,
    pub schemes: Schemes,
    pub names: Vec<&'static str>,
    pub orbit: Option<Orbit>,
    pub sigma: Vec<f64>,
    pub sigma_half: Vec<f64>,
    pub p: Vec<f64>,
    pub dp: Vec<f64>,
    pub ph: Vec<f64>,
    pub ps: f64,
    pub state: ColumnState,
    pub tile: Tile,
    /// Land bucket content \[kg m⁻²\] (budget only; β is fixed).
    pub bucket: f64,
    pub ustar: f64,
    pub time: f64,
    work: Workspace,
    sfc: SurfaceExchange,
}

impl Scm {
    /// Build from a parsed config with `model = "scm"`.
    pub fn from_config(cfg: &Config) -> Result<Self, String> {
        if cfg.model != ModelKind::Scm {
            return Err("not a single-column case (model = \"scm\")".into());
        }
        let st = ScmSettings::parse(cfg.scm.as_ref().ok_or("an scm case needs an [scm] block")?)?;
        let planet = cfg.planet.resolve().map_err(|e| e.to_string())?;
        let cond = cfg
            .moisture
            .as_ref()
            .map(|m| m.resolve())
            .transpose()
            .map_err(|e| e.to_string())?;
        let pc = cfg
            .physics
            .as_ref()
            .ok_or("an scm case needs the typed physics blocks")?;
        let sponge = cfg.dynamics.as_ref().and_then(|d| d.sponge.as_ref());
        let schemes = crate::physics::build_world(pc, sponge, cond.as_ref())?;
        let levels = match (&cfg.sigma_half, cfg.levels) {
            (Some(h), _) => SigmaLevels::from_half(h.clone()),
            (None, Some(n)) => SigmaLevels::equal(n),
            (None, None) => return Err("scm needs `levels` or `sigma_half`".into()),
        };
        let orbit = cfg.orbit.as_ref().map(|o| o.resolve(planet.rotation_rate));
        Ok(Self::new(st, planet, cond, schemes, levels, orbit))
    }

    pub fn new(
        st: ScmSettings,
        planet: Planet,
        cond: Option<Condensable>,
        schemes: Schemes,
        levels: SigmaLevels,
        orbit: Option<Orbit>,
    ) -> Self {
        let l = levels.nlev;
        let ps = planet.p0;
        let p: Vec<f64> = levels.full.iter().map(|s| s * ps).collect();
        let dp: Vec<f64> = levels.dsigma.iter().map(|d| d * ps).collect();
        let ph: Vec<f64> = levels.half.iter().map(|s| s * ps).collect();
        let ntr = usize::from(cond.is_some());
        let mut state = ColumnState::new(l, ntr);
        // initial profile: lapse rate from t_air at the lowest level, isothermal above the
        // tropopause; heights from the hydrostatic relation
        let (g, r) = (planet.gravity, planet.gas_constant);
        let mut z = 0.0;
        for k in (0..l).rev() {
            let t = (st.t_air - st.lapse_rate * z).max(st.t_tropopause);
            state.t[k] = t;
            if k > 0 {
                z += r * t / g * (p[k] / p[k - 1]).ln();
            }
            state.u[k] = st.wind[0];
            state.v[k] = st.wind[1];
            if let Some(c) = &cond {
                state.q[k] = st.rh * c.qsat(&planet, t, p[k]);
            }
        }
        let names = schemes
            .iter()
            .flat_map(|s| s.diagnostics().iter().copied())
            .collect();
        let mut tile = Tile::new(st.kind, 1.0, st.t_surface, st.heat_capacity, st.albedo);
        tile.beta = if st.kind == TileKind::Land {
            st.beta
        } else {
            1.0
        };
        Scm {
            planet,
            cond,
            schemes,
            names,
            orbit,
            sigma: levels.full.clone(),
            sigma_half: levels.half.clone(),
            p,
            dp,
            ph,
            ps,
            state,
            tile,
            bucket: 0.0,
            ustar: 0.0,
            time: 0.0,
            work: Workspace::new(l),
            sfc: SurfaceExchange::default(),
            settings: st,
        }
    }

    /// One physics step with `schemes` (indices into [`Self::schemes`]; all = the full
    /// chain). Returns the record.
    pub fn step_with(&mut self, which: &[usize]) -> ScmRecord {
        let st = &self.settings;
        let dt = st.dt;
        let (insolation, cos_zenith) = match &self.orbit {
            Some(o) => {
                let sun = o.sun(self.time);
                (
                    o.insolation(&sun, st.lat, st.lon),
                    o.cos_zenith(&sun, st.lat, st.lon),
                )
            }
            None => (0.0, 0.0),
        };
        let ctx = ColumnCtx {
            lat: st.lat,
            lon: st.lon,
            index: 0,
            sigma: &self.sigma,
            sigma_half: &self.sigma_half,
            ps: self.ps,
            p: &self.p,
            dp: &self.dp,
            p_half: &self.ph,
            phis: 0.0,
            time: self.time,
            dt,
            planet: &self.planet,
            cond: self.cond.as_ref(),
            insolation,
            cos_zenith,
        };
        let s = &mut self.state;
        s.begin(dt);
        let (t0, u0, v0, q0) = (s.t.clone(), s.u.clone(), s.v.clone(), s.q.clone());
        let sfc = &mut self.sfc;
        sfc.begin();
        sfc.tiles.clear();
        self.tile.ts_new = self.tile.ts;
        sfc.tiles.push(self.tile);
        sfc.ustar_prev = self.ustar;
        let mut diag = vec![0.0; self.names.len()];
        let mut off = 0;
        for (i, sc) in self.schemes.iter().enumerate() {
            let n = sc.diagnostics().len();
            if which.contains(&i) {
                sc.apply(&ctx, s, sfc, &mut diag[off..off + n], &mut self.work);
            }
            off += n;
        }
        if let Some(i) = self.names.iter().position(|n| *n == "olr") {
            diag[i] += sfc.lw_up_change;
        }
        // budgets: column + surface against the boundary fluxes the schemes reported
        let t_new = sfc.tiles[0].ts_new;
        let c = self.tile.heat_capacity;
        let d_sfc = if c.is_finite() {
            c * (t_new - self.tile.ts)
        } else {
            // prescribed temperature: the ocean below takes the net flux
            sfc.sfc_net * dt
        };
        let radiation = which
            .iter()
            .any(|&i| self.schemes[i].name() == "gray_radiation");
        let coupled = which
            .iter()
            .any(|&i| self.schemes[i].name() == "kprofile_boundary_layer");
        // energy entering through the top; without the surface solve, radiation's surface
        // absorption leaves the column; without radiation, the surface solve's radiative
        // input (from the given sw_down/lw_down, zero here) enters at the bottom
        let mut boundary = sfc.toa_net - sfc.lw_up_change;
        let t = &sfc.tiles[0];
        let sfc_rad = sfc.sw_down * (1.0 - t.albedo) + sfc.lw_down
            - crate::physics::radiation::STEFAN_BOLTZMANN * t.ts.powi(4);
        if radiation && !coupled {
            boundary -= sfc_rad;
        } else if coupled && !radiation {
            boundary += sfc_rad;
        }
        boundary += self.tile.qflux;
        let lv = self.cond.as_ref().map_or(0.0, |c| c.latent_heat);
        // snow reaches the surface with the latent-heat deficit L_f per kg (the atmosphere
        // kept the heat of freezing)
        let lf = self.cond.as_ref().map_or(0.0, |c| c.latent_heat_fusion);
        let d_sfc = d_sfc - lf * sfc.snow * dt;
        let d_col = column_energy_change(s, &self.dp, &self.planet, lv, (&t0, &u0, &v0, &q0));
        let energy_residual = (d_col + d_sfc) / dt - boundary;
        // water: column change = E − P (the ocean is an infinite reservoir; the land
        // bucket takes P − E)
        let mut water_residual = 0.0;
        if self.cond.is_some() {
            let dq: f64 = (0..s.nlev)
                .map(|k| (s.q[k] - q0[k]) * self.dp[k] / self.planet.gravity)
                .sum();
            water_residual = dq / dt - (sfc.evap - sfc.rain - sfc.snow);
        }
        // advance the surface and the large-scale forcing
        if c.is_finite() {
            self.tile.ts = t_new;
        }
        if self.tile.kind == TileKind::Land {
            self.bucket += dt * (sfc.rain + sfc.snow - sfc.evap);
        }
        if which
            .iter()
            .any(|&i| self.schemes[i].name() == "kprofile_boundary_layer")
        {
            self.ustar = sfc.ustar;
        }
        let [ug, vg] = st.wind;
        if st.fixed_wind {
            s.u.fill(ug);
            s.v.fill(vg);
        } else {
            let f = 2.0 * self.planet.rotation_rate * st.lat.sin();
            let (c, sn) = ((f * dt).cos(), (f * dt).sin());
            for k in 0..s.nlev {
                let (a, b) = (s.u[k] - ug, s.v[k] - vg);
                s.u[k] = ug + c * a + sn * b;
                s.v[k] = vg - sn * a + c * b;
            }
        }
        self.time += dt;
        ScmRecord {
            time: self.time,
            ts: self.tile.ts,
            diag,
            energy_residual,
            water_residual,
        }
    }

    /// One step of the full chain.
    pub fn step(&mut self) -> ScmRecord {
        let all: Vec<usize> = (0..self.schemes.len()).collect();
        self.step_with(&all)
    }

    /// Diagnostic index by name.
    pub fn diag_index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| *n == name)
    }
}

#[cfg(test)]
mod tests;
