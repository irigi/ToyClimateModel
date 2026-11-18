//! Surface model (prototype): slab (mixed-layer) ocean and simple land with a bucket
//! (SPEC.md §9.1; rewritten in P2 as tiles with land, snow and sea ice).
//!
//! Knows nothing about spectral space: it works on grid-point arrays, and the
//! atmosphere hands it fluxes per point.

pub mod albedo;
pub mod geography;
pub mod land;
pub mod ocean_slab;

use crate::config::SurfaceConfig;
use crate::model::RestartArray;
use crate::spectral::Complex;
use crate::Real;

/// Maximum number of surface tiles per cell (open ocean, sea ice, land; P2).
pub const MAX_TILES: usize = 3;

/// Kind of a surface tile (sets its roughness formula).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Ocean,
    SeaIce,
    Land,
}

/// One surface tile of a cell as the physics sees it during a step (the coupler
/// interface, SPEC.md §9.4.3). The surface fills it; the surface solve writes `ts_new`.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub kind: TileKind,
    /// Area fraction of the cell.
    pub frac: f64,
    /// Surface (skin) temperature \[K\].
    pub ts: f64,
    /// Heat capacity per area \[J m⁻² K⁻¹\]; infinite = prescribed temperature.
    pub heat_capacity: f64,
    /// Broadband albedo.
    pub albedo: f64,
    /// Evaporation efficiency β ∈ [0, 1].
    pub beta: f64,
    /// Momentum roughness \[m\] for land tiles (0 = the default).
    pub z0: f64,
    /// Prescribed heat-flux convergence below the tile \[W m⁻²\] (ocean q-flux).
    pub qflux: f64,
    /// Surface temperature after the physics step \[K\] (output).
    pub ts_new: f64,
}

impl Tile {
    pub fn new(kind: TileKind, frac: f64, ts: f64, heat_capacity: f64, albedo: f64) -> Self {
        Tile {
            kind,
            frac,
            ts,
            heat_capacity,
            albedo,
            beta: 1.0,
            z0: 0.0,
            qflux: 0.0,
            ts_new: ts,
        }
    }
}

/// Surface state and parameters per grid point.
#[derive(Debug, Clone)]
pub struct Surface {
    /// Surface temperature per grid point \[K\].
    pub ts: Vec<Real>,
    /// Heat capacity per unit area per point \[J m⁻² K⁻¹\] (ocean ρ_w c_w h, land soil).
    pub heat_capacity: Vec<f64>,
    /// Hold T_s fixed (prescribed SST) instead of integrating it.
    pub fixed: bool,
    /// Land fraction per point (cells with ≥ 0.5 are treated as land).
    pub land_fraction: Vec<Real>,
    /// Surface albedo per point; negative = use the radiation scheme's default.
    pub albedo: Vec<Real>,
    /// Bucket soil water \[kg m⁻²\] (land points only).
    pub bucket: Vec<Real>,
    /// Bucket capacity \[kg m⁻²\]; evaporation efficiency β = min(1, W / (0.75 W_max)).
    pub bucket_capacity: f64,
    /// Albedo of ice (ocean below `ocean_freeze`) and snow (land below `land_freeze`);
    /// negative disables the ice–albedo switch.
    pub ice_albedo: Real,
    pub ocean_freeze: Real,
    pub land_freeze: Real,
    /// Prescribed ocean heat-flux convergence per point \[W m⁻²\] (zero over land; global
    /// ocean mean zero). Added to the surface heat budget.
    pub qflux: Vec<Real>,
    /// Friction velocity of the last physics step \[m s⁻¹\] (ocean roughness; restart state).
    pub ustar: Vec<Real>,
}

impl Surface {
    /// An aquaplanet slab with initial surface temperature `ts`, configured by `sc`.
    pub fn slab(sc: &SurfaceConfig, ts: Vec<Real>) -> Self {
        let n = ts.len();
        let ocean_c = ocean_slab::heat_capacity(sc.mixed_layer_depth);
        let ocean_albedo = sc.ocean_albedo.unwrap_or(-1.0) as Real;
        Surface {
            ts,
            heat_capacity: vec![ocean_c; n],
            fixed: sc.fixed,
            land_fraction: vec![0.0; n],
            albedo: vec![ocean_albedo; n],
            bucket: vec![0.0; n],
            bucket_capacity: sc.bucket_capacity,
            ice_albedo: sc.ice_albedo as Real,
            ocean_freeze: sc.ocean_freeze as Real,
            land_freeze: sc.land_freeze as Real,
            qflux: vec![0.0; n],
            ustar: vec![0.0; n],
        }
    }

