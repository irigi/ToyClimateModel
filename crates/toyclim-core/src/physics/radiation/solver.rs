//! Multi-band two-stream solver (SPEC.md §9.4.1), independent of where the optical
//! properties come from (see [`super::Opacity`]).
//!
//! **Shortwave**, per band:
//! - δ-scaling of each layer's optical depth τ, single-scattering albedo ω and asymmetry g
//!   (Joseph, Wiscombe & Weinman 1976): f = g², τ' = (1 − ωf)τ, ω' = (1 − f)ω / (1 − ωf),
//!   g' = (g − f)/(1 − f).
//! - Two-stream coefficients of the "practical improved flux method" (PIFM, Zdunkowski,
//!   Welch & Korb 1980), the default of RRTMG_SW (`reftra_sw`, `kmodts = 2`):
//!   γ₁ = (8 − ω(5 + 3g))/4, γ₂ = 3ω(1 − g)/4, γ₃ = (2 − 3gμ₀)/4, γ₄ = 1 − γ₃. Unlike the
//!   Eddington coefficients (γ₂ < 0 for ω < 1/4), these give no negative reflectance.
//! - Homogeneous layer, written here from the two-stream equations
//!   dF⁺/dτ = γ₁F⁺ − γ₂F⁻ − γ₃ω e^{−τ/μ₀}/μ₀, dF⁻/dτ = γ₂F⁺ − γ₁F⁻ + γ₄ω e^{−τ/μ₀}/μ₀
//!   with the modes (ρ, 1)e^{−kτ} and (1, ρ)e^{−k(τ_l − τ)}, k = √(γ₁² − γ₂²),
//!   ρ = γ₂/(γ₁ + k) and the particular solution Z^± e^{−τ/μ₀}; only decaying exponentials
//!   appear, so it is stable for any τ (checked against doubling from thin layers in the
//!   tests).
//! - Layers and the surface (diffuse and direct albedo per band) combined by the adding
//!   method; net fluxes at all half levels.
//!
//! **Longwave**, per band: absorption and emission without scattering, diffusivity factor
//! D = 1.66 (the flux transmission of a layer is e^{−Dτ}), isothermal layers integrated
//! exactly (as in the gray scheme), surface emissivity 1. Each band's source is its Planck
//! fraction of σT⁴.
//!
//! **Clouds:** two-column approximation. Total cover C by maximum-random overlap
//! (Geleyn & Hollingsworth 1979); a clear solve and a cloudy solve in which each layer adds
//! the in-cloud optical properties scaled by c_k / C; fluxes (1 − C) F_clear + C F_cloudy.
//!
//! Index 0 is the top layer; half level k is the top of layer k, half level L the surface.

/// Longwave diffusivity factor.
pub const DIFFUSIVITY: f64 = 1.66;

/// Optical properties of one column, `[band * nlev + k]`.
#[derive(Debug, Clone, Default)]
pub struct LayerOptics {
    pub nlev: usize,
    pub nsw: usize,
    pub nlw: usize,
    /// Shortwave extinction optical depth, single-scattering albedo, asymmetry factor.
    pub sw_tau: Vec<f64>,
    pub sw_ssa: Vec<f64>,
    pub sw_g: Vec<f64>,
    /// Longwave absorption optical depth (vertical; the solver applies the diffusivity).
    pub lw_tau: Vec<f64>,
}

impl LayerOptics {
    pub fn new(nlev: usize, nsw: usize, nlw: usize) -> Self {
        let mut o = LayerOptics::default();
        o.resize(nlev, nsw, nlw);
        o
    }
    /// Set the shape and zero everything.
    pub fn resize(&mut self, nlev: usize, nsw: usize, nlw: usize) {
        self.nlev = nlev;
        self.nsw = nsw;
        self.nlw = nlw;
        for (v, n) in [
            (&mut self.sw_tau, nsw),
            (&mut self.sw_ssa, nsw),
            (&mut self.sw_g, nsw),
            (&mut self.lw_tau, nlw),
        ] {
            v.clear();
            v.resize(n * nlev, 0.0);
        }
    }
}

