//! Moist physics (M7): a generic condensable, large-scale condensation, bulk surface fluxes
//! over a prescribed sea-surface temperature, and boundary-layer vertical diffusion.
//!
//! The formulation follows the moist idealized test of Thatcher & Jablonowski (2016,
//! Geosci. Model Dev. 9, 1263–1292; "TJ16"), which builds on Reed & Jablonowski (2012).
//! Tracer 0 is the specific humidity q \[kg/kg\] of the condensable. Virtual-temperature
//! effects on the dynamics are neglected (dry dynamical core with a moisture tracer).

use super::{ColumnIn, ColumnOut, ColumnPhysics};
use crate::planet::Planet;
use crate::Real;
use serde::{Deserialize, Serialize};

/// Thermodynamic properties of the condensing species. Data, not code: water is the
/// tested preset; other species (e.g. methane) only need different numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condensable {
    pub name: String,
    /// Latent heat of vaporization L \[J/kg\].
    pub latent_heat: f64,
    /// Gas constant of the vapour R_v \[J kg⁻¹ K⁻¹\].
    pub r_vapor: f64,
    /// Saturation vapour pressure e₀ at T₀ \[Pa\].
    pub e0: f64,
    /// Reference temperature T₀ \[K\].
    pub t0: f64,
    /// Density of the liquid \[kg/m³\] (precipitation depth).
    pub rho_liquid: f64,
}

impl Condensable {
    /// Water with the TJ16 constants.
    pub fn water() -> Self {
        Condensable {
            name: "water".into(),
            latent_heat: 2.5e6,
            r_vapor: 461.5,
            e0: 610.78,
            t0: 273.16,
            rho_liquid: 1000.0,
        }
    }

    pub fn preset(name: &str) -> Option<Self> {
        match name {
            "water" => Some(Self::water()),
            _ => None,
        }
    }

    /// ε = R_d / R_v.
    pub fn epsilon(&self, planet: &Planet) -> f64 {
        planet.gas_constant / self.r_vapor
    }

    /// Saturation specific humidity q_sat(T, p) = (ε e₀ / p) exp(−L/R_v (1/T − 1/T₀))
    /// (TJ16 eq. 1).
    pub fn qsat(&self, planet: &Planet, t: f64, p: f64) -> f64 {
        self.epsilon(planet) * self.e0 / p
            * (-(self.latent_heat / self.r_vapor) * (1.0 / t - 1.0 / self.t0)).exp()
    }
}

/// Large-scale condensation: supersaturation is removed in one physics step, latent heat
/// warms the air, the condensate falls out immediately (no clouds, no re-evaporation).
/// Diagnostic `precip_ls`: surface precipitation rate \[kg m⁻² s⁻¹ = mm/s\].
pub struct LargeScaleCondensation {
    pub cond: Condensable,
}

impl ColumnPhysics for LargeScaleCondensation {
    fn name(&self) -> &'static str {
        "large_scale_condensation"
    }
    fn diagnostic_names(&self) -> Vec<&'static str> {
        vec!["precip_ls"]
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let l = col.t.len();
        let (lv, rv, cp) = (self.cond.latent_heat, self.cond.r_vapor, planet.cp);
        let ps = col.ps as f64;
        let mut precip = 0.0;
        for k in 0..l {
            let t = col.t[k] as f64;
            let q = col.tracers[k] as f64;
            let p = col.sigma[k] as f64 * ps;
            let qs = self.cond.qsat(planet, t, p);
            if q > qs {
                // TJ16 eq. (2): C = (q − q_s) / (Δt (1 + L² q_s / (c_p R_v T²)))
                let c = (q - qs) / (col.dt * (1.0 + lv * lv * qs / (cp * rv * t * t)));
                out.dt[k] += (lv / cp * c) as Real;
                out.dtracers[k] -= c as Real;
                let dp = (col.sigma_half[k + 1] - col.sigma_half[k]) as f64 * ps;
                precip += c * dp / planet.gravity;
            }
        }
        out.diag[0] += precip as Real;
    }
}

