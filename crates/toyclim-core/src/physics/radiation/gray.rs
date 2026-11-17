//! Gray radiation (prototype world radiation, kept as `scheme = "gray"` for idealized
//! comparators; SPEC.md §9.1, replaced as the world default in P3).
//!
//! Longwave (Frierson, Held & Zurita-Gotor 2006, J. Atmos. Sci. 63, 2548–2566, eqs. 4–7):
//! optical depth τ(σ) = τ₀(φ) [f_l σ + (1 − f_l) σ⁴], τ₀ = τ_e + (τ_p − τ_e) sin²φ with
//! `latitude_dependence` (else τ₀ = τ_e);
//! dU/dτ = U − B, dD/dτ = B − D, B = σ_SB T⁴, integrated exactly per layer
//! (isothermal layers), U(surface) = σ_SB T_s⁴ (tile mean), D(top) = 0.
//! Optionally τ grows with column water vapour (`tau_q` per kg/m² of vapour above).
//! Shortwave: a fraction `reflectance` of the incoming beam is reflected at the top without
//! absorption (a stand-in for clouds and aerosols until the P4 clouds; 0 by default); the
//! rest is absorbed with optical depth τ_sw(σ) = τ_sw0 σ² along the slant path τ/μ₀
//! (O'Gorman & Schneider 2008 use τ_sw0 = 0.22 with daily means), surface albedo from
//! the surface tiles (else `albedo`). The reflected beam leaves without further
//! absorption.
//!
//! Hands the downward fluxes at the surface to the surface solve (SPEC.md §9.3.3 step 1).
//! Diagnostics: `rsdt` (incoming at the top), `olr`, `asr` (absorbed stellar, by the
//! atmosphere and the surface), `rsds`, `rlds` (downward shortwave and longwave at the
//! surface) \[W m⁻²\]; `clt` (total cloud cover) with the experimental clouds.

use super::STEFAN_BOLTZMANN;
use crate::physics::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use crate::physics::condensation::Condensable;

pub struct GrayRadiation {
    /// LW optical depth at the equator and the poles; `tau_pole` applies only with
    /// `latitude_dependence` (an Earth-shaped prescription, on only in Earth-like idealized
    /// cases, SPEC.md §9.6 P3); otherwise τ₀ = `tau_eq` everywhere.
    pub tau_eq: f64,
    pub tau_pole: f64,
    pub latitude_dependence: bool,
    /// Linear (well-mixed) fraction f_l.
    pub f_linear: f64,
    /// Additional LW optical depth per kg m⁻² of water vapour (0 = none).
    pub tau_q: f64,
    /// SW absorption optical depth at the surface.
    pub tau_sw: f64,
    /// Fraction of the incoming shortwave reflected at the top (cloud stand-in).
    pub reflectance: f64,
    /// Surface albedo when the model has no surface tiles.
    pub albedo: f64,
    /// Surface temperature used when the model has no surface tiles \[K\].
    pub t_surface_default: f64,
    /// Diagnostic clouds (experimental): None = off.
    pub clouds: Option<Clouds>,
}

/// Diagnostic relative-humidity clouds for the gray scheme (experimental, untuned):
/// fraction c = clamp((RH − RH_c)/(1 − RH_c), 0, 1)² per level; a cloudy layer reflects a
/// fraction `sw_reflect`·c of the downward shortwave to space and adds `lw_tau`·c to the
/// layer's longwave optical depth. Diagnostic `clt` (random overlap).
#[derive(Debug, Clone)]
pub struct Clouds {
    pub cond: Condensable,
    pub rh_crit: f64,
    pub sw_reflect: f64,
    pub lw_tau: f64,
}

impl Clouds {
    fn fraction(&self, ctx: &ColumnCtx, st: &ColumnState, k: usize) -> f64 {
        let Some(q) = st.humidity() else {
            return 0.0;
        };
        let rh = q[k] / self.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        ((rh - self.rh_crit) / (1.0 - self.rh_crit))
            .clamp(0.0, 1.0)
            .powi(2)
    }
}

impl Default for GrayRadiation {
    fn default() -> Self {
        GrayRadiation {
            tau_eq: 6.0,
            tau_pole: 1.5,
            latitude_dependence: false,
            f_linear: 0.1,
            tau_q: 0.0,
            tau_sw: 0.22,
            reflectance: 0.0,
            albedo: 0.3,
            t_surface_default: 288.0,
            clouds: None,
        }
    }
}

