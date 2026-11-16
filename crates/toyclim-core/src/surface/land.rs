//! Land hydrology (prototype): a bucket after Manabe (1969).

use super::Surface;
use crate::Real;

impl Surface {
    /// Evaporation efficiency β = min(1, W / (0.75 W_max)) on land, 1 over the ocean.
    pub fn wetness(&self, p: usize) -> Real {
        if self.is_land(p) {
            (self.bucket[p] / (0.75 * self.bucket_capacity as Real)).min(1.0)
        } else {
            1.0
        }
    }
}

/// Bucket update on land points: W ← clamp(W + Δt (P − E), 0, W_max); water above the
/// capacity runs off and leaves the system.
pub(super) fn bucket_step(s: &mut Surface, dt: f64, precip: &[Real], evap: &[Real]) {
    let cap = s.bucket_capacity as Real;
    for p in 0..s.bucket.len() {
        if s.land_fraction[p] >= 0.5 {
            let w = s.bucket[p] + dt as Real * (precip[p] - evap[p]);
            s.bucket[p] = w.clamp(0.0, cap);
        }
    }
}