/// Cloud fraction per layer and the in-cloud optical properties (same layout as
/// [`LayerOptics`]).
#[derive(Debug, Clone, Default)]
pub struct CloudOptics {
    pub frac: Vec<f64>,
    pub optics: LayerOptics,
}

/// Boundary conditions and the result of one column solve. Fluxes \[W m⁻²\].
#[derive(Debug, Clone, Default)]
pub struct Fluxes {
    /// Net downward shortwave flux at the L + 1 half levels.
    pub sw_net: Vec<f64>,
    /// Shortwave reflected to space; downward shortwave at the surface per band.
    pub sw_up_top: f64,
    pub sw_down_sfc: Vec<f64>,
    /// Longwave upward and downward fluxes at the half levels.
    pub lw_up: Vec<f64>,
    pub lw_down: Vec<f64>,
    /// Total cloud cover used.
    pub cloud_cover: f64,
}

/// Inputs of a column solve besides the optics.
pub struct Boundary<'a> {
    /// Stellar flux on a horizontal surface at the top \[W m⁻²\] and cos zenith.
    pub s0: f64,
    pub mu0: f64,
    /// Fraction of the stellar flux in each shortwave band (sums to 1).
    pub sw_weights: &'a [f64],
    /// Surface albedo per shortwave band (direct = diffuse).
    pub albedo: &'a [f64],
    /// Layer Planck source per longwave band, `[band * nlev + k]` \[W m⁻²\].
    pub lw_source: &'a [f64],
    /// Surface emission per longwave band \[W m⁻²\].
    pub lw_sfc: &'a [f64],
}

/// One homogeneous layer: diffuse reflectance and transmittance, direct-beam reflectance,
/// diffuse transmittance of the direct beam and direct transmittance.
#[derive(Debug, Clone, Copy)]
pub struct Layer {
    pub r_dif: f64,
    pub t_dif: f64,
    pub r_dir: f64,
    pub t_dir: f64,
    pub trn: f64,
}

/// δ-scaled PIFM layer of optical depth `tau`, single-scattering albedo `ssa`, asymmetry
/// `g`, at cos zenith `mu0`.
pub fn layer(tau: f64, ssa: f64, g: f64, mu0: f64) -> Layer {
    // δ-scaling
    let f = g * g;
    let w0 = ssa.clamp(0.0, 1.0 - 1e-9);
    let tau = (1.0 - w0 * f) * tau;
    let w = (1.0 - f) * w0 / (1.0 - w0 * f);
    let g = if f < 1.0 { (g - f) / (1.0 - f) } else { 0.0 };
    let w = w.min(1.0 - 1e-9);
    let g1 = (8.0 - w * (5.0 + 3.0 * g)) * 0.25;
    let g2 = 3.0 * w * (1.0 - g) * 0.25;
    let k = ((g1 - g2) * (g1 + g2)).max(0.0).sqrt();
    let rho = g2 / (g1 + k);
    let e = (-k * tau).exp();
    let den = 1.0 - rho * rho * e * e;
    let r_dif = rho * (1.0 - e * e) / den;
    let t_dif = e * (1.0 - rho * rho) / den;
    // direct beam; move μ₀ off the resonance kμ₀ = 1
    let mut mu = mu0;
    if ((k * mu).powi(2) - 1.0).abs() < 1e-6 {
        mu *= 1.0 + 2e-6;
    }
    let trn = (-tau / mu).exp();
    let g3 = (2.0 - 3.0 * g * mu) * 0.25;
    let g4 = 1.0 - g3;
    let im = 1.0 / mu;
    let det = (g1 + im) * (g1 - im) - g2 * g2;
    let zp = (g3 * w * im * (g1 - im) + g2 * g4 * w * im) / det;
    let zm = ((g1 + im) * g4 * w * im + g2 * g3 * w * im) / det;
    let a1 = (rho * e * zp * trn - zm) / den;
    let r_dir = a1 * rho * (1.0 - e * e) + zp * (1.0 - trn * e);
    let t_dir = a1 * e * (1.0 - rho * rho) + trn * (zm - rho * zp);
    Layer {
        r_dif,
        t_dif,
        r_dir: r_dir.max(0.0),
        t_dir: t_dir.max(0.0),
        trn,
    }
}

