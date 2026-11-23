//! The "anvil" convective cloud geometry of [`DiagnosticClouds`].
//!
//! Convective cloud geometry, `convective` = "column" (default, the P4 scheme) or "anvil"
//! (P4 question 13 option (b)), after Slingo (1987, Q. J. R. Meteorol. Soc. 113, 899–927,
//! §3c and Table 3; read from the PDF supplied by the owner, 2026-11-22). Transcribed:
//! - eq. (1) C_c = a + b ln P, P the convective precipitation at cloud base [mm day⁻¹],
//!   a, b "empirical constants" not printed; Table 1 (C_c 0, 0.1 … 0.8 at P 0.14, 0.31,
//!   0.70, 1.6, 3.4, 7.7, 17, 38, 85 mm day⁻¹) gives b = 0.8 / ln(85 / 0.14) = 0.1248 and
//!   a = −b ln 0.14 = 0.2454; C_c ≤ 0.8;
//! - "in cases of deep convection only 25% of the predicted amount is allowed to occupy the
//!   full depth, the remaining 75% being treated as low-level shallow convection";
//! - eq. (2) anvil cirrus C_H = 2.0 (C_c − 0.3), "provided that convection extends above
//!   400 mb and the convective cloudiness exceeds 40% (i.e. equivalent precipitation rate
//!   of greater than 3.4 mm d⁻¹)"; layer clouds occupy one model layer;
//! - Table 3 (sensitivity study, which improved the tropical OLR): liquid water contents
//!   cumulonimbus 2.5, cumulus 1.0, anvil cirrus 0.035 g m⁻³; control 0.01 q_s ρ;
//! - P is averaged over three hours and the lowest base / highest top over that time are
//!   used (not done here: the instantaneous values, as in "column"; option (a) of question 13
//!   was tested offline and not built).
//!
//! Here: the convective cover C_c keeps this scheme's law min(c_max, b' ln(1 + P)) (knobs
//! `conv_coef`, `conv_max`). If the convective top is above σ = `deep_sigma` (0.4, the
//! paper's 400 mb at p_s = 1000 hPa) the column is "deep": a core of `core_share` (0.25)
//! C_c fills the convecting depth, the rest (1 − share) C_c sits at σ ≥ `shallow_sigma`
//! (0.8, about 2 km, the paper's low/middle threshold; the paper does not give the depth of
//! its low-level part — my reading); otherwise C_c fills the convecting depth as before. The
//! anvil, C_H = min(1, 2 (a + b ln P − 0.3)) with the paper's a, b and only when
//! a + b ln P > 0.4 (P > 3.4 mm day⁻¹), occupies the top convecting layer (the highest
//! layer at σ ≥ σ_min if the top is above it), with in-cloud water `anvil_water`
//! (0.035 g m⁻³ × the layer depth, Table 3). The convective in-cloud water per layer is
//! min(β q_sat, r_max) as for stratiform cloud, but its column total over the convective
//! layers is capped at `conv_water_path` (a knob, not from the paper; 1 kg m⁻² initially).
//! Where convective and stratiform cloud share a layer the stored in-cloud water is their
//! area-weighted mean.

use super::DiagnosticClouds;
use crate::physics::column::{ColumnCtx, ColumnState};
use crate::surface::Tile;

/// Slingo (1987) eq. (1) constants from his Table 1 (P in mm day⁻¹).
const SLINGO_A: f64 = 0.2454;
const SLINGO_B: f64 = 0.1248;

impl DiagnosticClouds {
    /// Slingo's (1987) anvil cover for convective precipitation `p` \[kg m⁻² s⁻¹\]: eq. (2)
    /// with his eq. (1) (Table 1 constants), zero unless his C_c exceeds 0.4.
    pub fn anvil(p: f64) -> f64 {
        let pmm = p * 86400.0;
        if pmm <= 0.0 {
            return 0.0;
        }
        let c = SLINGO_A + SLINGO_B * pmm.ln();
        if c <= 0.4 {
            0.0
        } else {
            (2.0 * (c.min(0.8) - 0.3)).min(1.0)
        }
    }
}

impl DiagnosticClouds {
    /// The "anvil" geometry (see the module documentation).
    pub(super) fn apply_anvil(
        &self,
        ctx: &ColumnCtx,
        st: &ColumnState,
        q: &[f64],
        tiles: &[Tile],
        m: &mut [f64],
        diag: &mut [f64],
    ) {
        let l = st.nlev;
        let (planet, g) = (ctx.planet, ctx.planet.gravity);
        let cc = self.convective(st.conv_precip);
        let top = st.conv_top;
        let active = cc > 0.0 && top + 1 < l;
        let deep = active && ctx.sigma[top] < self.deep_sigma;
        // convective fraction per layer (core and shallow part independent within a layer)
        let conv_frac = |k: usize| -> f64 {
            if !active || k < top || k + 1 >= l || ctx.sigma[k] < self.sigma_min {
                return 0.0;
            }
            if deep {
                let core = self.core_share * cc;
                let low = if ctx.sigma[k] >= self.shallow_sigma {
                    (1.0 - self.core_share) * cc
                } else {
                    0.0
                };
                1.0 - (1.0 - core) * (1.0 - low)
            } else {
                cc
            }
        };
        let raw = |k: usize| {
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            (self.water_ratio * qs).min(self.water_max) * ctx.dp[k] / g
        };
        let total: f64 = (0..l).filter(|&k| conv_frac(k) > 0.0).map(raw).sum();
        let scale = if total > self.conv_water_path && total > 0.0 {
            self.conv_water_path / total
        } else {
            1.0
        };
        // anvil layer: the top convecting layer, or the highest one below σ_min
        let anvil_k = (top..l - 1).find(|&k| ctx.sigma[k] >= self.sigma_min);
        let ca = if deep {
            Self::anvil(st.conv_precip)
        } else {
            0.0
        };
        let inv = self.column_inversion(ctx, st, q, tiles);
        let (mut path, mut conv_path) = (0.0, 0.0);
        for k in 0..l {
            if ctx.sigma[k] < self.sigma_min {
                continue;
            }
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            let ws = raw(k);
            let mut cv = conv_frac(k);
            let cs =
                self.layer_cloud(ctx, qs, q[k], k, cv, inv) * self.freeze_dry(q[k], ctx.sigma[k]);
            let mut wv = scale * ws;
            if cv > 0.0 {
                conv_path += wv;
            }
            if ca > 0.0 && anvil_k == Some(k) {
                // anvil and convective cloud of the layer as independent covers
                let rho = ctx.p[k] / (planet.gas_constant * st.t[k]);
                let wa = self.anvil_water * ctx.dp[k] / (rho * g);
                let c2 = 1.0 - (1.0 - cv) * (1.0 - ca);
                wv = (cv * wv + (c2 - cv) * wa) / c2;
                cv = c2;
            }
            let c = 1.0 - (1.0 - cs) * (1.0 - cv);
            if c <= 0.0 {
                continue;
            }
            let w = (cs * ws + (c - cs) * wv) / c;
            if self.cre_split {
                m[3 * l + k] = self.split_fraction(ctx.sigma[k], cs, cv, c);
            }
            let fi = self.cond.ice_fraction(st.t[k]);
            m[k] = c;
            m[l + k] = (1.0 - fi) * w;
            m[2 * l + k] = fi * w;
            path += c * w;
        }
        diag[0] += path;
        if self.cre_split && conv_path > 0.0 {
            diag[1] += cc;
            diag[2] += conv_path;
        }
        self.inversion_diag(inv, diag);
    }
}