impl ColumnScheme for GrayRadiation {
    fn name(&self) -> &'static str {
        "gray_radiation"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        if self.clouds.is_some() {
            &["rsdt", "olr", "asr", "rsds", "rlds", "clt"]
        } else {
            &["rsdt", "olr", "asr", "rsds", "rlds"]
        }
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        work: &mut Workspace,
    ) {
        let l = st.nlev;
        let g = ctx.planet.gravity;
        let half = ctx.sigma_half;
        let tau0 = if self.latitude_dependence {
            self.tau_eq + (self.tau_pole - self.tau_eq) * ctx.lat.sin().powi(2)
        } else {
            self.tau_eq
        };
        let [tau, cf, b, down, up, sw, sw_abs, ..] = &mut work.buf;
        for k in 0..l {
            cf[k] = self.clouds.as_ref().map_or(0.0, |c| c.fraction(ctx, st, k));
        }
        let lw_cloud = self.clouds.as_ref().map_or(0.0, |c| c.lw_tau);
        let sw_reflect = self.clouds.as_ref().map_or(0.0, |c| c.sw_reflect);
        // LW optical depth at half levels (0 at the top)
        let mut vapour_above = 0.0;
        let mut cloud_tau_above = 0.0;
        for k in 0..=l {
            let s = half[k];
            if k > 0 {
                if self.tau_q > 0.0 {
                    if let Some(q) = st.humidity() {
                        vapour_above += q[k - 1] * ctx.dp[k - 1] / g;
                    }
                }
                cloud_tau_above += lw_cloud * cf[k - 1];
            }
            tau[k] = tau0 * (self.f_linear * s + (1.0 - self.f_linear) * s.powi(4))
                + self.tau_q * vapour_above
                + cloud_tau_above;
        }
        let (sfc_emission, albedo) = if sfc.tiles.is_empty() {
            (
                STEFAN_BOLTZMANN * self.t_surface_default.powi(4),
                self.albedo,
            )
        } else {
            (sfc.lw_emission(), sfc.albedo())
        };
        for k in 0..l {
            b[k] = STEFAN_BOLTZMANN * st.t[k].powi(4);
        }
        down[0] = 0.0;
        for k in 0..l {
            let tr = (-(tau[k + 1] - tau[k])).exp();
            down[k + 1] = down[k] * tr + b[k] * (1.0 - tr);
        }
        up[l] = sfc_emission;
        for k in (0..l).rev() {
            let tr = (-(tau[k + 1] - tau[k])).exp();
            up[k] = up[k + 1] * tr + b[k] * (1.0 - tr);
        }
        // shortwave: downward beam along the slant path; absorption heats the layer, cloud
        // reflection leaves to space
        let s0 = ctx.insolation;
        let mu = ctx.cos_zenith.max(0.05);
        let mut reflected = self.reflectance * s0;
        sw[0] = s0 - reflected;
        for k in 0..l {
            let (s1, s2) = (half[k], half[k + 1]);
            let after = sw[k] * (-(self.tau_sw * (s2 * s2 - s1 * s1)) / mu).exp();
            sw_abs[k] = sw[k] - after;
            let r = sw_reflect * cf[k] * after;
            reflected += r;
            sw[k + 1] = after - r;
        }
        let sw_sfc = sw[l];
        // heating: the layer gains SW absorbed + net LW convergence
        for k in 0..l {
            let lw_net = (up[k + 1] - down[k + 1]) - (up[k] - down[k]);
            st.add_t(k, g / (ctx.planet.cp * ctx.dp[k]) * (sw_abs[k] + lw_net));
        }
        let asr = s0 - albedo * sw_sfc - reflected;
        sfc.sw_down = sw_sfc;
        sfc.lw_down = down[l];
        sfc.toa_net = asr - up[0];
        diag[0] += s0;
        diag[1] += up[0];
        diag[2] += asr;
        diag[3] += sw_sfc;
        diag[4] += down[l];
        if self.clouds.is_some() {
            diag[5] += 1.0 - cf[..l].iter().map(|c| 1.0 - c).product::<f64>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;
    use crate::planet::Planet;
    use crate::surface::{Tile, TileKind};

    fn run(t: &[f64], ts: f64, insolation: f64) -> (Vec<f64>, SurfaceExchange, Vec<f64>) {
        let l = t.len();
        let half: Vec<f64> = (0..=l).map(|k| k as f64 / l as f64).collect();
        let geo = Geometry::new(&half, 1e5);
        let planet = Planet::earth();
        let mut ctx = geo.ctx(&planet, None, 1800.0);
        ctx.insolation = insolation;
        ctx.cos_zenith = 1.0;
        let mut st = ColumnState::new(l, 0);
        st.begin(1800.0);
        st.t.copy_from_slice(t);
        let mut sfc = SurfaceExchange::default();
        sfc.tiles
            .push(Tile::new(TileKind::Ocean, 1.0, ts, f64::INFINITY, 0.3));
        let mut diag = vec![0.0; 5];
        GrayRadiation::default().apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(l));
        (st.dt.clone(), sfc, diag)
    }

    fn surface_absorbed(sfc: &SurfaceExchange) -> f64 {
        sfc.sw_down * (1.0 - sfc.albedo()) + sfc.lw_down - sfc.lw_emission()
    }

    /// Energy conservation: TOA net = atmospheric heating + surface absorption.
    #[test]
    fn column_energy_budget_closes() {
        let t: Vec<f64> = (0..20).map(|k| 200.0 + 4.0 * k as f64).collect();
        let (dt, sfc, diag) = run(&t, 290.0, 400.0);
        let p = Planet::earth();
        let atm: f64 = dt.iter().map(|&x| x * p.cp * 1e5 / 20.0 / p.gravity).sum();
        let toa_net = diag[2] - diag[1];
        assert!((toa_net - sfc.toa_net).abs() < 1e-12);
        let sh = surface_absorbed(&sfc);
        assert!(
            (toa_net - atm - sh).abs() < 1e-6,
            "{toa_net} vs {atm} + {sh}"
        );
    }

    /// An isothermal column at the surface temperature: OLR = σT⁴, the atmosphere cools
    /// and the surface loses heat, with the energy budget closing.
    #[test]
    fn isothermal_column_cools_to_space() {
        let t = vec![250.0; 30];
        let (dt, sfc, diag) = run(&t, 250.0, 0.0);
        let b = STEFAN_BOLTZMANN * 250f64.powi(4);
        assert!((diag[1] - b).abs() < 1e-9);
        assert!(dt.iter().all(|&x| x <= 0.0));
        let sh = surface_absorbed(&sfc);
        assert!(sh < 0.0);
        let p = Planet::earth();
        let atm: f64 = dt.iter().map(|&x| x * p.cp * 1e5 / 30.0 / p.gravity).sum();
        assert!((-diag[1] - atm - sh).abs() < 1e-6);
    }

    /// The top reflectance removes its share of the beam before any absorption: absorbed
    /// stellar flux drops by at least that share and the column budget still closes.
    #[test]
    fn reflectance_reduces_absorption_and_conserves() {
        let t: Vec<f64> = (0..20).map(|k| 200.0 + 4.0 * k as f64).collect();
        let l = t.len();
        let half: Vec<f64> = (0..=l).map(|k| k as f64 / l as f64).collect();
        let geo = Geometry::new(&half, 1e5);
        let planet = Planet::earth();
        let mut ctx = geo.ctx(&planet, None, 1800.0);
        ctx.insolation = 400.0;
        ctx.cos_zenith = 1.0;
        let run = |r: f64| {
            let mut st = ColumnState::new(l, 0);
            st.begin(1800.0);
            st.t.copy_from_slice(&t);
            let mut sfc = SurfaceExchange::default();
            sfc.tiles
                .push(Tile::new(TileKind::Ocean, 1.0, 290.0, f64::INFINITY, 0.06));
            let mut diag = vec![0.0; 5];
            let g = GrayRadiation {
                reflectance: r,
                ..Default::default()
            };
            g.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(l));
            let atm: f64 = st
                .dt
                .iter()
                .map(|&x| x * planet.cp * 1e5 / 20.0 / planet.gravity)
                .sum();
            let toa = diag[2] - diag[1];
            assert!((toa - atm - surface_absorbed(&sfc)).abs() < 1e-6);
            diag[2]
        };
        let (a0, a1) = (run(0.0), run(0.23));
        assert!(a1 <= (1.0 - 0.23) * a0 + 1e-9, "{a0} {a1}");
    }
}
