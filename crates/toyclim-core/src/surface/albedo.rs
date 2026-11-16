//! Surface albedo (prototype: per-point value with an ice–albedo temperature switch;
//! replaced by per-tile, per-band albedo in P2/P3, SPEC.md §9.4.6).

use super::Surface;
use crate::Real;

impl Surface {
    /// Effective surface albedo at point p (negative = the radiation scheme's default).
    pub fn albedo_at(&self, p: usize) -> Real {
        if self.ice_albedo >= 0.0 {
            let freeze = if self.is_land(p) {
                self.land_freeze
            } else {
                self.ocean_freeze
            };
            if self.ts[p] < freeze {
                return self.ice_albedo;
            }
        }
        self.albedo[p]
    }
}
