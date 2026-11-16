//! Surface model (SPEC.md §9.4.5–9.4.6): every cell has fractions of open ocean, sea ice and
//! land (land fraction from the heightmap, no threshold), each a tile with its own surface
//! temperature, albedo, roughness and wetness.
//!
//! - **Ocean**: slab mixed layer (shared by open water and the water under the ice) with an
//!   optional prescribed heat-flux convergence (q-flux), or a prescribed SST.
//! - **Sea ice**: zero-layer thermodynamics (Semtner 1976, as described in UM Documentation
//!   Paper 45 §3): no heat storage, linear temperature profile through ice and snow,
//!   equivalent depth h_E = h_i + (k_i/k_s) h_s, conductive flux k_i (T_f − T_s)/h_E with the
//!   base at the sea-water freezing point T_f; surface melt when T_s reaches T_melt;
//!   growth or bottom melt from the conductive flux and from the mixed layer
//!   ([`sea_ice`]).
//! - **Land**: 5-layer soil ([`soil`]), bucket hydrology (Manabe 1969) and a snowpack
//!   ([`land`]).
//!
//! **Coupling** (SPEC.md §9.4.3): the surface hands the physics one [`Tile`] per present
//! tile with a linearised heat uptake G = C ΔT_s/Δt_p + g0 + g1 ΔT_s and a cap `t_max`;
//! the surface solve returns T_s⁺, G, evaporation and melt per tile; [`Surface::apply_physics`]
//! advances the reservoirs with those fluxes over Δt.
//!
//! **Budgets.** [`Surface::energy`] is the heat content of soil and mixed layer minus
//! L_f × frozen water (snow, ice) \[J m⁻²\], relative to liquid water; [`Surface::water`] is
//! the stored water (bucket, snow on land and on ice). The ocean is an infinite water
//! reservoir: what enters or leaves it is reported per step (`ocean_uptake`), as are runoff
//! and `ice_cap_flux` (the energy gained when frozen mass above a cap is discarded).
//!
//! Knows nothing about spectral space: it works on grid-point arrays.

pub mod albedo;
pub mod geography;
pub mod land;
pub mod ocean_slab;
pub mod sea_ice;
pub mod soil;

use crate::model::RestartArray;
use crate::spectral::Complex;
use crate::Real;
use albedo::AlbedoParams;
use soil::{SoilColumn, NSOIL};

/// Maximum number of surface tiles per cell (open ocean, sea ice, land).
pub const MAX_TILES: usize = 3;
/// Exchange slots of the tiles.
pub const SLOT_OCEAN: usize = 0;
pub const SLOT_ICE: usize = 1;
pub const SLOT_LAND: usize = 2;

/// Kind of a surface tile (sets its roughness formula).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Ocean,
    SeaIce,
    Land,
}

/// One surface tile of a cell as the physics sees it during a step (the coupler
/// interface, SPEC.md §9.4.3). The surface fills the inputs; the surface solve writes the
/// outputs.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub kind: TileKind,
    /// Exchange slot (`SLOT_*`).
    pub slot: usize,
    /// Area fraction of the cell.
    pub frac: f64,
    /// Surface (skin) temperature \[K\].
    pub ts: f64,
    /// Heat capacity per area \[J m⁻² K⁻¹\]; infinite = prescribed temperature.
    pub heat_capacity: f64,
    /// Further heat uptake G = g0 + g1 ΔT_s \[W m⁻²\], \[W m⁻² K⁻¹\] (soil, ice conduction;
    /// g0 = −q-flux for a slab).
    pub g0: f64,
    pub g1: f64,
    /// Highest surface temperature \[K\]: above it the surplus melts snow or ice.
    pub t_max: f64,
    /// Largest melt the tile can take \[W m⁻²\] (L_f × snow / Δt_p on land; infinite on
    /// sea ice). Energy above it warms the tile past `t_max`.
    pub melt_max: f64,
    /// Broadband albedo.
    pub albedo: f64,
    /// Evaporation efficiency β ∈ [0, 1].
    pub beta: f64,
    /// Momentum roughness \[m\] for land tiles (0 = the default).
    pub z0: f64,
    /// Latent heat of the tile's evaporation above L_v \[J/kg\]: L_f × the share that
    /// sublimates from snow or ice.
    pub fusion: f64,
    /// Outputs: T_s⁺ \[K\], heat into the tile body G, melt \[W m⁻²\], evaporation
    /// \[kg m⁻² s⁻¹\].
    pub ts_new: f64,
    pub ground: f64,
    pub melt: f64,
    pub evap: f64,
}

