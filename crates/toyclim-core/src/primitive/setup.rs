//! Construction of the primitive-equation model from a run configuration.

use super::cases;
use super::state::PeInitial;
use super::vertical::SigmaLevels;
use super::{PeSettings, Primitive};
use crate::config::{Config, SurfaceConfig};
use crate::spectral::{Complex, Transform};
use crate::surface::{geography, ocean_slab, Surface};
use crate::time::leapfrog::RawFilter;
use crate::Real;

/// Global-mean surface pressure of a spectral ln p_s \[Pa\] (from the ℓ = 0 coefficient).
fn init_ps_mean(lnps: &[Complex]) -> f64 {
    ((lnps[0].re as f64) / std::f64::consts::SQRT_2).exp()
}

/// A zonally symmetric surface temperature profile on the grid.
fn zonal_grid(tr: &Transform, f: impl Fn(f64) -> f64) -> Vec<Real> {
    let mut v = Vec::with_capacity(tr.ngrid());
    for &lat in &tr.grid.lat {
        v.extend(std::iter::repeat_n(f(lat) as Real, tr.nlon()));
    }
    v
}

/// The TJ16 SST profile on the grid.
fn tj16_sst_grid(tr: &Transform) -> Vec<Real> {
    let sst = crate::physics::idealized::tj16::SurfaceTemperature::tj16();
    zonal_grid(tr, |lat| sst.at(lat, 0.0))
}

/// The "Control" SST of the Aqua-Planet Experiment (Neale & Hoskins 2000, Atmos. Sci.
/// Lett. 1, 101–107; APE SST specification, www.met.reading.ac.uk/~mike/APE):
/// T_s = 27 (1 − sin²(3φ/2)) °C for |φ| < π/3, 0 °C elsewhere \[K\].
pub fn ape_control_sst(lat: f64) -> f64 {
    let t = if lat.abs() < std::f64::consts::FRAC_PI_3 {
        27.0 * (1.0 - (1.5 * lat).sin().powi(2))
    } else {
        0.0
    };
    273.15 + t
}

/// Build the surface of `sc`. With a heightmap, also sets the orography of `init` and a
/// hydrostatically adjusted initial surface pressure.
fn build_surface(
    sc: &SurfaceConfig,
    tr: &Transform,
    planet: &crate::planet::Planet,
    init: &mut PeInitial,
) -> Result<Surface, String> {
    let ts: Vec<Real> = match &sc.initial {
        None => tj16_sst_grid(tr),
        Some(toml::Value::String(s)) if s == "tj16" => tj16_sst_grid(tr),
        Some(toml::Value::String(s)) if s == "ape_control" => zonal_grid(tr, ape_control_sst),
        Some(v) => {
            let t = v
                .as_float()
                .or_else(|| v.as_integer().map(|i| i as f64))
                .ok_or("surface.initial must be \"tj16\", \"ape_control\" or a temperature")?;
            vec![t as Real; tr.ngrid()]
        }
    };
    let mut sfc = Surface::slab(sc, ts);
    if let Some(path) = &sc.heightmap {
        let (h, nlat_h, nlon_h) = crate::io::heightmap::read(std::path::Path::new(path))
            .map_err(|e| format!("heightmap {path}: {e}"))?;
        let geo =
            geography::from_heightmap(tr, planet.gravity, &h, nlat_h, nlon_h, sc.orography_filter);
        // hydrostatic initial surface pressure over the mountains (T ≈ 288 K)
        let ps_mean = init_ps_mean(&init.state.lnps);
        let lnps: Vec<Real> = geo
            .phis
            .iter()
            .map(|&ph| (ps_mean.ln() - ph as f64 / (planet.gas_constant * 288.0)) as Real)
            .collect();
        init.state.lnps = tr.analysis(&lnps);
        init.phis_grid = geo.phis;
        sfc.set_land(sc, &geo.land_fraction);
    }
    if sc.ocean_heat_transport != 0.0 {
        ocean_slab::set_qflux(&mut sfc, tr, sc.ocean_heat_transport);
    }
    Ok(sfc)
}

