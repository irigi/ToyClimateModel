//! Diagnostic clouds (SPEC.md §9.4.7, step 6 of §9.3.3): fraction and in-cloud water per
//! layer from the state after convection and condensation, stored in
//! [`SurfaceExchange::cloud`] for the next radiation call. No prognostic cloud water; the
//! scheme changes neither temperature nor humidity.
//!
//! - Stratiform fraction (Sundqvist type): c_s = 1 − √((1 − RH)/(1 − RH_c)) for RH > RH_c,
//!   with RH_c(σ) = RH_u + (RH_s − RH_u) exp(1 − σ^(−n)): RH_s near the surface, RH_u
//!   aloft (the shape of the ECHAM profile, Lohmann & Roeckner 1996; not checked against
//!   the paper, the values are knobs).
//! - Convective fraction from the convective precipitation P of this step, after Slingo
//!   (1987): c_c = min(c_max, b ln(1 + P / 1 mm day⁻¹)), between the top of the convecting
//!   layer and the second level from the bottom. (Slingo's form, a + b ln P, is singular
//!   at small P; the coefficients are knobs.)
//! - Optional "freeze-dry" reduction of the stratiform fraction in cold, dry low levels
//!   (Vavrus & Waliser 2008, J. Climate 21, 5673–5687; the paper could not be fetched, the
//!   form and constants are transcribed from the NCAR CAM source, `cloud_fraction.F90`,
//!   subroutine `cldfrc`, branch `cam_cesm2_2_rel`): for σ ≥ σ_fd,
//!   c_s ← c_s · max(f_min, min(1, q / q_fd)), with q_fd = 3e-3 kg kg⁻¹ and f_min = 0.15.
//!   CAM applies it to its low-cloud RH cloud below the pressure bound `premib`, whose value
//!   is not in that file; σ_fd = 0.75 (750 hPa) is unverified. On by default (P4 frozen knobs); `freeze_dry_q = 0` switches it off.
//! - Combined at each level as independent covers: c = 1 − (1 − c_s)(1 − c_c).
//! - In-cloud water mixing ratio r = min(β q_sat(T, p), r_max), split into ice and liquid
//!   with f_ice(T) of the condensable; path r Δp / g.
//! - No clouds above σ_min.
//!
//! Convective cloud geometry, `convective` = "column" (default, the P4 scheme) or "anvil":
//! see [`anvil`]. Low cloud, `low_cloud` = "rh" (default, the P4 scheme), "slingo" or "slingo_lts": see
//! [`low`].
//!
//! Diagnostic: `clwvi`, the grid-mean condensed water path \[kg m⁻²\] (the total cover
//! `clt` comes from the radiation, which uses these clouds in the next step).
//!
//! Optional CRE split (`cre_split`, diagnostic only, P4 question 13): the memory gains a
//! fourth block with the stratiform-only fraction (c_s after freeze-dry), from which the
//! radiation makes an extra call without the convective cloud (`olr_nc`, `asr_nc`); and
//! two more diagnostics, `cc_conv` (the convective fraction c_c of the column) and
//! `cwp_conv` (its in-cloud water path Σ r Δp / g over the convective layers, 0 without
//! convective cloud). Nothing else changes.
//!
//! With `cre_split_low_sigma` = σ_low > 0 (also diagnostic, P4 question 13, review round of
//! 2026-11-23) the fourth block instead holds the total fraction with the *stratiform* cloud
//! removed at σ ≥ σ_low (the convective fraction of those layers is kept, with the layer's
//! stored in-cloud water; the layers above keep the total), so `olr_nc` and `asr_nc` become
//! the fluxes without low stratiform cloud.

mod anvil;
mod config;
mod low;

pub(crate) use config::diagnostic_clouds;
pub use config::CLOUD_KEYS;

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::condensation::Condensable;

