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
}

/// Convective cloud geometry of [`DiagnosticClouds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvectiveCloud {
    /// C_c over the whole convecting depth with β q_sat in-cloud water (the P4 scheme).
    Column,
    /// Slingo (1987): core + low-level shallow part + anvil at the top, capped water path.
    Anvil,
}

/// Slingo (1987) eq. (1) constants from his Table 1 (P in mm day⁻¹).
const SLINGO_A: f64 = 0.2454;
const SLINGO_B: f64 = 0.1248;

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
        if self.cre_split {
            &["clwvi", "cc_conv", "cwp_conv"]
        } else {
            &["clwvi"]
        }
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
            self.apply_anvil(ctx, st, q, m, diag);
            return;
        }
        let (planet, g) = (ctx.planet, ctx.planet.gravity);
        let cc = self.convective(st.conv_precip);
        let (mut path, mut conv_path) = (0.0, 0.0);
        for k in 0..l {
            if ctx.sigma[k] < self.sigma_min {
                continue;
            }
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            let cs = Self::stratiform(q[k] / qs, self.rh_crit(ctx.sigma[k]))
                * self.freeze_dry(q[k], ctx.sigma[k]);
            let cv = if k >= st.conv_top && k + 1 < l {
                cc
            } else {
                0.0
            };
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
    }
}

