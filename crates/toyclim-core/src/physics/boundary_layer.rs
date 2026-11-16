//! Boundary layer and the coupled surface solve (SPEC.md §9.4.3).
//!
//! **K-profile** after Troen & Mahrt (1986) in the form of Frierson, Held & Zurita-Gotor
//! (2006, J. Atmos. Sci. 63, 2548–2566, eqs. 16–20; transcribed from the paper):
//! - boundary-layer height h where the bulk Richardson number relative to the lowest
//!   level, Ri(z) = g z [θ_v(z) − θ_v(z_a)] / θ_v(z_a) / |v(z)|² (eq. 16, θ_v replaced by the
//!   virtual dry static energy), first exceeds Ri_c = 1;
//! - K(z) = K_b(z) for z < f_b h, K(z) = K_b(f_b h) (z / f_b h) [1 − (z − f_b h)/((1 − f_b) h)]²
//!   for f_b h < z < h (eqs. 17–18), f_b = 0.1;
//! - K_b(z) = κ u* z for Ri_a < 0 and κ u* z / [1 + (Ri_a/Ri_c) ln(z/z₀) / (1 − Ri_a/Ri_c)]
//!   for Ri_a > 0 (eqs. 19–20; u_a √C = u* with the drag coefficient of
//!   [`super::surface_flux`], which uses Louis 1979 rather than the paper's eqs. 12–14);
//! - the same K for dry static energy, tracers and momentum; above h a background
//!   diffusivity (knob, default 0 as in the paper).
//!
//! **Implicit solve.** Backward Euler over Δt_p for s = c_p T + Φ (Φ frozen during the
//! step), every tracer, u and v, one tridiagonal system per quantity. The downward
//! elimination leaves the lowest level as x_L⁺ = A + B·J in the unknown surface flux J.
//! Each surface tile then solves its energy balance implicitly for T_s⁺,
//! C_s (T_s⁺ − T_s)/Δt_p = SW_net + LW_down − σT_s⁺⁴ − H − L E + Q, with σT⁴ and q_sat
//! linearised about T_s and H, E written with the lowest-level values from the
//! elimination, treating the cell as if the tile covered it. The area-mean fluxes are
//! back-substituted into the atmosphere: what the tiles lose the atmosphere gains, to
//! round-off. The extra emission 4σT_s³ΔT_s of the linearised surface goes to space
//! (`lw_up_change`). A prescribed surface temperature (infinite heat capacity) keeps
//! T_s and passes the residual into the ocean.
//!
//! **Momentum**: implicit surface stress τ = ρ C_D |V| v_L⁺; no Rayleigh friction.
//! **Frictional heating**: the kinetic energy that diffusion and surface drag remove from
//! each layer is returned as heat in that layer, so the column's total energy changes only
//! by the surface heat fluxes.

use super::column::{ColumnCtx, ColumnScheme, ColumnState, SurfaceExchange, Workspace};
use super::radiation::STEFAN_BOLTZMANN;
use super::surface_flux::{self as sf, ExchangeParams, KARMAN};
use crate::surface::MAX_TILES;

/// K-profile boundary layer with the coupled surface solve.
#[derive(Debug, Clone)]
pub struct KProfile {
    pub exchange: ExchangeParams,
    /// Critical Richardson number Ri_c (1).
    pub ri_crit: f64,
    /// Surface-layer fraction f_b (0.1).
    pub surface_layer_fraction: f64,
    /// Diffusivity above the boundary layer \[m² s⁻¹\] (0).
    pub background_k: f64,
    /// Return the kinetic energy removed by diffusion and drag as heat (true).
    pub frictional_heating: bool,
}

impl Default for KProfile {
    fn default() -> Self {
        KProfile {
            exchange: ExchangeParams::default(),
            ri_crit: 1.0,
            surface_layer_fraction: 0.1,
            background_k: 0.0,
            frictional_heating: true,
        }
    }
}

