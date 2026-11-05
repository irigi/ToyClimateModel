//! Gray radiation (M8, first step): two-stream gray longwave and a single shortwave band.
//!
//! Longwave (Frierson, Held & Zurita-Gotor 2006, J. Atmos. Sci. 63, 2548–2566):
//! optical depth τ(σ) = τ₀(φ) [f_l σ + (1 − f_l) σ⁴], τ₀ = τ_e + (τ_p − τ_e) sin²φ;
//! dU/dτ = U − B, dD/dτ = B − D, B = σ_SB T⁴, integrated exactly per layer
//! (isothermal layers), U(surface) = σ_SB T_s⁴, D(top) = 0.
//! Optionally τ grows with column water vapour (`tau_q` per kg/m² of vapour above).
//! Shortwave: absorption optical depth τ_sw(σ) = τ_sw0 σ² along the slant path τ/μ₀
//! (O'Gorman & Schneider 2008 use τ_sw0 = 0.22 with daily means), surface albedo α.
//! The reflected beam leaves without further absorption.
//! Diagnostics: `olr`, `asr` (absorbed shortwave, atmosphere + surface), `sw_sfc_down`,
//! `lw_sfc_down` \[W m⁻²\].

use super::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::Real;

/// Stefan–Boltzmann constant \[W m⁻² K⁻⁴\].
pub const STEFAN_BOLTZMANN: f64 = 5.670374e-8;

pub struct GrayRadiation {
    /// LW optical depth at the equator and the poles.
    pub tau_eq: f64,
    pub tau_pole: f64,
    /// Linear (well-mixed) fraction f_l.
    pub f_linear: f64,
    /// Additional LW optical depth per kg m⁻² of water vapour (0 = none).
    pub tau_q: f64,
    /// SW absorption optical depth at the surface.
    pub tau_sw: f64,
    /// Surface albedo.
    pub albedo: f64,
    /// Surface temperature used when the model has no prognostic surface \[K\].
    pub t_surface_default: f64,
    /// Diagnostic clouds (M10, experimental): None = off.
    pub clouds: Option<Clouds>,
}

/// Diagnostic relative-humidity clouds for the gray scheme (experimental, untuned):
/// fraction c = clamp((RH − RH_c)/(1 − RH_c), 0, 1)² per level; a cloudy layer reflects a
/// fraction `sw_reflect`·c of the downward shortwave to space and adds `lw_tau`·c to the
/// layer's longwave optical depth. Diagnostic `cloud_cover` (random overlap).
#[derive(Debug, Clone)]
pub struct Clouds {
    pub cond: super::moist::Condensable,
    pub rh_crit: f64,
    pub sw_reflect: f64,
    pub lw_tau: f64,
}

impl Clouds {
    fn fractions(&self, planet: &Planet, col: &ColumnIn) -> Vec<f64> {
        let l = col.t.len();
        if col.tracers.len() < l {
            return vec![0.0; l];
        }
        (0..l)
            .map(|k| {
                let p = col.sigma[k] as f64 * col.ps as f64;
                let rh = col.tracers[k] as f64 / self.cond.qsat(planet, col.t[k] as f64, p);
                ((rh - self.rh_crit) / (1.0 - self.rh_crit))
                    .clamp(0.0, 1.0)
                    .powi(2)
            })
            .collect()
    }
}

impl Default for GrayRadiation {
    fn default() -> Self {
        GrayRadiation {
            tau_eq: 6.0,
            tau_pole: 1.5,
            f_linear: 0.1,
            tau_q: 0.0,
            tau_sw: 0.22,
            albedo: 0.3,
            t_surface_default: 288.0,
            clouds: None,
        }
    }
}

