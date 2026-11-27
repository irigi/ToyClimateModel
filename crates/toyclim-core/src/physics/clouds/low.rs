//! Low cloud of [`DiagnosticClouds`] after Slingo (1987), `low_cloud = "slingo"`.
//!
//! P4 question 13 option (iv), owner decision of 2026-11-23; off by default (`"rh"` is the
//! P4 scheme). Slingo, J. M.,
//! 1987: The development and verification of a cloud prediction scheme for the ECMWF model.
//! Q. J. R. Meteorol. Soc. 113, 899–927, §2c, pp. 905–907, read from the PDF supplied by the
//! owner (equations checked against the page images, the text layer garbles eq. 9).
//! Transcribed:
//! - eq. (5) RH_e = RH (1.0 − C_c), "the relative humidity of the layer after adjustment for
//!   the presence of convective clouds";
//! - eq. (6) C_L' = [Max{0.0, (RH_e − 0.8)/0.2}]²;
//! - eq. (7) "C_L = 0 if there is subsidence, i.e. ω ≥ 0. There is a linear transition up to
//!   a weak ascent: C_L = C_L'(−10.0 ω) for ω ≥ −0.1 where 0.1 Pa s⁻¹ = 3.6 mb h⁻¹.
//!   Otherwise C_L = C_L'";
//! - eq. (8) inversion cloud C_L' = −6.67 Δθ/Δp − 0.667, "where Δθ/Δp is the lapse rate
//!   (K mb⁻¹) in the most stable layer below 750 mb" (zero at −0.1 K mb⁻¹, one at −0.25);
//! - eq. (9) with the relative humidity at the base of the inversion: C_L = 0 if
//!   RH_base < 0.6, C_L = C_L'{1.0 − (0.8 − RH_base)/0.2} for 0.6 ≤ RH_base ≤ 0.8, C_L = C_L'
//!   otherwise;
//! - the two low-cloud classes "are almost mutually exclusive so that the two schemes combine
//!   without difficulty" (the combination is not written; here the larger of the two);
//! - Fig. 1: low cloud below about 800 mb; "no cloud is allowed in the lowest model layer";
//!   layer clouds occupy one model layer, the largest amount of each of the three parts
//!   being ascribed to its layer of maximum cloudiness (not done here: every layer keeps
//!   its own fraction, as in the rest of this scheme);
//! - "a different vertical resolution may need changes, particularly to the dependence on
//!   atmospheric stability (Eq. (8))" — the eq. (8) constants are knobs.
//!
//! Here, at σ ≥ `low_sigma` (0.8, Fig. 1): the RH cloud C_L' is eq. (6) with RH_e and the
//! threshold `low_rh_crit` (0.8) if `low_rh = "slingo"` (default), or this scheme's
//! Sundqvist fraction with RH_c(σ) if `low_rh = "sundqvist"`; it is multiplied by the eq. (7)
//! factor min(1, max(0, −ω/`low_omega`)) with `low_omega` = 0.1 Pa s⁻¹ (ω of the state the
//! physics sees, [`ColumnCtx::omega`]; no ω, as in the SCM, counts as ω = 0). The inversion
//! cloud: over the pairs of adjacent full levels (k, k+1) with p_k ≥ `inv_pressure`
//! (750 hPa) and k+1 above the lowest level, the most negative
//! Δθ/Δp = (θ_k − θ_{k+1})/(p_k − p_{k+1}) \[K hPa⁻¹\] (θ = T (1000 hPa/p)^κ) defines the
//! inversion; C_inv = clamp(−`inv_slope` Δθ/Δp − `inv_offset`, 0, 1) (6.67 hPa K⁻¹, 0.667),
//! times the eq. (9) factor with RH_base the relative humidity of level k+1 (the base);
//! it sits in layer k+1 and combines with the RH cloud of that layer as the maximum. The
//! freeze-dry factor multiplies the RH cloud only (see [`super`]); σ_min is unchanged.
//! Mid and high levels keep the Sundqvist cloud. With `cre_split`, the stratiform-only block holds the
//! new low cloud. Two more diagnostics: `cl_inv`, the inversion cloud C_inv of the column
//! (0 without an inversion), and `inv_dthdp`, the Δθ/Δp of its most stable layer
//! \[K hPa⁻¹\] (0 if no pair qualifies, e.g. p_s below 750 hPa).
//!
//! **L26 does not resolve the inversion eq. (8) needs** (owner step 3 of 2026-11-23,
//! `tools/inversion_check.py`: the trade belts are as stable as the warm pool, the deck
//! inversions are spread over two layers and eq. (8) gives 0.04–0.19 there). "slingo" stays
//! in the code, off; the owner chose "slingo_lts" instead.
//!
//! ## `low_cloud = "slingo_lts"`
//!
//! The RH low cloud with the eq. (7) gate exactly as "slingo" (same knobs `low_sigma`,
//! `low_rh`, `low_rh_crit`, `low_omega`; no RH cloud in the lowest layer), and in place of
//! eqs. (8)–(9) the lower-tropospheric-stability stratus law of Klein & Hartmann (1993,
//! J. Climate 6, 1587–1606). **Source of the numbers:** the CAM implementation, ESCOMP/CAM tag
//! `cam_cesm2_2_rel`, `src/physics/cam/cloud_fraction.F90`, subroutine `cldfrc`, and
//! `bld/namelist_files/namelist_defaults_cam.xml` (fetched 2026-11-23), with the same
//! constants quoted on the CESM forum (bb.cgd.ucar.edu thread "parameterization of cloud in
//! CAM3"); **neither the 1993 paper nor the CAM3 technical note (NCAR/TN-464+STR) was read**
//! (AMS 403; UCAR servers refused the download). Code written independently; nothing copied.
//! Transcribed from the CAM source:
//! - strat = min(1, max(0, f_ocn ((θ(k700) − θ_s) · 0.057 − 0.5573))), k700 the level nearest
//!   700 hPa by reference pressure;
//! - θ_s = (SST − Γ φ_s / g)(p₀/p_s)^κ, Γ = 6.5 K km⁻¹, p₀ = 1000 hPa, only where the ocean
//!   fraction (CAM `ocnfrac`, open ocean without sea ice) exceeds 0.01;
//! - placement: the most stable adjacent pair of levels whose lower level is at p ≥ `premib`
//!   (750 hPa for CAM4 physics; here `inv_pressure`), Δθ/Δp = (θ_lower − θ_upper)/(p_lower −
//!   p_upper); the stratus goes into the lower level of that pair as
//!   min(strat, max(RH_k, RH_{k+1})) and combines with the RH cloud of that layer by maximum.
//! - **Stated deviation (owner decision (A), 2026-11-23):** CAM only forms the stratus where
//!   that most stable Δθ/Δp is below −0.125 K/hPa (and then tries the surface pair). The
//!   trigger is omitted, because L26 does not resolve layer inversions (same argument as for
//!   eq. 8): with it the decks get 0.09–0.37, without it 0.36–0.68 (`tools/lts_check.py`).
//!   Without the trigger a level pair always exists over the ocean, so the surface pair is
//!   never needed and is not coded. CAM caps with the RH already reduced for its convective
//!   cloud; here the plain RH q/q_sat.
//!
//! Diagnostic switch `low_gate` (default true; P4 question 13, owner decision after step A,
//! 2026-11-26): `false` drops the eq. (7) factor (both "slingo" and "slingo_lts"), so the RH
//! low cloud forms under subsidence too; everything else is unchanged.
//!
//! Here: f_ocn = the area fraction of the `Ocean` tiles of the cell and SST their
//! area-weighted skin temperature (no tiles: no stratus); k700 = the level with σ nearest 0.7.
//! Knobs `lts_slope` (0.057 K⁻¹) and `lts_intercept` (−0.5573); `lts_slope = 0` leaves the
//! gated RH cloud alone (the "gate only" diagnostic variant). Freeze-dry multiplies the
//! RH cloud only, not the stratus (as CAM; corrected 2026-11-26); σ_min is unchanged.
//! Diagnostics `cl_lts` (the stratus placed, after the RH cap) and `lts` (θ(k700) − θ_s \[K\], 0 without open ocean) in the slots of
//! `cl_inv` and `inv_dthdp`.