/// Diagnostic slots of [`KProfile`].
pub const KPROFILE_DIAGNOSTICS: &[&str] = &[
    "shf", "lhf", "evap", "tauu", "tauv", "ustar", "pblh", "rlus", "sfc_net", "tas", "huss", "uas",
    "vas", "sfcwind", "gust",
];

/// Forward elimination of the implicit diffusion system shared by all quantities:
/// x_k⁺ = E_k + F_k x_{k+1}⁺, for interface coefficients `a` (a\[k\] between layers k and
/// k+1) and c_k = Δt g / Δp_k. Writes the pivots `den` and `f`.
fn eliminate(a: &[f64], c: &[f64], den: &mut [f64], f: &mut [f64]) {
    let l = c.len();
    for k in 0..l {
        let a_up = if k > 0 { a[k - 1] } else { 0.0 };
        let a_dn = if k + 1 < l { a[k] } else { 0.0 };
        let f_prev = if k > 0 { f[k - 1] } else { 0.0 };
        den[k] = 1.0 + c[k] * (a_up + a_dn) - c[k] * a_up * f_prev;
        f[k] = c[k] * a_dn / den[k];
    }
}

/// Downward sweep in place: `e` holds x on entry and E on return. Returns (A, B) of
/// x_L⁺ = A + B·J, J the upward flux through the bottom of the lowest layer.
fn sweep(e: &mut [f64], a: &[f64], c: &[f64], den: &[f64]) -> (f64, f64) {
    let l = e.len();
    for k in 0..l {
        let prev = if k > 0 {
            c[k] * a[k - 1] * e[k - 1]
        } else {
            0.0
        };
        e[k] = (e[k] + prev) / den[k];
    }
    (e[l - 1], c[l - 1] / den[l - 1])
}

/// Back substitution from the lowest-level value `xl`; `e` holds E on entry, x⁺ on return.
fn back(e: &mut [f64], f: &[f64], xl: f64) {
    let l = e.len();
    e[l - 1] = xl;
    for k in (0..l - 1).rev() {
        e[k] += f[k] * e[k + 1];
    }
}

/// Implicit vertical diffusion of `x` in place with zero flux at the top and the upward
/// surface flux `j` at the bottom: Δp_k/g (x⁺ − x)/Δt = J_{k+1/2} − J_{k−1/2},
/// J_{k+1/2} = a_k (x_{k+1}⁺ − x_k⁺) between layers k and k+1. `c_k` = Δt g / Δp_k. The
/// column integral Σ x Δp/g changes by Δt·j.
pub fn implicit_diffusion(
    x: &mut [f64],
    a: &[f64],
    c: &[f64],
    j: f64,
    den: &mut [f64],
    f: &mut [f64],
) {
    let l = x.len();
    eliminate(a, &c[..l], &mut den[..l], &mut f[..l]);
    let (aa, bb) = sweep(x, a, c, &den[..l]);
    back(x, &f[..l], aa + bb * j);
}

/// Result of one tile's implicit energy balance.
struct TileSolve {
    /// Sensible heat flux, evaporation, net flux into the surface.
    h: f64,
    e: f64,
    net: f64,
    /// T_s⁺ − T_s \[K\] and the linearised extra emission 4σT_s³ΔT_s.
    dts: f64,
    lw_change: f64,
}