    /// Use `albedo` wherever no albedo was configured (the radiation scheme's default).
    pub fn set_default_albedo(&mut self, albedo: f64) {
        for a in self.albedo.iter_mut().filter(|a| **a < 0.0) {
            *a = albedo as Real;
        }
    }

    /// The tiles of point `p` (one per cell until P2: land, or ocean that the prototype
    /// ice switch may mark as sea ice for its roughness).
    pub fn tiles_at(&self, p: usize, out: &mut Vec<Tile>) {
        out.clear();
        let land = self.is_land(p);
        let frozen = self.ice_albedo >= 0.0
            && self.ts[p]
                < if land {
                    self.land_freeze
                } else {
                    self.ocean_freeze
                };
        let kind = match (land, frozen) {
            (true, _) => TileKind::Land,
            (false, true) => TileKind::SeaIce,
            (false, false) => TileKind::Ocean,
        };
        let c = if self.fixed {
            f64::INFINITY
        } else {
            self.heat_capacity[p]
        };
        let mut t = Tile::new(kind, 1.0, self.ts[p] as f64, c, self.albedo_at(p) as f64);
        t.beta = self.wetness(p) as f64;
        t.qflux = self.qflux[p] as f64;
        out.push(t);
    }

    /// Apply a physics step: tile temperatures move towards the implicit solution
    /// `ts_new` by the fraction `frac` = Δt/Δt_p (the leapfrog atmosphere receives the
    /// fluxes at the rate (T_s⁺ − T_s)·C/Δt_p, the surface advances by Δt), u* is stored,
    /// and the land bucket gets precipitation − evaporation over `dt`.
    /// `exchange` holds [`crate::physics::driver::Exchange`] slots per point.
    pub fn apply_physics(&mut self, exchange: &[f64], frac: f64, dt: f64) {
        use crate::physics::driver::Exchange as X;
        let cap = self.bucket_capacity as Real;
        for p in 0..self.ts.len() {
            let e = &exchange[p * X::N..(p + 1) * X::N];
            if !self.fixed {
                let t = self.ts[p] as f64;
                self.ts[p] = (t + frac * (e[X::TS] - t)) as Real;
            }
            self.ustar[p] = e[X::USTAR] as Real;
            if self.is_land(p) {
                let w = self.bucket[p] as f64 + dt * (e[X::PRECIP] - e[X::EVAP]);
                self.bucket[p] = (w as Real).clamp(0.0, cap);
            }
        }
    }

    /// Turn the cells with land fraction ≥ 0.5 into land (heat capacity, albedo, bucket).
    pub fn set_land(&mut self, sc: &SurfaceConfig, frac: &[f64]) {
        for (p, &f) in frac.iter().enumerate() {
            self.land_fraction[p] = f as Real;
            if f >= 0.5 {
                self.heat_capacity[p] = sc.land_heat_capacity;
                self.albedo[p] = sc.land_albedo as Real;
                self.bucket[p] = (sc.initial_bucket * sc.bucket_capacity) as Real;
            }
        }
    }

    pub fn is_land(&self, p: usize) -> bool {
        self.land_fraction[p] >= 0.5
    }

    /// Prognostic surface state for a restart: grid arrays packed two values per complex
    /// number (`grid_ts`, `grid_bucket`, `grid_ustar`).
    pub fn restart_arrays(&self) -> Vec<RestartArray> {
        vec![
            ("grid_ts".to_string(), pack(&self.ts)),
            ("grid_bucket".to_string(), pack(&self.bucket)),
            ("grid_ustar".to_string(), pack(&self.ustar)),
        ]
    }

    /// Restore from [`Self::restart_arrays`] (`grid_bucket`, `grid_ustar` are optional for old
    /// restarts).
    pub fn load_restart(&mut self, arrays: &[RestartArray]) -> Result<(), String> {
        let ts = arrays
            .iter()
            .find(|(n, _)| n == "grid_ts")
            .ok_or("restart lacks grid_ts")?;
        unpack(&ts.1, &mut self.ts);
        if let Some(b) = arrays.iter().find(|(n, _)| n == "grid_bucket") {
            unpack(&b.1, &mut self.bucket);
        }
        if let Some(b) = arrays.iter().find(|(n, _)| n == "grid_ustar") {
            unpack(&b.1, &mut self.ustar);
        }
        Ok(())
    }
}

fn pack(x: &[Real]) -> Vec<Complex> {
    x.chunks(2)
        .map(|c| Complex::new(c[0], c.get(1).copied().unwrap_or(0.0)))
        .collect()
}

fn unpack(packed: &[Complex], out: &mut [Real]) {
    for (i, c) in packed.iter().enumerate() {
        out[2 * i] = c.re;
        if 2 * i + 1 < out.len() {
            out[2 * i + 1] = c.im;
        }
    }
}