use super::{DiagnosticClouds, LowCloud, LowRh};
use crate::physics::column::{ColumnCtx, ColumnState};
use crate::surface::{Tile, TileKind};

/// Lapse rate of the θ_s height correction \[K m⁻¹\] (CAM `lapse`, U.S. Standard Atmosphere).
const LTS_LAPSE: f64 = 6.5e-3;

impl DiagnosticClouds {
    /// Slingo (1987) eq. (6) with threshold `crit` (his 0.8): [max(0, (RH_e − crit)/(1 − crit))]²,
    /// at most 1.
    pub fn slingo_rh(rh_e: f64, crit: f64) -> f64 {
        let x = ((rh_e - crit) / (1.0 - crit)).clamp(0.0, 1.0);
        x * x
    }

    /// Slingo (1987) eq. (7) factor for ω \[Pa s⁻¹\]: 0 under subsidence (ω ≥ 0), linear to 1
    /// at ω = −`low_omega`.
    /// With `low_gate = false` (diagnostic) the factor is 1.
    pub fn omega_factor(&self, omega: f64) -> f64 {
        if !self.low_gate {
            return 1.0;
        }
        (-omega / self.low_omega).clamp(0.0, 1.0)
    }

    /// Slingo (1987) inversion of a column (eqs. 8–9, see the module documentation):
    /// (base level, C_inv, Δθ/Δp \[K hPa⁻¹\]) or `None` if no pair of levels qualifies.
    pub fn inversion(&self, ctx: &ColumnCtx, t: &[f64], q: &[f64]) -> Option<(usize, f64, f64)> {
        let l = t.len();
        let kappa = ctx.planet.kappa();
        let theta = |k: usize| t[k] * (1.0e5 / ctx.p[k]).powf(kappa);
        let mut best: Option<(usize, f64)> = None;
        for k in 0..l.saturating_sub(2) {
            if ctx.p[k] < self.inv_pressure {
                continue;
            }
            let s = (theta(k) - theta(k + 1)) / ((ctx.p[k] - ctx.p[k + 1]) / 100.0);
            if best.is_none_or(|(_, b)| s < b) {
                best = Some((k + 1, s));
            }
        }
        let (kb, s) = best?;
        let c = (-self.inv_slope * s - self.inv_offset).clamp(0.0, 1.0);
        let rh = q[kb] / self.cond.qsat(ctx.planet, t[kb], ctx.p[kb]);
        // eq. (9): 1 − (0.8 − RH_base)/0.2 = (RH_base − 0.6)/0.2 between 0.6 and 0.8
        let f = ((rh - 0.6) / 0.2).clamp(0.0, 1.0);
        Some((kb, c * f, s))
    }

