//! The common interface of all models, used by the headless runner and the viewer.

use crate::grid::GaussianGrid;
use crate::spectral::Complex;
use crate::Real;

/// A grid-point field `[lev][lat][lon]` (nlev = 1 for 2D fields).
#[derive(Debug, Clone)]
pub struct GridField {
    pub nlev: usize,
    pub data: Vec<Real>,
}

/// A named spectral array in a restart file.
pub type RestartArray = (String, Vec<Complex>);

/// Model-independent interface.
pub trait Model: Send {
    /// Short model name ("shallow_water", "primitive").
    fn kind(&self) -> &'static str;
    /// Advance one time step.
    fn step(&mut self);
    /// Model time \[s\].
    fn time(&self) -> f64;
    /// Number of steps taken.
    fn step_count(&self) -> u64;
    /// Time step \[s\].
    fn dt(&self) -> f64;
    fn grid(&self) -> &GaussianGrid;
    /// Full σ levels (empty for 2D models).
    fn sigma(&self) -> Vec<f64> {
        Vec::new()
    }
    /// Names of fields available from [`Model::field`].
    fn field_names(&self) -> Vec<&'static str>;
    /// A grid field by name, or None.
    fn field(&self, name: &str) -> Option<GridField>;
    /// Scalar diagnostics (name, value) for the current state, in a fixed order.
    fn diagnostics(&self) -> Vec<(&'static str, f64)>;
    /// False if the state contains NaN or Inf.
    fn is_finite(&self) -> bool;
    /// Spectral state for a restart (both leapfrog levels), plus scalar counters.
    fn restart_state(&self) -> (Vec<RestartArray>, Vec<(String, f64)>);
    /// Restore from [`Model::restart_state`] output.
    fn load_restart_state(
        &mut self,
        arrays: &[RestartArray],
        scalars: &[(String, f64)],
    ) -> Result<(), String>;
}

/// Max over a slice (NaN-propagating to make blow-ups visible).
pub fn max_abs(x: &[Real]) -> f64 {
    let mut m = 0.0f64;
    for &v in x {
        let a = (v as f64).abs();
        if a.is_nan() {
            return f64::NAN;
        }
        m = m.max(a);
    }
    m
}
