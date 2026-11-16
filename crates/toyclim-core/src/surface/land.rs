//! Land water and snow of one cell (SPEC.md §9.4.5), per unit land area.
//!
//! - Bucket (Manabe 1969): capacity W_max = 150 kg m⁻² **[knob]**, evaporation efficiency
//!   β = min(1, W / (0.75 W_max)), runoff (`mrro`) when full; runoff leaves the system.
//! - Snow water equivalent S: snowfall accumulates; the snow-covered fraction
//!   f_snow = S / (S + 15 kg m⁻²) **[knob]** evaporates with β = 1 and sublimates
//!   (L_v + L_f); the surface temperature is held at T_melt while snow remains and the
//!   surplus melts it (melt water into the bucket); S is capped at 5000 kg m⁻², the
//!   excess leaves as (frozen) runoff and its latent energy is reported (`ice_cap_flux`).
//! - The soil column takes the surface solve's heat flux, plus any melt energy left when
//!   the snow is gone.

use super::Surface;

/// Per land area: runoff \[kg m⁻² s⁻¹\], cap energy \[W m⁻²\].
#[derive(Debug, Clone, Copy, Default)]
pub struct LandResult {
    pub runoff: f64,
    pub ice_cap: f64,
}

/// Advance the land of point p over `dt` with the surface solve's heat flux into the soil
/// `ground`, evaporation `evap`, melt `melt` and the rain and snow falling on it.
#[allow(clippy::too_many_arguments)]
pub fn step(
    s: &mut Surface,
    p: usize,
    ground: f64,
    evap: f64,
    melt: f64,
    rain: f64,
    snow: f64,
    dt: f64,
) -> LandResult {
    let (fs, beta) = s.land_cover(p);
    let pr = &s.params;
    let lf = pr.lf;
    let beff = fs + (1.0 - fs) * beta;
    let share = if beff > 0.0 { fs / beff } else { 0.0 };
    let mut sn = s.snow[p] + snow * dt;
    let mut w = s.bucket[p] + rain * dt;
    let mut f_soil = ground;
    let e = evap * dt;
    if e >= 0.0 {
        let es = share * e;
        let take = es.min(sn);
        sn -= take;
        // sublimation charged with L_f but taken from the bucket: return L_f to the soil
        let deficit = es - take;
        f_soil += lf * deficit / dt;
        w -= e - es + deficit;
    } else {
        // dew and frost
        let ds = share * -e;
        sn += ds;
        w += -e - ds;
    }
    if melt > 0.0 && lf > 0.0 {
        let em = melt * dt;
        let ms = sn.min(em / lf);
        sn -= ms;
        w += ms;
        f_soil += (em - ms * lf) / dt;
    } else {
        f_soil += melt;
    }
    let mut t = s.soil_t[p];
    pr.soil.step(&mut t, dt, f_soil);
    let mut r = 0.0;
    if w > pr.bucket_capacity {
        r += w - pr.bucket_capacity;
        w = pr.bucket_capacity;
    }
    let mut cap = 0.0;
    if sn > pr.snow_cap {
        let ex = sn - pr.snow_cap;
        r += ex;
        cap += lf * ex / dt;
        sn = pr.snow_cap;
    }
    s.soil_t[p] = t;
    s.bucket[p] = w;
    s.snow[p] = sn;
    LandResult {
        runoff: r / dt,
        ice_cap: cap,
    }
}