    /// Lower-tropospheric-stability stratus of a column ("slingo_lts", see the module
    /// documentation): (level, fraction after the RH cap, LTS \[K\]) or `None` without open
    /// ocean or without a qualifying pair of levels.
    pub fn lts_stratus(
        &self,
        ctx: &ColumnCtx,
        t: &[f64],
        q: &[f64],
        tiles: &[Tile],
    ) -> Option<(usize, f64, f64)> {
        let (mut f_ocn, mut sst) = (0.0, 0.0);
        for tile in tiles.iter().filter(|t| t.kind == TileKind::Ocean) {
            f_ocn += tile.frac;
            sst += tile.frac * tile.ts;
        }
        if f_ocn <= 0.01 {
            return None;
        }
        sst /= f_ocn;
        let l = t.len();
        let kappa = ctx.planet.kappa();
        let theta = |k: usize| t[k] * (1.0e5 / ctx.p[k]).powf(kappa);
        let theta_s =
            (sst - LTS_LAPSE * ctx.phis / ctx.planet.gravity) * (1.0e5 / ctx.ps).powf(kappa);
        let k700 = (0..l).min_by(|&a, &b| {
            (ctx.sigma[a] - 0.7)
                .abs()
                .total_cmp(&(ctx.sigma[b] - 0.7).abs())
        })?;
        let lts = theta(k700) - theta_s;
        let strat = (f_ocn * (self.lts_slope * lts + self.lts_intercept)).clamp(0.0, 1.0);
        let mut best: Option<(usize, f64)> = None;
        for k in 1..l {
            if ctx.p[k] < self.inv_pressure {
                continue;
            }
            let s = (theta(k) - theta(k - 1)) / (ctx.p[k] - ctx.p[k - 1]);
            if best.is_none_or(|(_, b)| s < b) {
                best = Some((k, s));
            }
        }
        let (k, _) = best?;
        let rh = |k: usize| q[k] / self.cond.qsat(ctx.planet, t[k], ctx.p[k]);
        let cap = rh(k).max(rh((k + 1).min(l - 1)));
        Some((k, strat.min(cap).max(0.0), lts))
    }