/// Shortwave of one band for unit flux at the top: adds the net downward flux at the
/// half levels to `net` (scaled by `scale`); returns (reflected at the top, downward at
/// the surface), both for unit incident flux. `lay` and the scratch vectors have L
/// entries (L + 1 for the half-level ones).
fn sw_band(
    lay: &[Layer],
    albedo: f64,
    scale: f64,
    net: &mut [f64],
    s: &mut SwScratch,
) -> (f64, f64) {
    let l = lay.len();
    s.resize(l);
    // top-down: direct beam, total transmission of the beam, reflectivity of the stack
    // above for diffuse light from below
    s.tb[0] = 1.0;
    s.tt[0] = 1.0;
    s.ra[0] = 0.0;
    for k in 0..l {
        let y = &lay[k];
        let d = 1.0 / (1.0 - y.r_dif * s.ra[k]);
        let x = s.tt[k] - s.tb[k];
        s.ra[k + 1] = y.r_dif + y.t_dif * y.t_dif * s.ra[k] * d;
        s.tb[k + 1] = s.tb[k] * y.trn;
        s.tt[k + 1] = s.tb[k] * (y.trn + y.t_dir) + (x + s.tb[k] * y.r_dir * s.ra[k]) * y.t_dif * d;
    }
    // bottom-up: reflectivity of the stack below for diffuse light and for the beam
    s.rb[l] = albedo;
    s.rbd[l] = albedo;
    for k in (0..l).rev() {
        let y = &lay[k];
        let d = 1.0 / (1.0 - y.r_dif * s.rb[k + 1]);
        s.rb[k] = y.r_dif + y.t_dif * y.t_dif * s.rb[k + 1] * d;
        s.rbd[k] = y.r_dir + (y.trn * s.rbd[k + 1] + y.t_dir * s.rb[k + 1]) * y.t_dif * d;
    }
    let mut top = 0.0;
    let mut sfc_down = 0.0;
    for k in 0..=l {
        let x = s.tt[k] - s.tb[k];
        let d = 1.0 / (1.0 - s.ra[k] * s.rb[k]);
        let up = (s.tb[k] * s.rbd[k] + x * s.rb[k]) * d;
        let dn = (x + s.tb[k] * s.rbd[k] * s.ra[k]) * d;
        net[k] += scale * (s.tb[k] + dn - up);
        if k == 0 {
            top = up;
        }
        if k == l {
            sfc_down = s.tb[k] + dn;
        }
    }
    (top, sfc_down)
}

/// Scratch vectors of the solver.
#[derive(Debug, Clone, Default)]
pub struct SwScratch {
    tb: Vec<f64>,
    tt: Vec<f64>,
    ra: Vec<f64>,
    rb: Vec<f64>,
    rbd: Vec<f64>,
    lay: Vec<Layer>,
    tau: Vec<f64>,
    ssa: Vec<f64>,
    g: Vec<f64>,
    trs: Vec<f64>,
}

impl SwScratch {
    fn resize(&mut self, l: usize) {
        for v in [
            &mut self.tb,
            &mut self.tt,
            &mut self.ra,
            &mut self.rb,
            &mut self.rbd,
        ] {
            v.resize(l + 1, 0.0);
        }
    }
}

/// Total cloud cover by maximum-random overlap.
pub fn cloud_cover(c: &[f64]) -> f64 {
    let mut clear = 1.0;
    let mut prev: f64 = 0.0;
    for &ck in c {
        let ck = ck.clamp(0.0, 1.0);
        if prev < 1.0 - 1e-12 {
            clear *= (1.0 - prev.max(ck)) / (1.0 - prev);
        } else {
            clear = 0.0;
        }
        prev = ck;
    }
    (1.0 - clear).clamp(0.0, 1.0)
}