impl ColumnPhysics for GrayRadiation {
    fn name(&self) -> &'static str {
        "gray_radiation"
    }
    fn diagnostic_names(&self) -> Vec<&'static str> {
        let mut v = vec!["olr", "asr", "sw_sfc_down", "lw_sfc_down"];
        if self.clouds.is_some() {
            v.push("cloud_cover");
        }
        v
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let l = col.t.len();
        let ps = col.ps as f64;
        let g = planet.gravity;
        let lat = col.lat as f64;
        let tau0 = self.tau_eq + (self.tau_pole - self.tau_eq) * lat.sin().powi(2);
        // LW optical depth at half levels (0 at the top)
        let cf = match &self.clouds {
            Some(c) => c.fractions(planet, col),
            None => vec![0.0; l],
        };
        let lw_cloud = self.clouds.as_ref().map_or(0.0, |c| c.lw_tau);
        let sw_reflect = self.clouds.as_ref().map_or(0.0, |c| c.sw_reflect);
        let mut tau = vec![0.0; l + 1];
        let mut vapour_above = 0.0;
        let mut cloud_tau_above = 0.0;
        for k in 0..=l {
            let s = col.sigma_half[k] as f64;
            if k > 0 && self.tau_q > 0.0 && col.tracers.len() >= l {
                vapour_above += col.tracers[k - 1] as f64
                    * (col.sigma_half[k] - col.sigma_half[k - 1]) as f64
                    * ps
                    / g;
            }
            if k > 0 {
                cloud_tau_above += lw_cloud * cf[k - 1];
            }
            tau[k] = tau0 * (self.f_linear * s + (1.0 - self.f_linear) * s.powi(4))
                + self.tau_q * vapour_above
                + cloud_tau_above;
        }
        let ts = col
            .t_surface
            .map(|t| t as f64)
            .unwrap_or(self.t_surface_default);
        let b: Vec<f64> = col
            .t
            .iter()
            .map(|&t| STEFAN_BOLTZMANN * (t as f64).powi(4))
            .collect();
        // downward LW
        let mut down = vec![0.0; l + 1];
        for k in 0..l {
            let tr = (-(tau[k + 1] - tau[k])).exp();
            down[k + 1] = down[k] * tr + b[k] * (1.0 - tr);
        }
        // upward LW
        let mut up = vec![0.0; l + 1];
        up[l] = STEFAN_BOLTZMANN * ts.powi(4);
        for k in (0..l).rev() {
            let tr = (-(tau[k + 1] - tau[k])).exp();
            up[k] = up[k + 1] * tr + b[k] * (1.0 - tr);
        }
        // shortwave: downward beam along the slant path
        let s0 = col.insolation as f64;
        let mu = (col.cos_zenith as f64).max(0.05);
        // downward beam: absorption (heats the layer) then cloud reflection (leaves to space)
        let mut sw = vec![0.0; l + 1];
        let mut sw_abs = vec![0.0; l];
        let mut reflected = 0.0;
        sw[0] = s0;
        for k in 0..l {
            let (s1, s2) = (col.sigma_half[k] as f64, col.sigma_half[k + 1] as f64);
            let after = sw[k] * (-(self.tau_sw * (s2 * s2 - s1 * s1)) / mu).exp();
            sw_abs[k] = sw[k] - after;
            let r = sw_reflect * cf[k] * after;
            reflected += r;
            sw[k + 1] = after - r;
        }
        let sw_sfc = sw[l];
        let albedo = col.surface_albedo.map(|a| a as f64).unwrap_or(self.albedo);
        let sw_sfc_abs = sw_sfc * (1.0 - albedo);
        // heating: net downward flux F = SW_down − (U − D); layer gains F(top) − F(bottom)
        for k in 0..l {
            let lw_net = (up[k + 1] - down[k + 1]) - (up[k] - down[k]);
            let dp = (col.sigma_half[k + 1] - col.sigma_half[k]) as f64 * ps;
            out.dt[k] += (g / (planet.cp * dp) * (sw_abs[k] + lw_net)) as Real;
        }
        *out.surface_heat += (sw_sfc_abs + down[l] - up[l]) as Real;
        out.diag[0] += up[0] as Real;
        out.diag[1] += (s0 - albedo * sw_sfc - reflected) as Real;
        if self.clouds.is_some() {
            out.diag[4] += (1.0 - cf.iter().map(|c| 1.0 - c).product::<f64>()) as Real;
        }
        out.diag[2] += sw_sfc as Real;
        out.diag[3] += down[l] as Real;
    }
}

