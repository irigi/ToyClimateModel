//! Offline column evaluation of the semi-gray scheme for the calibration tool
//! (`toyclim radiation`, `tools/calibrate_radiation.py`): the same code as in the model.
//!
//! Optional clouds (P4 question 13, owner decision of 2026-11-27, step 1; diagnostic only): a
//! column may carry `cloud`, the cloud memory the diagnostic clouds leave for the radiation,
//! `[fraction | liquid path | ice path]` per layer (3 L values, paths in kg m⁻²), evaluated with
//! the cloud optics of the `[clouds]` defaults (r_liquid 10 µm, r_ice 30 µm, k_lw_liquid
//! 0.090361 m² g⁻¹, the values every P4 case uses) and the model's cloud overlap, so that the
//! cloud radiative effect of a layer group can be computed from saved snapshots. `mu_min`
//! overrides the offline floor of the cos zenith (1e-3; the model's is 0.05).

use super::cloud_optics::CloudOpticsParams;
use super::semigray::SemiGray;
use super::solver::{CloudOptics, Fluxes};
use super::TwoStream;
use crate::physics::column::{ColumnCtx, ColumnState, SurfaceExchange};
use crate::planet::Planet;
use crate::surface::{Tile, TileKind};
use serde::{Deserialize, Serialize};

/// One column, clear sky unless `cloud` is given. Arrays from the top down.
#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    /// Half-level pressures \[Pa\] (L + 1, top first, last = surface pressure).
    pub p_half: Vec<f64>,
    /// Layer temperature \[K\] and specific humidity \[kg/kg\] (L).
    pub t: Vec<f64>,
    pub q: Vec<f64>,
    /// Surface temperature \[K\] and albedo \[visible, near-IR\].
    pub ts: f64,
    pub albedo: [f64; 2],
    /// Stellar flux on a horizontal surface at the top \[W m⁻²\], cos zenith, star T_eff \[K\].
    pub s0: f64,
    pub mu0: f64,
    pub t_eff: f64,
    pub co2_ppm: f64,
    /// Gravity \[m s⁻²\] (default Earth's).
    #[serde(default)]
    pub gravity: Option<f64>,
    /// Cloud memory `[fraction | liquid path | ice path]` (3 L values, top first, paths
    /// \[kg m⁻²\]); none = clear sky.
    #[serde(default)]
    pub cloud: Option<Vec<f64>>,
    /// Floor of the cos zenith (default 1e-3).
    #[serde(default)]
    pub mu_min: Option<f64>,
}

/// Fluxes of a column \[W m⁻²\]; half-level arrays from the top down.
#[derive(Debug, Clone, Serialize)]
pub struct Result {
    pub olr: f64,
    pub lw_down_sfc: f64,
    pub sw_up_top: f64,
    pub sw_down_sfc: f64,
    pub sw_net_sfc: f64,
    pub sw_atm: f64,
    pub lw_up: Vec<f64>,
    pub lw_down: Vec<f64>,
    pub sw_net: Vec<f64>,
}

/// Evaluate `cols` with the semi-gray knobs of `knobs` (a `[radiation]` table without
/// `scheme`); each column's own CO₂ replaces the composition's.
pub fn evaluate(knobs: &toml::Table, cols: &[Column]) -> std::result::Result<Vec<Result>, String> {
    let base = crate::physics::semigray(knobs, 400.0)?;
    cols.iter().map(|c| one(&base, c)).collect()
}