impl ColumnScheme for KProfile {
    fn name(&self) -> &'static str {
        "kprofile_boundary_layer"
    }
    fn diagnostics(&self) -> &'static [&'static str] {
        KPROFILE_DIAGNOSTICS
    }
    fn apply(
        &self,
        ctx: &ColumnCtx,
        st: &mut ColumnState,
        sfc: &mut SurfaceExchange,
        diag: &mut [f64],
        work: &mut Workspace,
    ) {
        let l = st.nlev;
        if sfc.tiles.is_empty() || l < 2 {
            return;
        }
        let planet = ctx.planet;
        let (g, rd, cp) = (planet.gravity, planet.gas_constant, planet.cp);
        let dtp = ctx.dt;
        let cond = ctx.cond.filter(|_| st.humidity().is_some());
        let lv = cond.map_or(0.0, |c| c.latent_heat);
        let ex = &self.exchange;
        let [zf, zh, phi, tv, ah, fco, den, cc, es, eq, eu, ev] = &mut work.buf;

        // geometry: virtual temperature, heights of full and half levels, geopotential
        for k in 0..l {
            let q = if cond.is_some() { st.q[k] } else { 0.0 };
            tv[k] = st.t[k] * (1.0 + 0.608 * q);
        }
        zh[l] = 0.0;
        for k in (0..l).rev() {
            let h = rd * tv[k] / g;
            zf[k] = zh[k + 1] + h * (ctx.p_half[k + 1] / ctx.p[k]).ln();
            zh[k] = if k > 0 {
                zh[k + 1] + h * (ctx.p_half[k + 1] / ctx.p_half[k]).ln()
            } else {
                zf[0]
            };
            phi[k] = ctx.phis + g * zf[k];
        }
        let ka = l - 1;
        let za = zf[ka];
        let wg2 = ex.gustiness * ex.gustiness;
        let wind = (st.u[ka] * st.u[ka] + st.v[ka] * st.v[ka] + wg2).sqrt();
        let rho_a = ctx.p[ka] / (rd * tv[ka]);
        let sva = cp * tv[ka] + g * za; // virtual dry static energy above the surface
        let qa = if cond.is_some() { st.q[ka] } else { 0.0 };

        // per-tile exchange coefficients (as if the tile covered the cell)
        let nt = sfc.tiles.len().min(MAX_TILES);
        let mut cd = [0.0; MAX_TILES];
        let mut ch = [0.0; MAX_TILES];
        let (mut cd_mean, mut ri_mean, mut lnz0m, mut lnz0h) = (0.0, 0.0, 0.0, 0.0);
        for (i, t) in sfc.tiles[..nt].iter().enumerate() {
            let (z0m, z0h) = sf::roughness(ex, t.kind, t.z0, sfc.ustar_prev, g);
            let q_eff = cond.map_or(0.0, |c| {
                t.beta * c.qsat(planet, t.ts, ctx.ps) + (1.0 - t.beta) * qa
            });
            let svs = cp * t.ts * (1.0 + 0.608 * q_eff);
            let ri = g * za * (sva - svs) / (svs * wind * wind);
            (cd[i], ch[i]) = sf::coefficients(za, z0m, z0h, ri);
            cd_mean += t.frac * cd[i];
            ri_mean += t.frac * ri;
            lnz0m += t.frac * z0m.ln();
            lnz0h += t.frac * z0h.ln();
        }
        let (z0m, z0h) = (lnz0m.exp(), lnz0h.exp());
        let ustar0 = cd_mean.sqrt() * wind;

        // boundary-layer height (FHZ06 eq. 16)
        let mut h = zf[0];
        let (mut ri_prev, mut z_prev) = (0.0, za);
        for k in (0..ka).rev() {
            let svk = cp * tv[k] + g * zf[k];
            let w2 = st.u[k] * st.u[k] + st.v[k] * st.v[k] + wg2;
            let ri = g * zf[k] * (svk - sva) / sva / w2;
            if ri > self.ri_crit {
                h = z_prev + (self.ri_crit - ri_prev) / (ri - ri_prev) * (zf[k] - z_prev);
                break;
            }
            (ri_prev, z_prev) = (ri, zf[k]);
        }

        // diffusivity at the interfaces k+1/2 (between layers k and k+1), FHZ06 eqs. 17–20
        let ratio = (ri_mean / self.ri_crit).min(1.0);
        let kb = |z: f64| {
            let base = KARMAN * ustar0 * z;
            if ri_mean <= 0.0 {
                base
            } else if ratio >= 1.0 {
                0.0
            } else {
                base / (1.0 + ratio * (z / z0m).ln().max(0.0) / (1.0 - ratio))
            }
        };
        let fb = self.surface_layer_fraction;
        let fbh = fb * h;
        for k in 0..l - 1 {
            let z = zh[k + 1];
            let kk = if z < fbh {
                kb(z)
            } else if z < h {
                let s = 1.0 - (z - fbh) / ((1.0 - fb) * h);
                kb(fbh) * z / fbh * s * s
            } else {
                0.0
            } + self.background_k;
            let rho_h = ctx.p_half[k + 1] / (rd * 0.5 * (tv[k] + tv[k + 1]));
            ah[k] = rho_h * rho_h * g * kk / (ctx.p[k + 1] - ctx.p[k]);
        }
        for k in 0..l {
            cc[k] = dtp * g / ctx.dp[k];
        }
        let (ah, cc) = (&ah[..l - 1], &cc[..l]);
        eliminate(ah, cc, &mut den[..l], &mut fco[..l]);
        let (den, fco) = (&den[..l], &fco[..l]);

        // downward sweeps of s, q, u, v
        for k in 0..l {
            es[k] = cp * st.t[k] + phi[k];
            eq[k] = if cond.is_some() { st.q[k] } else { 0.0 };
            eu[k] = st.u[k];
            ev[k] = st.v[k];
        }
        let (a_s, b) = sweep(&mut es[..l], ah, cc, den);
        let (a_q, _) = sweep(&mut eq[..l], ah, cc, den);
        let (a_u, _) = sweep(&mut eu[..l], ah, cc, den);
        let (a_v, _) = sweep(&mut ev[..l], ah, cc, den);

        // implicit energy balance of each tile
        let (sw_down, lw_down) = (sfc.sw_down, sfc.lw_down);
        let solve = |i: usize, t: &crate::surface::Tile| -> TileSolve {
            let d_h = rho_a * ch[i] * wind;
            let dh = d_h / (1.0 + d_h * b);
            let h0 = dh * (cp * t.ts + ctx.phis - a_s);
            let h1 = dh * cp;
            let (e0, e1) = match cond {
                Some(c) => {
                    let d_e = t.beta * rho_a * ch[i] * wind;
                    let de = d_e / (1.0 + d_e * b);
                    let qs = c.qsat(planet, t.ts, ctx.ps);
                    (de * (qs - a_q), de * c.dqsat_dt(qs, t.ts))
                }
                None => (0.0, 0.0),
            };
            let sig = STEFAN_BOLTZMANN * t.ts.powi(3);
            let r0 = sw_down * (1.0 - t.albedo) + lw_down - sig * t.ts;
            let r1 = 4.0 * sig;
            let dts = if t.heat_capacity.is_finite() {
                (r0 - h0 - lv * e0 + t.qflux) / (t.heat_capacity / dtp + r1 + h1 + lv * e1)
            } else {
                0.0
            };
            let (hh, ee) = (h0 + h1 * dts, e0 + e1 * dts);
            TileSolve {
                h: hh,
                e: ee,
                net: r0 - r1 * dts - hh - lv * ee,
                dts,
                lw_change: r1 * dts,
            }
        };
        let (mut hm, mut em, mut netm, mut lwc, mut dm) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let (mut ts_new, mut rlus) = (0.0, 0.0);
        for i in 0..nt {
            let r = solve(i, &sfc.tiles[i]);
            let t = &mut sfc.tiles[i];
            t.ts_new = t.ts + r.dts;
            let f = t.frac;
            hm += f * r.h;
            em += f * r.e;
            netm += f * r.net;
            lwc += f * r.lw_change;
            dm += f * rho_a * cd[i] * wind;
            ts_new += f * t.ts_new;
            rlus += f * (STEFAN_BOLTZMANN * t.ts.powi(4) + r.lw_change);
        }
        // momentum: J = −D_m x_L⁺ → x_L⁺ = A / (1 + B D_m)
        let (ul, vl) = (a_u / (1.0 + b * dm), a_v / (1.0 + b * dm));
        let (taux, tauy) = (dm * ul, dm * vl);

        // back substitution and the new state (frictional heating from the KE change)
        back(&mut eu[..l], fco, ul);
        back(&mut ev[..l], fco, vl);
        back(&mut es[..l], fco, a_s + b * hm);
        for k in 0..l {
            let dke = 0.5 * (eu[k] * eu[k] + ev[k] * ev[k] - st.u[k] * st.u[k] - st.v[k] * st.v[k]);
            let heat = if self.frictional_heating { -dke } else { 0.0 };
            st.set_u(k, eu[k]);
            st.set_v(k, ev[k]);
            st.set_t(k, (es[k] - phi[k] + heat) / cp);
        }
        if cond.is_some() {
            back(&mut eq[..l], fco, a_q + b * em);
            for k in 0..l {
                st.set_q(0, k, eq[k]);
            }
        }
        // other tracers: no surface flux
        for it in usize::from(cond.is_some())..st.ntracers() {
            eq[..l].copy_from_slice(st.tracer(it));
            let (a_x, _) = sweep(&mut eq[..l], ah, cc, den);
            back(&mut eq[..l], fco, a_x);
            for k in 0..l {
                st.set_q(it, k, eq[k]);
            }
        }

        let ustar = ((taux * taux + tauy * tauy).sqrt() / rho_a).sqrt();
        sfc.shf += hm;
        sfc.evap += em;
        sfc.lhf += lv * em;
        sfc.taux += taux;
        sfc.tauy += tauy;
        sfc.ustar = ustar;
        sfc.lw_up_change += lwc;
        sfc.sfc_net += netm;

        // near-surface diagnostics from the new state
        let tl = st.t[ka];
        let ql = if cond.is_some() { st.q[ka] } else { 0.0 };
        let w_thv = hm / (rho_a * cp) + 0.608 * tl * em / rho_a;
        let inv_l = sf::inverse_obukhov(ustar, w_thv, tl * (1.0 + 0.608 * ql), g);
        let rh2 = sf::profile_fraction(2.0, za, z0h, inv_l, true);
        let rm10 = sf::profile_fraction(10.0, za, z0m, inv_l, false);
        let theta_l = tl + g * za / cp;
        let tas = ts_new + (theta_l - ts_new) * rh2 - g * 2.0 / cp;
        let qs_sfc: f64 = cond.map_or(0.0, |c| {
            sfc.tiles[..nt]
                .iter()
                .map(|t| t.frac * (t.beta * c.qsat(planet, t.ts_new, ctx.ps) + (1.0 - t.beta) * ql))
                .sum()
        });
        let huss = qs_sfc + (ql - qs_sfc) * rh2;
        let (uas, vas) = (st.u[ka] * rm10, st.v[ka] * rm10);
        let sfcwind = (uas * uas + vas * vas).sqrt();
        let gf = if inv_l < 0.0 {
            ex.gust_unstable
        } else {
            ex.gust_stable
        };
        for (d, x) in diag.iter_mut().zip([
            hm,
            lv * em,
            em,
            taux,
            tauy,
            ustar,
            h,
            rlus,
            netm,
            tas,
            huss,
            uas,
            vas,
            sfcwind,
            sfcwind + gf * ustar,
        ]) {
            *d += x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The implicit diffusion conserves the column integral up to the surface flux and
    /// smooths.
    #[test]
    fn implicit_diffusion_conserves() {
        let mut x = [1.0, 2.0, 5.0, 3.0];
        let x0 = x;
        let a = [3e-3, 1e-2, 4e-2];
        let dp = [2e4, 3e4, 3e4, 2e4];
        let (dt, g) = (3600.0, 9.81);
        let c: Vec<f64> = dp.iter().map(|d| dt * g / d).collect();
        let (mut den, mut f) = ([0.0; 4], [0.0; 4]);
        let j = 0.01;
        implicit_diffusion(&mut x, &a, &c, j, &mut den, &mut f);
        let s0: f64 = x0.iter().zip(&dp).map(|(a, b)| a * b / g).sum();
        let s1: f64 = x.iter().zip(&dp).map(|(a, b)| a * b / g).sum();
        assert!(
            (s1 - s0 - dt * j).abs() < 1e-12 * s0,
            "{} vs {}",
            s1 - s0,
            dt * j
        );
        let var = |v: &[f64]| v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>();
        assert!(var(&x) < var(&x0));
    }
}