/// Build the primitive-equation model from a run configuration.
pub fn from_config(cfg: &Config) -> Result<Primitive, String> {
    let planet = cfg.planet.resolve().map_err(|e| e.to_string())?;
    let levels = match (&cfg.sigma_half, cfg.levels) {
        (Some(h), _) => SigmaLevels::from_half(h.clone()),
        (None, Some(n)) => SigmaLevels::equal(n),
        (None, None) => return Err("primitive model needs `levels` or `sigma_half`".into()),
    };
    let tr = Transform::new(cfg.truncation, planet.radius);
    let mut init = cases::build(&tr, &planet, &levels, &cfg.initial_condition)?;
    let moisture = match &cfg.moisture {
        Some(m) => Some(m.resolve().map_err(|e| e.to_string())?),
        None => None,
    };
    let sponge = cfg.dynamics.as_ref().and_then(|d| d.sponge.as_ref());
    let physics = match &cfg.physics {
        Some(pc) => crate::physics::build_world(pc, sponge, moisture.as_ref())?,
        None => {
            if sponge.is_some() {
                return Err("[dynamics.sponge] needs the typed world physics".into());
            }
            crate::physics::build(cfg.forcing.as_ref(), &planet, moisture.as_ref())?
        }
    };
    let coupled = crate::physics::needs_surface(&physics);
    match (coupled, cfg.surface.is_some()) {
        (true, false) => return Err("the world physics (K-profile boundary layer) needs a [surface] block".into()),
        (false, true) if !physics.is_empty() => {
            return Err("a [surface] needs the world physics (the idealized bundles prescribe their own surface)".into())
        }
        _ => {}
    }
    // surface albedo where none is configured: the gray scheme's `albedo` (0.3)
    let default_albedo = cfg
        .physics
        .as_ref()
        .and_then(|p| p.radiation.params.get("albedo"))
        .or_else(|| cfg.forcing.as_ref().and_then(|f| f.get("albedo")))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(0.3);
    if let Some(m) = &cfg.moisture {
        let q = cases::initial_humidity(&tr, &planet, &levels, &m.initial)?;
        init.state.tracers = vec![super::state::Spec3::from_levels(q)];
    }
    let transport = cfg
        .dynamics
        .as_ref()
        .and_then(|d| d.tracer_transport.as_deref())
        .unwrap_or("spectral");
    match transport {
        "spectral" => {}
        "semi_lagrangian" => {
            // grid-point tracers (SPEC.md §9.8): synthesize the initial spectral fields
            let st = &mut init.state;
            st.gq = st
                .tracers
                .iter()
                .map(|t| {
                    let lv: Vec<Vec<crate::Real>> =
                        (0..t.nlev()).map(|k| tr.synthesis(&t[k])).collect();
                    let mut g = super::state::Grid3::from_levels(lv);
                    g.as_mut_slice().iter_mut().for_each(|x| *x = x.max(0.0));
                    g
                })
                .collect();
            st.tracers.clear();
        }
        other => {
            return Err(format!(
                "[dynamics] tracer_transport = \"{other}\": expected \"spectral\" or \"semi_lagrangian\""
            )
            )
        }
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
        energy_fixer: cfg.dynamics.as_ref().is_some_and(|d| d.energy_fixer),
    };
    let orbit = cfg.orbit.as_ref().map(|o| o.resolve(planet.rotation_rate));
    let surface = match &cfg.surface {
        None => None,
        Some(sc) => {
            let mut s = build_surface(sc, &tr, &planet, &mut init)?;
            s.set_default_albedo(default_albedo);
            Some(s)
        }
    };
    let mut m = Primitive::new(tr, planet, settings, levels, init, physics);
    m.orbit = orbit;
    m.surface = surface;
    m.mass_target = m.mass_of(&m.curr.lnps);
    m.condensable = moisture;
    Ok(m)
}
