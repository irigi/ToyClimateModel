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