/// Prescribed surface temperature.
#[derive(Debug, Clone)]
pub enum SurfaceTemperature {
    /// TJ16 eq. (6): T_s = ΔT exp(−φ²/(2 Δφ²)) + T_min.
    Gaussian {
        delta_t: f64,
        t_min: f64,
        width: f64,
    },
    Uniform(f64),
}

impl SurfaceTemperature {
    pub fn tj16() -> Self {
        SurfaceTemperature::Gaussian {
            delta_t: 29.0,
            t_min: 271.0,
            width: 26f64.to_radians(),
        }
    }
    pub fn at(&self, lat: f64, _lon: f64) -> f64 {
        match self {
            SurfaceTemperature::Gaussian {
                delta_t,
                t_min,
                width,
            } => delta_t * (-(lat * lat) / (2.0 * width * width)).exp() + t_min,
            SurfaceTemperature::Uniform(t) => *t,
        }
    }
}

/// Height of the lowest full level above the surface, TJ16 eq. (12) \[m\].
fn lowest_level_height(planet: &Planet, col: &ColumnIn, tv: f64) -> f64 {
    let l = col.t.len();
    let ln_ratio = -(col.sigma_half[l - 1] as f64).ln(); // ln p_s − ln p_{L−1/2}
    planet.gas_constant * tv / planet.gravity * ln_ratio / 2.0
}

/// Bulk sensible and latent heat fluxes at the lowest level over a wet surface
/// (TJ16 eqs. 7–8 with the time-implicit update). Diagnostics: `evap` \[kg m⁻² s⁻¹\],
/// `shf` (sensible heat flux, W m⁻²), `lhf` (latent heat flux, W m⁻²).
pub struct SurfaceFluxes {
    pub cond: Condensable,
    pub sst: SurfaceTemperature,
    /// Bulk transfer coefficients C_H, C_E.
    pub c_h: f64,
    pub c_e: f64,
}

impl ColumnPhysics for SurfaceFluxes {
    fn name(&self) -> &'static str {
        "surface_fluxes"
    }
    fn diagnostic_names(&self) -> Vec<&'static str> {
        vec!["evap", "shf", "lhf"]
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let k = col.t.len() - 1;
        let (ta, qa) = (col.t[k] as f64, col.tracers[k] as f64);
        let wind = ((col.u[k] * col.u[k] + col.v[k] * col.v[k]) as f64).sqrt();
        let tv = ta * (1.0 + 0.608 * qa);
        let za = lowest_level_height(planet, col, tv);
        let ps = col.ps as f64;
        let ts = col
            .t_surface
            .map(|t| t as f64)
            .unwrap_or_else(|| self.sst.at(col.lat as f64, col.lon as f64));
        let qs = self.cond.qsat(planet, ts, ps);
        let dt = col.dt;
        let (ah, ae) = (self.c_h * wind * dt / za, self.c_e * wind * dt / za);
        let t_new = (ta + ah * ts) / (1.0 + ah);
        let q_new = (qa + ae * qs) / (1.0 + ae);
        out.dt[k] += ((t_new - ta) / dt) as Real;
        out.dtracers[k] += ((q_new - qa) / dt) as Real;
        // Diagnose the fluxes actually applied to the lowest layer (mass Δp/g). TJ16 eq. (10)
        // writes E = ρ_a L C_E |v_a| (q_sat,s − q_a), which is smaller by ρ_a z_a g / Δp ≈ 1/2
        // than the moisture their eq. (8) adds, because z_a is the height of the lowest *full*
        // level (about half the layer depth). We keep eq. (8) and report the applied flux,
        // so that precipitation balances evaporation.
        let layer_mass = (col.sigma_half[k + 1] - col.sigma_half[k]) as f64 * ps / planet.gravity;
        let evap = layer_mass * (q_new - qa) / dt;
        out.diag[0] += evap as Real;
        out.diag[1] += (layer_mass * planet.cp * (t_new - ta) / dt) as Real;
        out.diag[2] += (evap * self.cond.latent_heat) as Real;
        let shf = layer_mass * planet.cp * (t_new - ta) / dt;
        *out.surface_heat -= (shf + evap * self.cond.latent_heat) as Real;
    }
}