impl Tile {
    pub fn new(kind: TileKind, frac: f64, ts: f64, heat_capacity: f64, albedo: f64) -> Self {
        let slot = match kind {
            TileKind::Ocean => SLOT_OCEAN,
            TileKind::SeaIce => SLOT_ICE,
            TileKind::Land => SLOT_LAND,
        };
        Tile {
            kind,
            slot,
            frac,
            ts,
            heat_capacity,
            g0: 0.0,
            g1: 0.0,
            t_max: f64::INFINITY,
            melt_max: f64::INFINITY,
            albedo,
            beta: 1.0,
            z0: 0.0,
            fusion: 0.0,
            ts_new: ts,
            ground: 0.0,
            melt: 0.0,
            evap: 0.0,
        }
    }
}

/// Surface parameters (the same for every cell).
#[derive(Debug, Clone)]
pub struct SurfaceParams {
    /// Prescribed SST (no sea ice, the ocean takes any flux).
    pub fixed: bool,
    /// Mixed-layer heat capacity \[J m⁻² K⁻¹\].
    pub ocean_c: f64,
    /// Sea ice on/off.
    pub sea_ice: bool,
    /// Freezing point of sea water \[K\] (271.35, −1.8 °C).
    pub t_freeze_sea: f64,
    /// Melting point of snow and ice surfaces \[K\].
    pub t_melt: f64,
    /// Latent heat of fusion \[J/kg\] (from the condensable; 0 in dry models).
    pub lf: f64,
    /// Ice and snow density \[kg m⁻³\], conductivities \[W m⁻¹ K⁻¹\] (CAM6 §8.2 table).
    pub rho_ice: f64,
    pub rho_snow: f64,
    pub k_ice: f64,
    pub k_snow: f64,
    /// Sea-ice concentration scale h₀ (sic = min(1, h/h₀)) and thickness cap \[m\].
    pub ice_h0: f64,
    pub ice_h_max: f64,
    /// Bucket capacity \[kg m⁻²\] and the snow cap \[kg m⁻²\].
    pub bucket_capacity: f64,
    pub snow_cap: f64,
    pub soil: SoilColumn,
    pub albedo: AlbedoParams,
}

impl Default for SurfaceParams {
    fn default() -> Self {
        SurfaceParams {
            fixed: false,
            ocean_c: ocean_slab::heat_capacity(50.0),
            sea_ice: true,
            t_freeze_sea: 271.35,
            t_melt: 273.15,
            lf: 3.34e5,
            rho_ice: 917.0,
            rho_snow: 330.0,
            k_ice: 2.034,
            k_snow: 0.31,
            ice_h0: 0.5,
            ice_h_max: 50.0,
            bucket_capacity: 150.0,
            snow_cap: 5000.0,
            soil: SoilColumn::layered(2.0e6, 1.0, None),
            albedo: AlbedoParams::default(),
        }
    }
}

/// Fluxes of the last [`Surface::apply_physics`] per point (cell area).
#[derive(Debug, Clone, Default)]
pub struct SurfaceFluxes {
    /// Runoff \[kg m⁻² s⁻¹\].
    pub runoff: Vec<f64>,
    /// Water into the ocean reservoir \[kg m⁻² s⁻¹\] (negative: taken from it).
    pub ocean_uptake: Vec<f64>,
    /// Energy gained by discarding frozen mass above the caps \[W m⁻²\].
    pub ice_cap: Vec<f64>,
    /// Prescribed ocean heat-flux convergence applied \[W m⁻²\].
    pub qflux: Vec<f64>,
}