/// Solve one column: gas optics `gas`, optional clouds, boundary `b`. Fills `out`.
pub fn solve(
    gas: &LayerOptics,
    clouds: Option<&CloudOptics>,
    b: &Boundary,
    out: &mut Fluxes,
    s: &mut SwScratch,
) {
    let l = gas.nlev;
    out.sw_net.clear();
    out.sw_net.resize(l + 1, 0.0);
    out.lw_up.clear();
    out.lw_up.resize(l + 1, 0.0);
    out.lw_down.clear();
    out.lw_down.resize(l + 1, 0.0);
    out.sw_down_sfc.clear();
    out.sw_down_sfc.resize(gas.nsw, 0.0);
    out.sw_up_top = 0.0;
    let cc = clouds.map_or(0.0, |c| cloud_cover(&c.frac));
    out.cloud_cover = cc;
    let columns: &[(f64, bool)] = if cc > 0.0 {
        &[(1.0 - cc, false), (cc, true)]
    } else {
        &[(1.0, false)]
    };
    for &(wcol, cloudy) in columns {
        if wcol <= 0.0 {
            continue;
        }
        let cl = if cloudy { clouds } else { None };
        // shortwave
        if b.s0 > 0.0 && b.mu0 > 0.0 {
            for band in 0..gas.nsw {
                let scale = wcol * b.s0 * b.sw_weights[band];
                if scale == 0.0 {
                    continue;
                }
                s.tau.clear();
                s.ssa.clear();
                s.g.clear();
                for k in 0..l {
                    let i = band * l + k;
                    let (mut t, mut ws, mut wsg) = (
                        gas.sw_tau[i],
                        gas.sw_tau[i] * gas.sw_ssa[i],
                        gas.sw_tau[i] * gas.sw_ssa[i] * gas.sw_g[i],
                    );
                    if let Some(c) = cl {
                        let x = c.frac[k] / cc;
                        let o = &c.optics;
                        let tc = x * o.sw_tau[i];
                        t += tc;
                        ws += tc * o.sw_ssa[i];
                        wsg += tc * o.sw_ssa[i] * o.sw_g[i];
                    }
                    s.tau.push(t);
                    s.ssa.push(if t > 0.0 { ws / t } else { 0.0 });
                    s.g.push(if ws > 0.0 { wsg / ws } else { 0.0 });
                }
                let mut lay = std::mem::take(&mut s.lay);
                lay.clear();
                lay.extend((0..l).map(|k| layer(s.tau[k], s.ssa[k], s.g[k], b.mu0)));
                let (top, dn) = sw_band(&lay, b.albedo[band], scale, &mut out.sw_net, s);
                s.lay = lay;
                out.sw_up_top += scale * top;
                out.sw_down_sfc[band] += scale * dn;
            }
        }
        // longwave
        for band in 0..gas.nlw {
            let tr = |k: usize| {
                let i = band * l + k;
                let mut t = gas.lw_tau[i];
                if let Some(c) = cl {
                    t += c.frac[k] / cc * c.optics.lw_tau[i];
                }
                (-DIFFUSIVITY * t).exp()
            };
            let src = &b.lw_source[band * l..(band + 1) * l];
            let mut dn = 0.0;
            s.trs.clear();
            for k in 0..l {
                let t = tr(k);
                s.trs.push(t);
                dn = dn * t + src[k] * (1.0 - t);
                out.lw_down[k + 1] += wcol * dn;
            }
            let mut up = b.lw_sfc[band];
            out.lw_up[l] += wcol * up;
            for k in (0..l).rev() {
                let t = s.trs[k];
                up = up * t + src[k] * (1.0 - t);
                out.lw_up[k] += wcol * up;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thin layer of the same two-stream equations, combined by doubling: an independent
    /// check of the closed form.
    fn doubling(tau: f64, ssa: f64, g: f64, mu0: f64) -> Layer {
        // δ-scaled inputs as in `layer`
        let f = g * g;
        let tau = (1.0 - ssa * f) * tau;
        let w = (1.0 - f) * ssa / (1.0 - ssa * f);
        let g = (g - f) / (1.0 - f);
        let g1 = (8.0 - w * (5.0 + 3.0 * g)) * 0.25;
        let g2 = 3.0 * w * (1.0 - g) * 0.25;
        let g3 = (2.0 - 3.0 * g * mu0) * 0.25;
        let g4 = 1.0 - g3;
        let n = 30;
        let dt = tau / 2f64.powi(n);
        let mut y = Layer {
            r_dif: g2 * dt,
            t_dif: 1.0 - g1 * dt,
            r_dir: g3 * w * dt / mu0,
            t_dir: g4 * w * dt / mu0,
            trn: (-dt / mu0).exp(),
        };
        for _ in 0..n {
            let d = 1.0 / (1.0 - y.r_dif * y.r_dif);
            let r_dir = y.r_dir + (y.trn * y.r_dir + y.t_dir * y.r_dif) * y.t_dif * d;
            let t_dir = y.trn * y.t_dir + (y.t_dir + y.trn * y.r_dir * y.r_dif) * y.t_dif * d;
            y = Layer {
                r_dif: y.r_dif + y.t_dif * y.t_dif * y.r_dif * d,
                t_dif: y.t_dif * y.t_dif * d,
                r_dir,
                t_dir,
                trn: y.trn * y.trn,
            };
        }
        y
    }

    #[test]
    fn closed_form_matches_doubling() {
        for &(tau, w, g, mu) in &[
            (0.1, 0.0, 0.0, 0.5),
            (1.0, 0.5, 0.3, 0.7),
            (3.0, 0.99, 0.85, 0.3),
            (0.5, 0.9999, 0.0, 1.0),
            (10.0, 0.8, 0.6, 0.2),
        ] {
            let a = layer(tau, w, g, mu);
            let b = doubling(tau, w, g, mu);
            for (x, y) in [
                (a.r_dif, b.r_dif),
                (a.t_dif, b.t_dif),
                (a.r_dir, b.r_dir),
                (a.t_dir, b.t_dir),
                (a.trn, b.trn),
            ] {
                assert!((x - y).abs() < 1e-6, "{tau} {w} {g} {mu}: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn layer_limits() {
        // pure absorber: no reflection, direct beam e^{-τ/μ₀}
        let a = layer(0.7, 0.0, 0.0, 0.4);
        assert!(a.r_dif.abs() < 1e-15 && a.r_dir.abs() < 1e-15 && a.t_dir.abs() < 1e-15);
        assert!((a.trn - (-1.75f64).exp()).abs() < 1e-15);
        // conservative scattering: nothing absorbed
        for mu in [0.2, 0.5, 1.0] {
            let c = layer(2.0, 1.0, 0.5, mu);
            assert!((c.r_dif + c.t_dif - 1.0).abs() < 1e-6, "{c:?}");
            assert!((c.r_dir + c.t_dir + c.trn - 1.0).abs() < 1e-6, "{c:?}");
        }
        // resonance kμ₀ = 1 is finite
        // ω = 0, g = 0: γ₁ = 2, γ₂ = 0, k = 2 → μ₀ = 0.5
        let r = layer(1.0, 0.0, 0.0, 0.5);
        assert!(r.trn.is_finite() && r.t_dir.is_finite());
    }

    #[test]
    fn overlap() {
        assert_eq!(cloud_cover(&[0.0, 0.0]), 0.0);
        assert!((cloud_cover(&[0.5, 0.5]) - 0.5).abs() < 1e-12); // maximum overlap
        assert!((cloud_cover(&[0.5, 0.0, 0.5]) - 0.75).abs() < 1e-12); // random
        assert_eq!(cloud_cover(&[0.3, 1.0, 0.2]), 1.0);
    }
}