/// Boundary-layer vertical diffusion of potential temperature and q (TJ16 eqs. 15–17):
/// K = C_E |v_a| z_a below p_pbl, decaying as exp(−((p_pbl − p)/p_strato)²) above.
/// Backward-Euler (implicit) tridiagonal solve; zero flux at the top and the surface
/// (surface fluxes are a separate scheme).
pub struct BoundaryLayer {
    pub c_e: f64,
    /// Top of the boundary layer \[Pa\].
    pub p_pbl: f64,
    /// Decay scale above the boundary layer \[Pa\].
    pub p_strato: f64,
    /// Reference pressure for θ \[Pa\].
    pub p0: f64,
}

/// Solve the implicit diffusion (x_new − x)/dt = g/Δp_k [A_{k+1/2}(x_{k+1}−x_k) − A_{k−1/2}(x_k−x_{k−1})].
fn implicit_diffusion(x: &[f64], a_half: &[f64], dp: &[f64], g: f64, dt: f64) -> Vec<f64> {
    let l = x.len();
    // tridiagonal: lower[k] x_{k−1} + diag[k] x_k + upper[k] x_{k+1} = x_k
    let mut lower = vec![0.0; l];
    let mut diag = vec![1.0; l];
    let mut upper = vec![0.0; l];
    for k in 0..l {
        let c = g * dt / dp[k];
        if k + 1 < l {
            upper[k] = -c * a_half[k];
            diag[k] += c * a_half[k];
        }
        if k > 0 {
            lower[k] = -c * a_half[k - 1];
            diag[k] += c * a_half[k - 1];
        }
    }
    // Thomas algorithm
    let mut cp = vec![0.0; l];
    let mut dpv = vec![0.0; l];
    cp[0] = upper[0] / diag[0];
    dpv[0] = x[0] / diag[0];
    for k in 1..l {
        let m = diag[k] - lower[k] * cp[k - 1];
        cp[k] = upper[k] / m;
        dpv[k] = (x[k] - lower[k] * dpv[k - 1]) / m;
    }
    let mut y = vec![0.0; l];
    y[l - 1] = dpv[l - 1];
    for k in (0..l - 1).rev() {
        y[k] = dpv[k] - cp[k] * y[k + 1];
    }
    y
}

