//! The column physics interface.
//!
//! The model calls each scheme of a `Vec<Box<dyn ColumnPhysics>>` in a fixed order for
//! every column (columns in parallel), and adds the accumulated tendencies to the
//! grid-point tendencies before the spectral analysis. Schemes *add* to the output
//! slices; they never overwrite another scheme's contribution.
//!
//! Room for later milestones without changing call sites: tracers (humidity, M7) are a
//! `[tracer][lev]` block, surface fields and per-scheme diagnostics (e.g. precipitation)
//! are named slots declared by the scheme.

use crate::planet::Planet;
use crate::Real;

/// Read-only view of one column at the physics time level. Index 0 is the top level.
pub struct ColumnIn<'a> {
    /// Latitude \[rad\].
    pub lat: Real,
    /// Longitude \[rad\].
    pub lon: Real,
    /// Column index (row-major Gaussian grid), for schemes with per-column state.
    pub index: usize,
    /// Full σ levels.
    pub sigma: &'a [Real],
    /// Half σ levels (L+1 values, 0 at the top, 1 at the surface).
    pub sigma_half: &'a [Real],
    /// Surface pressure \[Pa\].
    pub ps: Real,
    /// Zonal wind \[m/s\].
    pub u: &'a [Real],
    /// Meridional wind \[m/s\].
    pub v: &'a [Real],
    /// Temperature \[K\].
    pub t: &'a [Real],
    /// Tracers `[tracer][lev]` flattened (empty until M7).
    pub tracers: &'a [Real],
    /// Surface geopotential \[m² s⁻²\].
    pub phis: Real,
    /// Model time \[s\].
    pub time: f64,
    /// Length of the step the tendencies are applied over (2Δt for leapfrog) \[s\].
    pub dt: f64,
}

/// Tendencies of one column, accumulated over the schemes.
pub struct ColumnOut<'a> {
    /// ∂u/∂t \[m s⁻²\].
    pub du: &'a mut [Real],
    /// ∂v/∂t \[m s⁻²\].
    pub dv: &'a mut [Real],
    /// ∂T/∂t \[K s⁻¹\].
    pub dt: &'a mut [Real],
    /// Tracer tendencies `[tracer][lev]` (empty until M7).
    pub dtracers: &'a mut [Real],
    /// This scheme's diagnostic slots (in the order of `diagnostic_names`).
    pub diag: &'a mut [Real],
}

/// A column physics scheme.
pub trait ColumnPhysics: Send + Sync {
    /// Short name (for logs and diagnostics).
    fn name(&self) -> &'static str;
    /// Names of per-column diagnostics this scheme writes (e.g. "precip"); default none.
    fn diagnostic_names(&self) -> Vec<&'static str> {
        Vec::new()
    }
    /// Add this scheme's tendencies for one column to `out`.
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut);
}