#[derive(Debug, Clone)]
pub struct DiagnosticClouds {
    pub cond: Condensable,
    /// Critical relative humidity near the surface and aloft, and the profile exponent.
    pub rh_crit_surface: f64,
    pub rh_crit_upper: f64,
    pub rh_crit_exponent: f64,
    /// Convective cover coefficient b and cap c_max.
    pub conv_coef: f64,
    pub conv_max: f64,
    /// In-cloud water as a fraction β of q_sat, capped at `water_max` \[kg kg⁻¹\].
    pub water_ratio: f64,
    pub water_max: f64,
    /// No clouds at σ below this.
    pub sigma_min: f64,
    /// Freeze-dry humidity scale q_fd \[kg kg⁻¹\] (0 = off), minimum factor and the σ at and
    /// below which it applies (low clouds).
    pub freeze_dry_q: f64,
    pub freeze_dry_min: f64,
    pub freeze_dry_sigma: f64,
    /// Diagnostic split of the cloud radiative effect into stratiform and convective parts.
    pub cre_split: bool,
    /// With `cre_split`: σ at and below which the extra call drops the stratiform cloud
    /// instead of the convective cloud (0 = off, the convective split).
    pub cre_split_low_sigma: f64,
    /// Convective cloud geometry.
    pub convective: ConvectiveCloud,
    /// "anvil": share of C_c in the full-depth core, σ of the top above which convection is
    /// deep, σ at and below which the shallow remainder sits, anvil in-cloud water content
    /// \[kg m⁻³\], cap of the convective in-cloud water path \[kg m⁻²\].
    pub core_share: f64,
    pub deep_sigma: f64,
    pub shallow_sigma: f64,
    pub anvil_water: f64,
    pub conv_water_path: f64,
    /// Low cloud: the RH scheme of P4 or Slingo (1987) eqs. (5)–(9).
    pub low_cloud: LowCloud,
    /// "slingo": σ at and below which eqs. (6)–(7) apply; the RH law there; its threshold;
    /// ω \[Pa s⁻¹\] of full ascent in eq. (7).
    pub low_sigma: f64,
    pub low_rh: LowRh,
    pub low_rh_crit: f64,
    pub low_omega: f64,
    /// "slingo": inversion search below this pressure \[Pa\]; eq. (8) slope \[hPa K⁻¹\] and
    /// offset.
    pub inv_pressure: f64,
    pub inv_slope: f64,
    pub inv_offset: f64,
    /// "slingo_lts": slope \[K⁻¹\] and intercept of the stratus law (Klein & Hartmann 1993
    /// as coded in CAM).
    pub lts_slope: f64,
    pub lts_intercept: f64,
}

/// Low-cloud treatment of [`DiagnosticClouds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowCloud {
    /// The Sundqvist RH cloud at every level (the P4 scheme).
    Rh,
    /// Slingo (1987): RH cloud suppressed under subsidence, plus inversion cloud.
    Slingo,
    /// Slingo's suppressed RH cloud plus the lower-tropospheric-stability stratus law
    /// (Klein & Hartmann 1993) in place of his inversion cloud.
    SlingoLts,
}

/// RH law of the Slingo low cloud C_L'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowRh {
    /// Slingo's eq. (6) with RH_e (eq. 5).
    Slingo,
    /// This scheme's Sundqvist fraction with RH_c(σ).
    Sundqvist,
}

/// Convective cloud geometry of [`DiagnosticClouds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvectiveCloud {
    /// C_c over the whole convecting depth with β q_sat in-cloud water (the P4 scheme).
    Column,
    /// Slingo (1987): core + low-level shallow part + anvil at the top, capped water path.
    Anvil,
}

impl DiagnosticClouds {
    pub fn new(cond: Condensable) -> Self {
        DiagnosticClouds {
            cond,
            rh_crit_surface: 0.95,
            rh_crit_upper: 0.9,
            rh_crit_exponent: 4.0,
            conv_coef: 0.125,
            conv_max: 0.8,
            water_ratio: 0.03,
            water_max: 5e-4,
            sigma_min: 0.1,
            freeze_dry_q: 3e-3,
            freeze_dry_min: 0.15,
            freeze_dry_sigma: 0.75,
            cre_split: false,
            cre_split_low_sigma: 0.0,
            convective: ConvectiveCloud::Column,
            core_share: 0.25,
            deep_sigma: 0.4,
            shallow_sigma: 0.8,
            anvil_water: 3.5e-5,
            conv_water_path: 1.0,
            low_cloud: LowCloud::Rh,
            low_sigma: 0.8,
            low_rh: LowRh::Slingo,
            low_rh_crit: 0.8,
            low_omega: 0.1,
            inv_pressure: 75000.0,
            inv_slope: 6.67,
            inv_offset: 0.667,
            lts_slope: 0.057,
            lts_intercept: -0.5573,
        }
    }