/// Rayleigh friction in the boundary layer (the momentum part of Held–Suarez):
/// k_v = k_f max(0, (σ − σ_b)/(1 − σ_b)).
pub struct RayleighFriction {
    pub kf: f64,
    pub sigma_b: f64,
}

impl ColumnPhysics for RayleighFriction {
    fn name(&self) -> &'static str {
        "rayleigh_friction"
    }
    fn tendencies(&self, _planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        for k in 0..col.t.len() {
            let kv =
                self.kf * ((col.sigma[k] as f64 - self.sigma_b) / (1.0 - self.sigma_b)).max(0.0);
            out.du[k] -= (kv * col.u[k] as f64) as Real;
            out.dv[k] -= (kv * col.v[k] as f64) as Real;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(t: &[Real], ts: Real, insolation: Real) -> (Vec<Real>, Real, Vec<Real>) {
        let l = t.len();
        let sigma: Vec<Real> = (0..l).map(|k| (k as Real + 0.5) / l as Real).collect();
        let half: Vec<Real> = (0..=l).map(|k| k as Real / l as Real).collect();
        let z = vec![0.0 as Real; l];
        let col = ColumnIn {
            lat: 0.0,
            lon: 0.0,
            index: 0,
            sigma: &sigma,
            sigma_half: &half,
            ps: 1e5,
            u: &z,
            v: &z,
            t,
            tracers: &[],
            phis: 0.0,
            t_surface: Some(ts),
            insolation,
            cos_zenith: 1.0,
            surface_albedo: None,
            wetness: 1.0,
            time: 0.0,
            dt: 1800.0,
        };
        let (mut du, mut dv, mut dt, mut diag) =
            (z.clone(), z.clone(), z.clone(), vec![0.0 as Real; 4]);
        let mut sh = 0.0 as Real;
        let mut out = ColumnOut {
            du: &mut du,
            dv: &mut dv,
            dt: &mut dt,
            dtracers: &mut [],
            diag: &mut diag,
            surface_heat: &mut sh,
        };
        GrayRadiation::default().tendencies(&Planet::earth(), &col, &mut out);
        (dt, sh, diag)
    }

    /// Energy conservation: TOA net = atmospheric heating + surface heat gain.
    #[test]
    fn column_energy_budget_closes() {
        let t: Vec<Real> = (0..20).map(|k| 200.0 + 4.0 * k as Real).collect();
        let (dt, sh, diag) = run(&t, 290.0, 400.0);
        let p = Planet::earth();
        let atm: f64 = dt
            .iter()
            .map(|&x| x as f64 * p.cp * 1e5 / 20.0 / p.gravity)
            .sum();
        let toa_net = diag[1] as f64 - diag[0] as f64; // ASR − OLR
        assert!(
            (toa_net - atm - sh as f64).abs() < 1e-6,
            "{toa_net} vs {atm} + {sh}"
        );
    }

    /// An isothermal column at the surface temperature: every upward flux equals σT⁴, so
    /// OLR = σT⁴; the atmosphere cools (it radiates to space) and the surface loses heat
    /// (downward LW < σT⁴), with the energy budget closing.
    #[test]
    fn isothermal_column_cools_to_space() {
        let t = vec![250.0 as Real; 30];
        let (dt, sh, diag) = run(&t, 250.0, 0.0);
        let b = STEFAN_BOLTZMANN * 250f64.powi(4);
        assert!((diag[0] as f64 - b).abs() < 1e-9);
        assert!(dt.iter().all(|&x| x <= 0.0));
        assert!(sh < 0.0);
        let p = Planet::earth();
        let atm: f64 = dt
            .iter()
            .map(|&x| x as f64 * p.cp * 1e5 / 30.0 / p.gravity)
            .sum();
        assert!((-(diag[0] as f64) - atm - sh as f64).abs() < 1e-6);
    }
}