impl DiagnosticClouds {
    /// The "anvil" geometry (see the module documentation).
    fn apply_anvil(
        &self,
        ctx: &ColumnCtx,
        st: &ColumnState,
        q: &[f64],
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
        let (mut path, mut conv_path) = (0.0, 0.0);
        for k in 0..l {
            if ctx.sigma[k] < self.sigma_min {
                continue;
            }
            let qs = self.cond.qsat(planet, st.t[k], ctx.p[k]);
            let cs = Self::stratiform(q[k] / qs, self.rh_crit(ctx.sigma[k]))
                * self.freeze_dry(q[k], ctx.sigma[k]);
            let ws = raw(k);
            let mut cv = conv_frac(k);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::column::testing::Geometry;
    use crate::physics::radiation::solver::cloud_cover;

    fn scheme() -> DiagnosticClouds {
        DiagnosticClouds::new(Condensable::water())
    }

    #[test]
    fn fraction_limits_and_profile() {
        assert_eq!(DiagnosticClouds::stratiform(0.5, 0.8), 0.0);
        assert_eq!(DiagnosticClouds::stratiform(1.01, 0.8), 1.0);
        let c = DiagnosticClouds::stratiform(0.95, 0.8);
        assert!((c - (1.0 - 0.25f64.sqrt())).abs() < 1e-12);
        let s = scheme();
        assert!((s.rh_crit(1.0) - 0.95).abs() < 1e-12);
        assert!((s.rh_crit(0.4) - 0.9).abs() < 1e-6);
        assert_eq!(s.convective(0.0), 0.0);
        let ten = 10.0 / 86400.0;
        assert!((s.convective(ten) - 0.125 * 11f64.ln()).abs() < 1e-12);
        assert_eq!(s.convective(1.0), 0.8);
        // freeze-dry: factor q / 3 g/kg between 0.15 and 1, low levels only; off with q_fd = 0
        let off = DiagnosticClouds {
            freeze_dry_q: 0.0,
            ..scheme()
        };
        assert_eq!(off.freeze_dry(1e-4, 0.95), 1.0);
        let f = scheme();
        assert!((f.freeze_dry(1.5e-3, 0.9) - 0.5).abs() < 1e-12);
        assert_eq!(f.freeze_dry(1e-4, 0.9), 0.15);
        assert_eq!(f.freeze_dry(5e-3, 0.9), 1.0);
        assert_eq!(f.freeze_dry(1e-4, 0.5), 1.0);
    }

    /// Freeze-dry in a cold, dry saturated column: low cloud reduced, still no tendencies,
    /// fractions in [0, 1]; a warm moist column is unchanged.
    #[test]
    fn freeze_dry_column() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let run = |s: &DiagnosticClouds, t_sfc: f64| {
            let mut st = ColumnState::new(26, 1);
            st.begin(1200.0);
            for k in 0..26 {
                st.t[k] = t_sfc - 50.0 * (1.0 - ctx.sigma[k]);
                st.q[k] = 0.97 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
            }
            let q0 = st.q.clone();
            let mut sfc = SurfaceExchange::default();
            s.apply(&ctx, &mut st, &mut sfc, &mut [0.0], &mut Workspace::new(26));
            assert_eq!(st.q, q0);
            assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
            assert!(sfc.cloud[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
            sfc.cloud
        };
        let off = DiagnosticClouds {
            freeze_dry_q: 0.0,
            ..scheme()
        };
        let on = scheme();
        let (a, b) = (run(&off, 245.0), run(&on, 245.0));
        assert!(b[25] < 0.5 * a[25] && b[25] > 0.0, "{} {}", a[25], b[25]);
        assert_eq!(a[5], b[5]); // aloft unchanged
        assert_eq!(run(&off, 300.0), run(&on, 300.0));
    }

    /// CRE split: same fraction and water blocks as without it, a fourth block with the
    /// stratiform-only fraction (≤ the total), and the convective fraction and in-cloud path.
    #[test]
    fn cre_split_memory() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let run = |s: &DiagnosticClouds| {
            let mut st = ColumnState::new(26, 1);
            st.begin(1200.0);
            for k in 0..26 {
                st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
                st.q[k] = 0.93 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
            }
            st.conv_precip = 20.0 / 86400.0;
            st.conv_top = 8;
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 3];
            s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
            assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
            (sfc.cloud, diag)
        };
        let (a, da) = run(&scheme());
        let on = DiagnosticClouds {
            cre_split: true,
            ..scheme()
        };
        let (b, db) = run(&on);
        assert_eq!(a.len(), 78);
        assert_eq!(b.len(), 104);
        assert_eq!(&a[..], &b[..78]);
        assert_eq!(da[0], db[0]);
        let cc = on.convective(20.0 / 86400.0);
        assert!((db[1] - cc).abs() < 1e-12);
        assert!(db[2] > 0.0);
        for k in 0..26 {
            assert!(b[78 + k] <= b[k] + 1e-15);
            if (8..25).contains(&k) && b[k] > 0.0 {
                let expect = 1.0 - (1.0 - b[78 + k]) * (1.0 - cc);
                assert!((b[k] - expect).abs() < 1e-12, "{k}");
            }
        }
        // without convection the stratiform-only block equals the total
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.93 * on.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        let mut sfc = SurfaceExchange::default();
        let mut d = [0.0; 3];
        on.apply(&ctx, &mut st, &mut sfc, &mut d, &mut Workspace::new(26));
        assert_eq!(&sfc.cloud[..26], &sfc.cloud[78..104]);
        assert_eq!(d[1], 0.0);
    }

    /// CRE split without low stratiform cloud (`cre_split_low_sigma`): the first three
    /// blocks and the diagnostics are unchanged; the fourth block equals the total above σ_low
    /// and the convective fraction alone at and below it (zero where there is no convection).
    #[test]
    fn cre_split_low_memory() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let run = |s: &DiagnosticClouds, conv: bool| {
            let mut st = ColumnState::new(26, 1);
            st.begin(1200.0);
            for k in 0..26 {
                st.t[k] = 295.0 - 70.0 * (1.0 - ctx.sigma[k]);
                st.q[k] = 0.97 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
            }
            if conv {
                st.conv_precip = 20.0 / 86400.0;
                st.conv_top = 8;
            }
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 3];
            s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
            assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
            (sfc.cloud, diag)
        };
        for convective in [ConvectiveCloud::Column, ConvectiveCloud::Anvil] {
            let conv = DiagnosticClouds {
                cre_split: true,
                convective,
                ..scheme()
            };
            let low = DiagnosticClouds {
                cre_split_low_sigma: 0.7,
                ..conv.clone()
            };
            for with_conv in [false, true] {
                let (a, da) = run(&conv, with_conv);
                let (b, db) = run(&low, with_conv);
                assert_eq!(&a[..78], &b[..78]);
                assert_eq!(da, db);
                let mut dropped = false;
                for k in 0..26 {
                    let (c, cs, x) = (b[k], a[78 + k], b[78 + k]);
                    if ctx.sigma[k] < 0.7 {
                        assert_eq!(x, c, "{k}");
                    } else {
                        // the convective cover of the layer: c = 1 − (1 − cs)(1 − cv)
                        let cv = if cs < 1.0 {
                            1.0 - (1.0 - c) / (1.0 - cs)
                        } else {
                            x
                        };
                        assert!((x - cv).abs() < 1e-12, "{k}: {x} {cv}");
                        assert!(x <= c + 1e-15);
                        if !with_conv {
                            assert_eq!(x, 0.0);
                        }
                        dropped |= cs > 0.0;
                    }
                }
                assert!(dropped, "the test column must have low stratiform cloud");
            }
        }
    }

    /// The scheme leaves T and q alone, stores fractions in [0, 1] and positive paths, and
    /// a saturated layer is overcast.
    #[test]
    fn column_memory_and_no_tendencies() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let s = scheme();
        let mut st = ColumnState::new(26, 1);
        st.begin(1200.0);
        for k in 0..26 {
            st.t[k] = 290.0 - 60.0 * (1.0 - ctx.sigma[k]);
            st.q[k] = 0.5 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
        }
        st.q[20] = 1.05 * s.cond.qsat(ctx.planet, st.t[20], ctx.p[20]);
        st.conv_precip = 20.0 / 86400.0;
        st.conv_top = 10;
        let (t0, q0) = (st.t.clone(), st.q.clone());
        let mut sfc = SurfaceExchange::default();
        let mut diag = [0.0; 1];
        s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
        assert_eq!(st.t, t0);
        assert_eq!(st.q, q0);
        assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
        let m = &sfc.cloud;
        assert_eq!(m.len(), 78);
        assert_eq!(m[20], 1.0);
        let cv = s.convective(st.conv_precip);
        assert!((m[12] - cv).abs() < 1e-12, "{}", m[12]);
        assert_eq!(m[5], 0.0); // above the convection, dry
        assert_eq!(m[25], 0.0); // lowest level: no convective cloud
        assert!(m.iter().all(|&x| x >= 0.0));
        assert!(m[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
        assert!(cloud_cover(&m[..26]) >= 1.0 - 1e-12 && diag[0] > 0.0);
        // warm layer liquid, cold layer ice
        assert!(m[26 + 20] > 0.0);
    }
    /// Slingo (1987) anvil cover: zero up to his C_c = 0.4 (P = 3.4 mm/day), 2 (C_c − 0.3)
    /// above, 1 at his cap (P ≥ 85 mm/day).
    #[test]
    fn anvil_cover_follows_slingo() {
        let mmd = 1.0 / 86400.0;
        assert_eq!(DiagnosticClouds::anvil(0.0), 0.0);
        assert_eq!(DiagnosticClouds::anvil(3.3 * mmd), 0.0);
        assert!(DiagnosticClouds::anvil(3.6 * mmd) > 0.2);
        // Table 1: C_c = 0.6 at 17 mm/day -> C_H = 0.6
        assert!((DiagnosticClouds::anvil(17.0 * mmd) - 0.6).abs() < 0.01);
        assert!((DiagnosticClouds::anvil(85.0 * mmd) - 1.0).abs() < 1e-3);
        assert_eq!(DiagnosticClouds::anvil(500.0 * mmd), 1.0);
    }

    /// "anvil" geometry in a dry column (no stratiform cloud): deep convection gives the core
    /// share over the depth, the shallow remainder at σ ≥ 0.8, an anvil in the top layer with
    /// thin cloud, the convective water path capped; shallow convection fills its depth with
    /// C_c and has no anvil; no tendencies; "column" is unchanged by the new knobs.
    #[test]
    fn anvil_geometry_column() {
        let geo = Geometry::l26();
        let planet = crate::planet::Planet::earth();
        let ctx = geo.ctx(&planet, None, 1200.0);
        let s = DiagnosticClouds {
            convective: ConvectiveCloud::Anvil,
            cre_split: true,
            conv_water_path: 0.2,
            ..scheme()
        };
        let p = 30.0 / 86400.0;
        let run = |s: &DiagnosticClouds, top: usize| {
            let mut st = ColumnState::new(26, 1);
            st.begin(1200.0);
            for k in 0..26 {
                st.t[k] = (300.0 - 80.0 * (1.0 - ctx.sigma[k])).max(200.0);
                st.q[k] = 0.3 * s.cond.qsat(ctx.planet, st.t[k], ctx.p[k]);
            }
            st.conv_precip = p;
            st.conv_top = top;
            let mut sfc = SurfaceExchange::default();
            let mut diag = [0.0; 3];
            s.apply(&ctx, &mut st, &mut sfc, &mut diag, &mut Workspace::new(26));
            assert!(st.dt.iter().chain(&st.dq).all(|&x| x == 0.0));
            (sfc.cloud, diag)
        };
        let cc = s.convective(p);
        let ca = DiagnosticClouds::anvil(p);
        assert!(ca > 0.5);
        // deep: top in the upper troposphere
        let top = (0..26).find(|&k| ctx.sigma[k] >= 0.2).unwrap();
        let (m, d) = run(&s, top);
        assert!(m[..26].iter().all(|&c| (0.0..=1.0).contains(&c)));
        assert!(m[78..].iter().all(|&c| c == 0.0)); // no stratiform cloud
        for k in 0..26 {
            let sg = ctx.sigma[k];
            if k < top || k == 25 {
                assert_eq!(m[k], 0.0, "{k}");
            } else if k == top {
                let c = 1.0 - (1.0 - 0.25 * cc) * (1.0 - ca);
                assert!((m[k] - c).abs() < 1e-12, "anvil layer {k}: {}", m[k]);
                // thin: in-cloud water close to 0.035 g/m³ of the layer, far below the core's
                let w = m[26 + k] + m[52 + k];
                assert!(w < 0.05, "anvil water {w}");
            } else if sg >= 0.8 {
                let c = 1.0 - (1.0 - 0.25 * cc) * (1.0 - 0.75 * cc);
                assert!((m[k] - c).abs() < 1e-12, "{k}");
            } else {
                assert!((m[k] - 0.25 * cc).abs() < 1e-12, "{k}");
            }
        }
        assert!((d[1] - cc).abs() < 1e-12);
        assert!((d[2] - 0.2).abs() < 1e-9, "capped convective path {}", d[2]);
        // shallow: top at σ ≈ 0.6, C_c over the depth, no anvil
        let top_s = (0..26).find(|&k| ctx.sigma[k] >= 0.6).unwrap();
        let (m2, _) = run(&s, top_s);
        for k in top_s..25 {
            assert!((m2[k] - cc).abs() < 1e-12, "{k}");
        }
        assert_eq!(m2[top_s - 1], 0.0);
        // "column" ignores the anvil knobs
        let col = DiagnosticClouds {
            conv_water_path: 0.2,
            core_share: 0.5,
            ..scheme()
        };
        assert_eq!(run(&col, top).0, run(&scheme(), top).0);
    }
}