fn one(base: &SemiGray, c: &Column) -> std::result::Result<Result, String> {
    let l = c.t.len();
    if c.p_half.len() != l + 1 || c.q.len() != l {
        return Err(format!(
            "column shapes: {} half levels, {} t, {} q",
            c.p_half.len(),
            l,
            c.q.len()
        ));
    }
    let mut planet = Planet::earth();
    if let Some(g) = c.gravity {
        planet.gravity = g;
    }
    let ps = c.p_half[l];
    let sigma_half: Vec<f64> = c.p_half.iter().map(|p| p / ps).collect();
    let p: Vec<f64> = (0..l)
        .map(|k| 0.5 * (c.p_half[k] + c.p_half[k + 1]))
        .collect();
    let dp: Vec<f64> = (0..l).map(|k| c.p_half[k + 1] - c.p_half[k]).collect();
    let sigma: Vec<f64> = p.iter().map(|x| x / ps).collect();
    let ctx = ColumnCtx {
        lat: 0.0,
        lon: 0.0,
        index: 0,
        sigma: &sigma,
        sigma_half: &sigma_half,
        ps,
        p: &p,
        dp: &dp,
        p_half: &c.p_half,
        phis: 0.0,
        time: 0.0,
        dt: 1.0,
        planet: &planet,
        cond: None,
        insolation: c.s0,
        cos_zenith: c.mu0,
        omega: &[],
    };
    let mut st = ColumnState::new(l, 1);
    st.t.copy_from_slice(&c.t);
    st.q.copy_from_slice(&c.q);
    let mut sfc = SurfaceExchange::default();
    let mut tile = Tile::new(TileKind::Ocean, 1.0, c.ts, f64::INFINITY, c.albedo[0]);
    tile.albedo_bands = c.albedo;
    sfc.tiles.push(tile);
    let sg = SemiGray {
        co2_ppm: c.co2_ppm,
        ..base.clone()
    };
    let mut scheme = TwoStream::new(Box::new(sg), c.t_eff);
    scheme.mu_min = c.mu_min.unwrap_or(1e-3);
    let mut cl = CloudOptics::default();
    let mut cloudy = false;
    if let Some(m) = &c.cloud {
        if m.len() != 3 * l {
            return Err(format!("cloud memory: {} values for {} layers", m.len(), l));
        }
        scheme.clouds = Some(CloudOpticsParams::new(10.0, 30.0, 0.090361, c.t_eff));
        sfc.cloud = m.clone();
        cloudy = scheme.cloud_optics(&sfc, l, &mut cl);
    }
    let mut f = Fluxes::default();
    scheme.fluxes(&ctx, &st, &sfc, cloudy.then_some(&cl), &mut f);
    Ok(Result {
        olr: f.lw_up[0],
        lw_down_sfc: f.lw_down[l],
        sw_up_top: f.sw_up_top,
        sw_down_sfc: f.sw_down_sfc.iter().sum(),
        sw_net_sfc: f.sw_net[l],
        sw_atm: f.sw_net[0] - f.sw_net[l],
        lw_up: f.lw_up,
        lw_down: f.lw_down,
        sw_net: f.sw_net,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(cloud: Option<Vec<f64>>) -> Column {
        let l = 10;
        let p_half: Vec<f64> = (0..=l).map(|k| 1e5 * k as f64 / l as f64).collect();
        let t: Vec<f64> = (0..l).map(|k| 210.0 + 8.0 * k as f64).collect();
        let q: Vec<f64> = (0..l).map(|k| 1e-5 * 3f64.powi(k as i32 / 2)).collect();
        Column {
            p_half,
            t,
            q,
            ts: 295.0,
            albedo: [0.06, 0.06],
            s0: 400.0,
            mu0: 0.6,
            t_eff: 5772.0,
            co2_ppm: 400.0,
            gravity: None,
            cloud,
            mu_min: None,
        }
    }

    /// Clouds in the offline column: none or zero fraction is the clear sky exactly; a low
    /// liquid cloud reflects (more SW up) and a high ice cloud traps LW (less OLR); doubling
    /// the water path of a cloud of the same fraction reflects more.
    #[test]
    fn offline_clouds() {
        let k = toml::Table::new();
        let l = 10;
        let mem = |k: usize, f: f64, lwp: f64, iwp: f64| {
            let mut m = vec![0.0; 3 * l];
            m[k] = f;
            m[l + k] = lwp;
            m[2 * l + k] = iwp;
            m
        };
        let r = evaluate(
            &k,
            &[
                column(None),
                column(Some(vec![0.0; 3 * l])),
                column(Some(mem(8, 0.5, 0.02, 0.0))),
                column(Some(mem(8, 0.5, 0.04, 0.0))),
                column(Some(mem(2, 0.5, 0.0, 0.02))),
            ],
        )
        .unwrap();
        assert_eq!(r[0].olr, r[1].olr);
        assert_eq!(r[0].sw_up_top, r[1].sw_up_top);
        assert!(
            r[2].sw_up_top > r[0].sw_up_top + 10.0,
            "{} {}",
            r[2].sw_up_top,
            r[0].sw_up_top
        );
        assert!(r[3].sw_up_top > r[2].sw_up_top);
        assert!(r[4].olr < r[0].olr - 5.0, "{} {}", r[4].olr, r[0].olr);
        assert!(evaluate(&k, &[column(Some(vec![0.0; 5]))]).is_err());
    }
}
