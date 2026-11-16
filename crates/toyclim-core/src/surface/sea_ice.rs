//! Slab ocean and zero-layer sea ice of one cell (SPEC.md §9.4.5), per unit ocean area.
//!
//! State: mixed-layer temperature T_ml (shared by open water and the water under the ice),
//! mean ice thickness h (concentration sic = min(1, h/h₀), floes h/sic ≥ h₀ thick), snow on
//! the ice S (per ocean area), ice surface temperature.
//!
//! Over a step Δt:
//! 1. open water: the mixed layer gains the surface solve's net flux and the q-flux; rain
//!    and snow falling into it join the ocean reservoir, snow melting with ocean heat;
//! 2. ice: snow accumulates; sublimation takes snow first, then ice; surface melt (the
//!    surplus above T_melt) melts snow first, then ice; the conductive flux at the base
//!    grows ice (ρ_i L_f dh/dt = −G) or melts it; rain on the ice runs into the ocean;
//! 3. the mixed layer freezes new ice when it would cool below T_f (the energy deficit
//!    grows ice, T_ml = T_f) and melts ice from below with any heat above T_f;
//! 4. caps: ice thickness ≤ 50 m, snow on ice ≤ 5000 kg m⁻²; frozen mass above a cap is
//!    discarded and its latent energy reported (`ice_cap_flux`).
//!
//! Melted snow, sublimated ice and dumped snow are exchanges with the ocean reservoir
//! (`uptake`). Ice is ocean water: freezing and melting change neither.

use super::Surface;

/// Results of the surface solve for the ice tile (per ice area).
#[derive(Debug, Clone, Copy)]
pub struct IceFluxes {
    /// T_s⁺ − T_s \[K\].
    pub dts: f64,
    /// Conductive flux into the ice (to the base) \[W m⁻²\].
    pub ground: f64,
    /// Sublimation \[kg m⁻² s⁻¹\] and surface melt \[W m⁻²\].
    pub evap: f64,
    pub melt: f64,
}

/// Fluxes of one ocean cell over a step.
#[derive(Debug, Clone, Copy)]
pub struct OceanFluxes {
    /// Open water: (net flux into the water \[W m⁻²\], evaporation \[kg m⁻² s⁻¹\]).
    pub open: Option<(f64, f64)>,
    pub ice: Option<IceFluxes>,
    /// Rain and snow \[kg m⁻² s⁻¹\], q-flux \[W m⁻²\].
    pub rain: f64,
    pub snow: f64,
    pub qflux: f64,
}

/// Per ocean area: water into the ocean reservoir \[kg m⁻² s⁻¹\], cap energy \[W m⁻²\].
#[derive(Debug, Clone, Copy, Default)]
pub struct OceanResult {
    pub uptake: f64,
    pub ice_cap: f64,
}

pub fn step(s: &mut Surface, p: usize, o: &OceanFluxes, frac: f64, dt: f64) -> OceanResult {
    let pr = &s.params;
    let (lf, rho, c, tf) = (pr.lf, pr.rho_ice, pr.ocean_c, pr.t_freeze_sea);
    let (h_max, snow_cap, h0) = (pr.ice_h_max, pr.snow_cap, pr.ice_h0);
    let a = s.sic(p);
    let w = 1.0 - a;
    if pr.fixed {
        let e = o.open.map_or(0.0, |x| x.1);
        return OceanResult {
            uptake: o.rain + o.snow - e,
            ice_cap: 0.0,
        };
    }
    let mut up = 0.0;
    let mut de = dt * o.qflux; // energy into the mixed layer [J m⁻²]
    if let Some((g, e)) = o.open {
        de += dt * w * (g - o.snow * lf);
        up += w * (o.rain + o.snow - e);
    }
    let (mut h, mut sn) = (s.ice_h[p], s.ice_snow[p]);
    if a > 0.0 {
        let fx = o.ice.expect("ice fluxes for an ice-covered cell");
        up += a * o.rain;
        sn += a * o.snow * dt;
        let subl = a * fx.evap * dt;
        if subl >= 0.0 {
            let take = subl.min(sn);
            sn -= take;
            let rest = subl - take;
            h -= rest / rho;
            up -= rest / dt;
        } else {
            sn -= subl; // frost
        }
        let mut em = a * fx.melt * dt;
        if lf > 0.0 {
            let ms = sn.min(em / lf);
            sn -= ms;
            up += ms / dt;
            em -= ms * lf;
            h -= em / (rho * lf);
            h -= a * fx.ground * dt / (rho * lf);
            if h < 0.0 {
                de += -h * rho * lf;
                h = 0.0;
            }
        }
        s.ice_ts[p] = (s.ice_ts[p] + frac * fx.dts).min(pr.t_melt);
    }
    let mut t = s.t_ml[p] + de / c;
    if lf > 0.0 && pr.sea_ice {
        if t < tf {
            if h <= 0.0 {
                s.ice_ts[p] = tf;
            }
            h += c * (tf - t) / (rho * lf);
            t = tf;
        } else if h > 0.0 {
            let avail = c * (t - tf);
            let need = rho * lf * h;
            if avail >= need {
                t -= need / c;
                h = 0.0;
            } else {
                h -= avail / (rho * lf);
                t = tf;
            }
        }
        if h <= 0.0 && sn > 0.0 {
            // snow without ice falls into the ocean and melts with ocean heat
            t -= lf * sn / c;
            up += sn / dt;
            sn = 0.0;
        }
    }
    let mut cap = 0.0;
    if h > h_max {
        cap += rho * lf * (h - h_max) / dt;
        h = h_max;
    }
    let sn_max = snow_cap * (h / h0).min(1.0);
    if sn > sn_max {
        let ex = sn - sn_max;
        cap += lf * ex / dt;
        up += ex / dt;
        sn = sn_max;
    }
    s.t_ml[p] = t;
    s.ice_h[p] = h;
    s.ice_snow[p] = sn;
    OceanResult {
        uptake: up,
        ice_cap: cap,
    }
}
