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

    /// Advance the surface by `dt` \[s\] with the net downward heat flux `heat`
    /// \[W m⁻²\], precipitation and evaporation \[kg m⁻² s⁻¹\] per point.
    pub fn step(&mut self, dt: f64, heat: &[Real], precip: &[Real], evap: &[Real]) {
        if !self.fixed {
            for (p, (t, h)) in self.ts.iter_mut().zip(heat).enumerate() {
                *t += (dt / self.heat_capacity[p]) as Real * (h + self.qflux[p]);
            }
        }
        land::bucket_step(self, dt, precip, evap);
    }

    /// Prognostic surface state for a restart: grid arrays packed two values per complex
    /// number (`grid_ts`, `grid_bucket`).
    pub fn restart_arrays(&self) -> Vec<RestartArray> {
        vec![
            ("grid_ts".to_string(), pack(&self.ts)),
            ("grid_bucket".to_string(), pack(&self.bucket)),
        ]
    }

    /// Restore from [`Self::restart_arrays`] (`grid_bucket` is optional for old restarts).
    pub fn load_restart(&mut self, arrays: &[RestartArray]) -> Result<(), String> {
        let ts = arrays
            .iter()
            .find(|(n, _)| n == "grid_ts")
            .ok_or("restart lacks grid_ts")?;
        unpack(&ts.1, &mut self.ts);
        if let Some(b) = arrays.iter().find(|(n, _)| n == "grid_bucket") {
            unpack(&b.1, &mut self.bucket);
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
