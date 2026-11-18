//! Offline column evaluation of the semi-gray scheme for the calibration tool
//! (`toyclim radiation`, `tools/calibrate_radiation.py`): the same code as in the model.

use super::semigray::SemiGray;
use super::solver::Fluxes;
use super::TwoStream;
use crate::physics::column::{ColumnCtx, ColumnState, SurfaceExchange};
use crate::planet::Planet;
use crate::surface::{Tile, TileKind};
use serde::{Deserialize, Serialize};

/// One clear-sky column. Arrays from the top down.
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
    scheme.mu_min = 1e-3;
    let mut f = Fluxes::default();
    scheme.fluxes(&ctx, &st, &sfc, None, &mut f);
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