    /// Stratiform fraction of level k with q_sat `qs`, humidity `q`, the layer's convective
    /// fraction `cv` and the column's inversion `inv`. Freeze-dry multiplies the RH cloud
    /// only, before the inversion cloud or stratus is combined with it (as CAM `cldfrc`).
    pub(super) fn layer_cloud(
        &self,
        ctx: &ColumnCtx,
        qs: f64,
        q: f64,
        k: usize,
        cv: f64,
        inv: Option<(usize, f64, f64)>,
    ) -> f64 {
        let sigma = ctx.sigma[k];
        let fd = self.freeze_dry(q, ctx.p[k]);
        if self.low_cloud == LowCloud::Rh {
            return Self::stratiform(q / qs, self.rh_crit(sigma)) * fd;
        }
        let mut c = if k + 1 == ctx.sigma.len() {
            0.0 // Slingo: no RH cloud in the lowest model layer
        } else if sigma >= self.low_sigma {
            let rh = q / qs;
            let base = match self.low_rh {
                LowRh::Slingo => Self::slingo_rh(rh * (1.0 - cv), self.low_rh_crit),
                LowRh::Sundqvist => Self::stratiform(rh, self.rh_crit(sigma)),
            };
            base * self.omega_factor(ctx.omega.get(k).copied().unwrap_or(0.0))
        } else {
            Self::stratiform(q / qs, self.rh_crit(sigma))
        } * fd;
        if let Some((kb, ci, _)) = inv {
            if kb == k {
                c = c.max(ci);
            }
        }
        c
    }

    /// The column's extra low cloud (level, fraction, diagnostic): the Slingo inversion
    /// cloud or the stability stratus.
    pub(super) fn column_inversion(
        &self,
        ctx: &ColumnCtx,
        st: &ColumnState,
        q: &[f64],
        tiles: &[Tile],
    ) -> Option<(usize, f64, f64)> {
        match self.low_cloud {
            LowCloud::Rh => None,
            LowCloud::Slingo => self.inversion(ctx, &st.t, q),
            LowCloud::SlingoLts => self.lts_stratus(ctx, &st.t, q, tiles),
        }
    }

    /// Write the diagnostics of the extra low cloud (`cl_inv`, `inv_dthdp` or `cl_lts`,
    /// `lts`).
    pub(super) fn inversion_diag(&self, inv: Option<(usize, f64, f64)>, diag: &mut [f64]) {
        if self.low_cloud != LowCloud::Rh {
            let i = if self.cre_split { 3 } else { 1 };
            if let Some((_, ci, s)) = inv {
                diag[i] += ci;
                diag[i + 1] += s;
            }
        }
    }
}