impl ColumnPhysics for BoundaryLayer {
    fn name(&self) -> &'static str {
        "boundary_layer"
    }
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut) {
        let l = col.t.len();
        if l < 2 {
            return;
        }
        let ps = col.ps as f64;
        let g = planet.gravity;
        let r = planet.gas_constant;
        let kappa = planet.kappa();
        let kb = l - 1;
        let wind = ((col.u[kb] * col.u[kb] + col.v[kb] * col.v[kb]) as f64).sqrt();
        let qa = col.tracers.get(kb).copied().unwrap_or(0.0) as f64;
        let za = lowest_level_height(planet, col, col.t[kb] as f64 * (1.0 + 0.608 * qa));
        let k0 = self.c_e * wind * za;
        if k0 <= 0.0 {
            return;
        }
        let p: Vec<f64> = col.sigma.iter().map(|&s| s as f64 * ps).collect();
        let dp: Vec<f64> = (0..l)
            .map(|k| (col.sigma_half[k + 1] - col.sigma_half[k]) as f64 * ps)
            .collect();
        // A_{k+1/2} = ρ² g K / (p_{k+1} − p_k) at the interface between k and k+1
        let a_half: Vec<f64> = (0..l - 1)
            .map(|k| {
                let ph = col.sigma_half[k + 1] as f64 * ps;
                let kk = if ph > self.p_pbl {
                    k0
                } else {
                    k0 * (-((self.p_pbl - ph) / self.p_strato).powi(2)).exp()
                };
                let th = 0.5 * (col.t[k] + col.t[k + 1]) as f64;
                let rho = ph / (r * th);
                rho * rho * g * kk / (p[k + 1] - p[k])
            })
            .collect();
        let dt = col.dt;
        let pi: Vec<f64> = p.iter().map(|&pk| (pk / self.p0).powf(kappa)).collect();
        let theta: Vec<f64> = (0..l).map(|k| col.t[k] as f64 / pi[k]).collect();
        let th_new = implicit_diffusion(&theta, &a_half, &dp, g, dt);
        for k in 0..l {
            out.dt[k] += ((th_new[k] - theta[k]) * pi[k] / dt) as Real;
        }
        if col.tracers.len() >= l {
            let q: Vec<f64> = col.tracers[..l].iter().map(|&x| x as f64).collect();
            let q_new = implicit_diffusion(&q, &a_half, &dp, g, dt);
            for k in 0..l {
                out.dtracers[k] += ((q_new[k] - q[k]) / dt) as Real;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qsat_water_values() {
        let w = Condensable::water();
        let e = Planet::earth();
        // at T0 and 1000 hPa: q_s ≈ ε e0 / p ≈ 0.622 · 6.1078 / 1000 ≈ 3.8 g/kg
        let q0 = w.qsat(&e, 273.16, 1e5);
        assert!((q0 - 0.6219 * 610.78 / 1e5).abs() < 1e-5, "{q0}");
        // ~ 20 g/kg at 300 K
        let q300 = w.qsat(&e, 300.0, 1e5);
        assert!(q300 > 0.018 && q300 < 0.026, "{q300}");
    }

    #[test]
    fn implicit_diffusion_conserves_column_integral() {
        let x = [1.0, 2.0, 5.0, 3.0];
        let a = [3e-3, 1e-2, 4e-2];
        let dp = [2e4, 3e4, 3e4, 2e4];
        let y = implicit_diffusion(&x, &a, &dp, 9.81, 3600.0);
        let s0: f64 = x.iter().zip(&dp).map(|(a, b)| a * b).sum();
        let s1: f64 = y.iter().zip(&dp).map(|(a, b)| a * b).sum();
        assert!((s0 - s1).abs() < 1e-9 * s0);
        // smoother
        let var = |v: &[f64]| v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>();
        assert!(var(&y) < var(&x));
    }
}

#[cfg(test)]
mod scheme_tests {
    use super::*;

    fn column(t: &[Real], q: &[Real]) -> (Vec<Real>, Vec<Real>) {
        let l = t.len();
        let sigma: Vec<Real> = (0..l).map(|k| (k as Real + 0.5) / l as Real).collect();
        let half: Vec<Real> = (0..=l).map(|k| k as Real / l as Real).collect();
        let planet = Planet::earth();
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
            tracers: q,
            phis: 0.0,
            t_surface: None,
            insolation: 0.0,
            cos_zenith: 0.0,
            time: 0.0,
            dt: 1800.0,
        };
        let mut sh = 0.0 as Real;
        let (mut du, mut dv, mut dt, mut dq, mut diag) = (
            z.clone(),
            z.clone(),
            z.clone(),
            z.clone(),
            vec![0.0 as Real; 1],
        );
        let mut out = ColumnOut {
            du: &mut du,
            dv: &mut dv,
            dt: &mut dt,
            dtracers: &mut dq,
            diag: &mut diag,
            surface_heat: &mut sh,
        };
        LargeScaleCondensation {
            cond: Condensable::water(),
        }
        .tendencies(&planet, &col, &mut out);
        (dt, dq)
    }

    /// Condensation conserves moist enthalpy c_p T + L q per level and only removes
    /// supersaturation.
    #[test]
    fn condensation_conserves_moist_enthalpy() {
        let t = [220.0, 260.0, 290.0, 300.0];
        let q = [1e-5, 1e-3, 0.03, 0.001];
        let (dt, dq) = column(&t, &q);
        let w = Condensable::water();
        for k in 0..4 {
            let h = 1004.64 * dt[k] as f64 + w.latent_heat * dq[k] as f64;
            assert!(h.abs() < 1e-9, "level {k}: {h}");
        }
        assert!(dq[2] < 0.0, "supersaturated level must condense");
        assert_eq!(dq[3], 0.0, "subsaturated level untouched");
    }
}