/// Surface state per grid point.
#[derive(Debug, Clone)]
pub struct Surface {
    pub params: SurfaceParams,
    /// Land fraction of the cell.
    pub land_fraction: Vec<f64>,
    /// Land albedo maps \[visible, near-IR\] (default `params.albedo.land`).
    pub land_albedo: Vec<[f64; 2]>,
    /// Land momentum roughness \[m\] (0 = the boundary layer's default).
    pub land_z0: Vec<f64>,
    /// Prescribed ocean heat-flux convergence per ocean area \[W m⁻²\].
    pub qflux: Vec<f64>,
    /// Mixed-layer (sea surface) temperature \[K\].
    pub t_ml: Vec<f64>,
    /// Mean sea-ice thickness per ocean area \[m\], snow on the ice per ocean area
    /// \[kg m⁻²\], ice surface temperature \[K\].
    pub ice_h: Vec<f64>,
    pub ice_snow: Vec<f64>,
    pub ice_ts: Vec<f64>,
    /// Soil layer temperatures \[K\], bucket water and snow water equivalent \[kg m⁻²\].
    pub soil_t: Vec<[f64; NSOIL]>,
    pub bucket: Vec<f64>,
    pub snow: Vec<f64>,
    /// Friction velocity of the last physics step \[m s⁻¹\] (ocean roughness).
    pub ustar: Vec<f64>,
    /// Fluxes of the last step.
    pub fluxes: SurfaceFluxes,
}

impl Surface {
    /// An aquaplanet with mixed-layer temperature `ts`.
    pub fn aquaplanet(params: SurfaceParams, ts: Vec<f64>) -> Self {
        let n = ts.len();
        let z = vec![0.0; n];
        Surface {
            land_albedo: vec![params.albedo.land; n],
            params,
            land_fraction: z.clone(),
            land_z0: z.clone(),
            qflux: z.clone(),
            ice_h: z.clone(),
            ice_snow: z.clone(),
            ice_ts: ts.clone(),
            soil_t: ts.iter().map(|&t| [t; NSOIL]).collect(),
            bucket: z.clone(),
            snow: z.clone(),
            ustar: z.clone(),
            fluxes: SurfaceFluxes {
                runoff: z.clone(),
                ocean_uptake: z.clone(),
                ice_cap: z.clone(),
                qflux: z,
            },
            t_ml: ts,
        }
    }