    /// Critical relative humidity at σ.
    pub fn rh_crit(&self, sigma: f64) -> f64 {
        let x = (1.0 - sigma.max(1e-6).powf(-self.rh_crit_exponent)).exp();
        self.rh_crit_upper + (self.rh_crit_surface - self.rh_crit_upper) * x
    }

    /// Stratiform fraction for relative humidity `rh` with critical value `rc`.
    pub fn stratiform(rh: f64, rc: f64) -> f64 {
        if rh <= rc {
            0.0
        } else if rh >= 1.0 {
            1.0
        } else {
            1.0 - ((1.0 - rh) / (1.0 - rc)).sqrt()
        }
    }

    /// Freeze-dry factor for specific humidity `q` at σ (1 when off or above σ_fd).
    pub fn freeze_dry(&self, q: f64, sigma: f64) -> f64 {
        if self.freeze_dry_q <= 0.0 || sigma < self.freeze_dry_sigma {
            1.0
        } else {
            (q / self.freeze_dry_q).clamp(self.freeze_dry_min, 1.0)
        }
    }

    /// Convective fraction for precipitation `p` \[kg m⁻² s⁻¹\].
    pub fn convective(&self, p: f64) -> f64 {
        if p <= 0.0 {
            return 0.0;
        }
        (self.conv_coef * (1.0 + p * 86400.0).ln()).min(self.conv_max)
    }

    /// Fourth-block fraction of a layer at `sigma` with stratiform fraction `cs`, convective
    /// fraction `cv` and total `c`: the stratiform-only cover, or with `cre_split_low_sigma`
    /// the cover without stratiform cloud at and below that σ.
    fn split_fraction(&self, sigma: f64, cs: f64, cv: f64, c: f64) -> f64 {
        if self.cre_split_low_sigma <= 0.0 {
            cs
        } else if sigma >= self.cre_split_low_sigma {
            cv
        } else {
            c
        }
    }
}

impl ColumnScheme for DiagnosticClouds {
    fn name(&self) -> &'static str {
        "diagnostic_clouds"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        match (self.cre_split, self.low_cloud) {
            (false, LowCloud::Rh) => &["clwvi"],
            (true, LowCloud::Rh) => &["clwvi", "cc_conv", "cwp_conv"],
            (false, LowCloud::Slingo) => &["clwvi", "cl_inv", "inv_dthdp"],
            (true, LowCloud::Slingo) => &["clwvi", "cc_conv", "cwp_conv", "cl_inv", "inv_dthdp"],
            (false, LowCloud::SlingoLts) => &["clwvi", "cl_lts", "lts"],
            (true, LowCloud::SlingoLts) => &["clwvi", "cc_conv", "cwp_conv", "cl_lts", "lts"],
        }
    }
    fn needs_omega(&self) -> bool {
        self.low_cloud != LowCloud::Rh
    }
    fn memory(&self, nlev: usize) -> usize {
        if self.cre_split {
            4 * nlev
        } else {
            3 * nlev
        }
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        _work: &mut Workspace,
    ) {
        let l = st.nlev;
        let m = &mut sfc.cloud;
        m.clear();
        m.resize(self.memory(l), 0.0);
        let Some(q) = st.humidity() else {
            return;
        };
        if self.convective == ConvectiveCloud::Anvil {
            self.apply_anvil(ctx, st, q, &sfc.tiles, m, diag);
            return;
        }
        let (planet, g) = (ctx.planet, ctx.planet.gravity);
        let cc = self.convective(st.conv_precip);
        let inv = self.column_inversion(ctx, st, q, &sfc.tiles);
        let (mut path, mut conv_path) = (0.0, 0.0);
        for k in 0..l {
            if ctx.sigma[k] < self.sigma_min {
                continue;
            }
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            let cv = if k >= st.conv_top && k + 1 < l {
                cc
            } else {
                0.0
            };
            let cs =
                self.layer_cloud(ctx, qs, q[k], k, cv, inv) * self.freeze_dry(q[k], ctx.sigma[k]);
            let c = 1.0 - (1.0 - cs) * (1.0 - cv);
            if c <= 0.0 {
                continue;
            }
            let w = (self.water_ratio * qs).min(self.water_max) * ctx.dp[k] / g;
            if self.cre_split {
                m[3 * l + k] = self.split_fraction(ctx.sigma[k], cs, cv, c);
                if cv > 0.0 {
                    conv_path += w;
                }
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lts;