    /// Set the land fraction; land starts with the bucket `initial_bucket` × capacity full.
    pub fn set_land(&mut self, frac: &[f64], initial_bucket: f64) {
        for (p, &f) in frac.iter().enumerate() {
            self.land_fraction[p] = f.clamp(0.0, 1.0);
            if f > 0.0 {
                self.bucket[p] = initial_bucket * self.params.bucket_capacity;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.t_ml.len()
    }
    pub fn is_empty(&self) -> bool {
        self.t_ml.is_empty()
    }

    /// Sea-ice concentration (fraction of the ocean area).
    pub fn sic(&self, p: usize) -> f64 {
        if self.ice_h[p] <= 0.0 {
            0.0
        } else {
            (self.ice_h[p] / self.params.ice_h0).min(1.0)
        }
    }

    /// Area fractions of the cell (open ocean, sea ice, land).
    pub fn fractions(&self, p: usize) -> [f64; MAX_TILES] {
        let fl = self.land_fraction[p];
        let a = self.sic(p);
        [(1.0 - fl) * (1.0 - a), (1.0 - fl) * a, fl]
    }

    /// Land snow cover fraction and the soil wetness β of point p.
    pub fn land_cover(&self, p: usize) -> (f64, f64) {
        let s = self.snow[p];
        let fs = s / (s + self.params.albedo.snow_cover_land);
        let beta = (self.bucket[p] / (0.75 * self.params.bucket_capacity)).clamp(0.0, 1.0);
        (fs, beta)
    }

    /// Area-mean skin temperature of point p \[K\].
    pub fn ts_cell(&self, p: usize) -> f64 {
        let f = self.fractions(p);
        f[SLOT_OCEAN] * self.t_ml[p]
            + f[SLOT_ICE] * self.ice_ts[p]
            + f[SLOT_LAND] * self.soil_t[p][0]
    }

    /// The tiles of point p for a physics step of length `dtp`.
    pub fn tiles_at(&self, p: usize, dtp: f64, out: &mut Vec<Tile>) {
        out.clear();
        let pr = &self.params;
        let ab = &pr.albedo;
        let f = self.fractions(p);
        if f[SLOT_OCEAN] > 0.0 {
            let c = if pr.fixed { f64::INFINITY } else { pr.ocean_c };
            let mut t = Tile::new(
                TileKind::Ocean,
                f[SLOT_OCEAN],
                self.t_ml[p],
                c,
                ab.broadband(ab.ocean),
            );
            // the q-flux heats the mixed layer under ice and open water alike
            t.g0 = -self.qflux[p];
            out.push(t);
        }
        if f[SLOT_ICE] > 0.0 {
            let a = self.sic(p);
            let floe = self.ice_h[p] / a;
            let hs = self.ice_snow[p] / a; // kg m⁻² over the ice
            let he = floe + pr.k_ice / pr.k_snow * hs / pr.rho_snow;
            let g1 = pr.k_ice / he;
            let ts = self.ice_ts[p];
            let fsn = hs / (hs + ab.snow_cover_ice);
            let alb = albedo::blend(ab.ice(ts, pr.t_melt), ab.snow(ts, pr.t_melt), fsn);
            let mut t = Tile::new(TileKind::SeaIce, f[SLOT_ICE], ts, 0.0, ab.broadband(alb));
            t.g1 = g1;
            t.g0 = g1 * (ts - pr.t_freeze_sea);
            t.t_max = pr.t_melt;
            t.fusion = pr.lf;
            out.push(t);
        }
        if f[SLOT_LAND] > 0.0 {
            let st = &self.soil_t[p];
            let (g0, g1) = pr.soil.uptake(st, dtp);
            let (fs, beta) = self.land_cover(p);
            let alb = albedo::blend(self.land_albedo[p], ab.snow(st[0], pr.t_melt), fs);
            let mut t = Tile::new(TileKind::Land, f[SLOT_LAND], st[0], 0.0, ab.broadband(alb));
            t.g0 = g0;
            t.g1 = g1;
            t.z0 = self.land_z0[p];
            let beff = fs + (1.0 - fs) * beta;
            t.beta = beff;
            if self.snow[p] > 0.0 {
                t.t_max = pr.t_melt;
                t.melt_max = pr.lf * self.snow[p] / dtp;
            }
            // evaporation from the snow-covered part sublimates: L_v + L_f × share
            let share = if beff > 0.0 { fs / beff } else { 0.0 };
            t.fusion = pr.lf * share;
            out.push(t);
        }
    }

    /// Advance the surface with the fluxes of a physics step: reservoirs get flux × `dt`;
    /// prognostic skin temperatures without heat storage move by `frac` = Δt/Δt_p of the
    /// implicit increment. `exchange` holds [`crate::physics::driver::Exchange`] slots per
    /// point. Fills [`Self::fluxes`].
    pub fn apply_physics(&mut self, exchange: &[f64], frac: f64, dt: f64) {
        use crate::physics::driver::Exchange as X;
        for p in 0..self.len() {
            let e = &exchange[p * X::N..(p + 1) * X::N];
            self.ustar[p] = e[X::USTAR];
            let f = self.fractions(p);
            let (rain, snow) = (e[X::RAIN], e[X::SNOW]);
            let fl = f[SLOT_LAND];
            let (mut runoff, mut uptake, mut cap) = (0.0, 0.0, 0.0);
            if fl < 1.0 {
                let o = sea_ice::OceanFluxes {
                    open: (f[SLOT_OCEAN] > 0.0)
                        .then(|| (e[X::GROUND + SLOT_OCEAN], e[X::TEVAP + SLOT_OCEAN])),
                    ice: (f[SLOT_ICE] > 0.0).then(|| sea_ice::IceFluxes {
                        dts: e[X::TS + SLOT_ICE] - self.ice_ts[p],
                        ground: e[X::GROUND + SLOT_ICE],
                        evap: e[X::TEVAP + SLOT_ICE],
                        melt: e[X::MELT + SLOT_ICE],
                    }),
                    rain,
                    snow,
                    qflux: self.qflux[p],
                };
                let r = sea_ice::step(self, p, &o, frac, dt);
                uptake += (1.0 - fl) * r.uptake;
                cap += (1.0 - fl) * r.ice_cap;
            }
            if fl > 0.0 {
                let r = land::step(
                    self,
                    p,
                    e[X::GROUND + SLOT_LAND],
                    e[X::TEVAP + SLOT_LAND],
                    e[X::MELT + SLOT_LAND],
                    rain,
                    snow,
                    dt,
                );
                runoff += fl * r.runoff;
                cap += fl * r.ice_cap;
            }
            self.fluxes.runoff[p] = runoff;
            self.fluxes.ocean_uptake[p] = uptake;
            self.fluxes.ice_cap[p] = cap;
            self.fluxes.qflux[p] = if self.params.fixed {
                0.0
            } else {
                (1.0 - fl) * self.qflux[p]
            };
        }
    }

    /// Energy content of point p relative to liquid water at 0 K reference \[J m⁻²\]:
    /// soil and mixed-layer heat minus L_f × (snow + ice). A prescribed SST has none.
    pub fn energy(&self, p: usize) -> f64 {
        let pr = &self.params;
        let fl = self.land_fraction[p];
        let ocean = if pr.fixed {
            0.0
        } else {
            pr.ocean_c * self.t_ml[p] - pr.lf * (pr.rho_ice * self.ice_h[p] + self.ice_snow[p])
        };
        let land = pr.soil.heat(&self.soil_t[p]) - pr.lf * self.snow[p];
        (1.0 - fl) * ocean + fl * land
    }

    /// Stored water of point p (bucket, snow on land and on sea ice) \[kg m⁻²\].
    pub fn water(&self, p: usize) -> f64 {
        let fl = self.land_fraction[p];
        (1.0 - fl) * self.ice_snow[p] + fl * (self.bucket[p] + self.snow[p])
    }

    /// Prognostic state for a restart (grid arrays packed two values per complex number).
    pub fn restart_arrays(&self) -> Vec<RestartArray> {
        let mut v = vec![
            ("grid_t_ml".to_string(), pack(&self.t_ml)),
            ("grid_ice_h".to_string(), pack(&self.ice_h)),
            ("grid_ice_snow".to_string(), pack(&self.ice_snow)),
            ("grid_ice_ts".to_string(), pack(&self.ice_ts)),
            ("grid_bucket".to_string(), pack(&self.bucket)),
            ("grid_snow".to_string(), pack(&self.snow)),
            ("grid_ustar".to_string(), pack(&self.ustar)),
        ];
        for k in 0..NSOIL {
            let x: Vec<f64> = self.soil_t.iter().map(|t| t[k]).collect();
            v.push((format!("grid_soil_t{k}"), pack(&x)));
        }
        v
    }

    /// Restore from [`Self::restart_arrays`].
    pub fn load_restart(&mut self, arrays: &[RestartArray]) -> Result<(), String> {
        let get = |name: &str, out: &mut Vec<f64>| -> Result<(), String> {
            let a = arrays
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| format!("restart lacks {name}"))?;
            unpack(&a.1, out);
            Ok(())
        };
        get("grid_t_ml", &mut self.t_ml)?;
        get("grid_ice_h", &mut self.ice_h)?;
        get("grid_ice_snow", &mut self.ice_snow)?;
        get("grid_ice_ts", &mut self.ice_ts)?;
        get("grid_bucket", &mut self.bucket)?;
        get("grid_snow", &mut self.snow)?;
        get("grid_ustar", &mut self.ustar)?;
        let mut x = vec![0.0; self.len()];
        for k in 0..NSOIL {
            get(&format!("grid_soil_t{k}"), &mut x)?;
            for (t, v) in self.soil_t.iter_mut().zip(&x) {
                t[k] = *v;
            }
        }
        Ok(())
    }
}

/// f64 values two per complex number (exact in the f64 build).
fn pack(x: &[f64]) -> Vec<Complex> {
    x.chunks(2)
        .map(|c| Complex::new(c[0] as Real, c.get(1).copied().unwrap_or(0.0) as Real))
        .collect()
}

fn unpack(packed: &[Complex], out: &mut [f64]) {
    for (i, c) in packed.iter().enumerate() {
        out[2 * i] = c.re as f64;
        if 2 * i + 1 < out.len() {
            out[2 * i + 1] = c.im as f64;
        }
    }
}
