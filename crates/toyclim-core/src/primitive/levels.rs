//! Diagnostic 3-D fields on model levels and their interpolation to pressure levels.

use super::Primitive;
use crate::Real;

/// 2D fields interpolated to pressure levels.
pub const PRESSURE_FIELDS: &[&str] = &[
    "t850", "u850", "v850", "t500", "phi500", "u250", "u200", "vor850",
];

impl Primitive {
    /// Interpolate a 3D field (`[lev][lat][lon]`) to pressure `p` \[Pa\], linear in ln p
    /// between full levels; constant extrapolation outside the column.
    pub(crate) fn to_pressure(&self, field: &[Real], p: f64) -> Vec<Real> {
        let tr = &self.tr;
        let n = tr.ngrid();
        let l = self.nlev();
        let ps = tr.synthesis(&self.curr.lnps);
        let lnsig: Vec<f64> = self.levels.full.iter().map(|s| s.ln()).collect();
        (0..n)
            .map(|q| {
                let target = p.ln() - ps[q] as f64; // ln σ of the target
                if target <= lnsig[0] {
                    return field[q];
                }
                if target >= lnsig[l - 1] {
                    return field[(l - 1) * n + q];
                }
                let k = lnsig.partition_point(|&x| x < target) - 1;
                let w = (target - lnsig[k]) / (lnsig[k + 1] - lnsig[k]);
                (field[k * n + q] as f64 * (1.0 - w) + field[(k + 1) * n + q] as f64 * w) as Real
            })
            .collect()
    }

    /// A 3D field on model levels (`[lev][lat][lon]`) by name: u, v, vor, div, t, q, rh, cl, cldwp,
    /// phi.
    pub(crate) fn level_fields(&self, name: &str) -> Option<Vec<Real>> {
        let tr = &self.tr;
        let l = self.nlev();
        let nlon = tr.nlon();
        let n = tr.ngrid();
        let tref = self.planet.t_ref as Real;
        let unweight = |x: &mut [Real]| {
            for j in 0..tr.nlat() {
                let c = (1.0 / (1.0 - tr.grid.mu[j] * tr.grid.mu[j]).sqrt()) as Real;
                x[j * nlon..(j + 1) * nlon].iter_mut().for_each(|v| *v *= c);
            }
        };
        let s = &self.curr;
        let mut out = Vec::with_capacity(n * l);
        match name {
            "u" | "v" => {
                for k in 0..l {
                    let (mut u, mut v) = tr.uv_from_vort_div(&s.vor[k], &s.div[k]);
                    let x = if name == "u" { &mut u } else { &mut v };
                    unweight(x);
                    out.extend_from_slice(x);
                }
            }
            "vor" => (0..l).for_each(|k| out.extend(tr.synthesis(&s.vor[k]))),
            "div" => (0..l).for_each(|k| out.extend(tr.synthesis(&s.div[k]))),
            "t" => {
                (0..l).for_each(|k| out.extend(tr.synthesis(&s.tmp[k]).iter().map(|t| t + tref)))
            }
            "q" => {
                for k in 0..l {
                    out.extend(s.tracer_level(tr, 0, k)?);
                }
            }
            "rh" => {
                let cond = self.condensable.as_ref()?;
                s.tracer_level(tr, 0, 0)?;
                let ps: Vec<Real> = tr.synthesis(&s.lnps).iter().map(|x| x.exp()).collect();
                for k in 0..l {
                    let q = s.tracer_level(tr, 0, k)?;
                    let t = tr.synthesis(&s.tmp[k]);
                    for p in 0..n {
                        let pr = self.levels.full[k] * ps[p] as f64;
                        let qs = cond.qsat(&self.planet, (t[p] + tref) as f64, pr);
                        out.push((q[p] as f64 / qs) as Real);
                    }
                }
            }
            // clouds of the last physics step (SPEC.md §9.5.1 `cl`): fraction per layer, and
            // `cldwp`, the in-cloud condensate path per layer (liquid + ice) [kg m⁻²]; from the
            // cloud scheme's per-column memory `[frac | liquid | ice]` (only scheme with memory)
            "cl" | "cldwp" => {
                let m = self.physics.memory();
                if m.len() != n * 3 * l && m.len() != n * 4 * l {
                    return None;
                }
                let s = m.len() / n; // 3·L, or 4·L with the CRE-split block
                for k in 0..l {
                    for p in 0..n {
                        let c = &m[p * s..(p + 1) * s];
                        out.push(if name == "cl" {
                            c[k]
                        } else {
                            c[l + k] + c[2 * l + k]
                        });
                    }
                }
            }
            "phi" => {
                // geopotential Φ_k = Φ_s + Σ_j G_kj T_j (full T)
                let t: Vec<Vec<Real>> = (0..l)
                    .map(|k| tr.synthesis(&s.tmp[k]).iter().map(|x| x + tref).collect())
                    .collect();
                let g = &self.si.g;
                for k in 0..l {
                    for p in 0..n {
                        let mut v = self.phis_grid[p];
                        for j in k..l {
                            v += g[k * l + j] * t[j][p];
                        }
                        out.push(v);
                    }
                }
            }
            _ => return None,
        }
        Some(out)
    }

    /// Sea-level pressure \[Pa\] by the ECMWF reduction (Trenberth, Berry & Buja 1993,
    /// NCAR TN-396): T* = T_L (p_s/p_L)^{R γ/g} from the lowest level with γ = 6.5 K/km,
    /// T₀ = T* + γ z_s; if T* < 255 K, T* ← (T* + 255)/2; if T₀ > 290.5 K, γ ← (290.5 − T*)/z_s
    /// (or γ = 0 and T* ← (290.5 + T*)/2 when T* > 290.5 K); then
    /// p_msl = p_s exp[β (1 − αβ/2 + (αβ)²/3)], β = Φ_s/(R T*), α = R γ / g.
    /// **Unverified**: written from memory; the NCAR note was not reachable on 2026-11-16
    /// (docs/progress.md). Check before the P5 sea-level-pressure row.
    pub(crate) fn sea_level_pressure(&self) -> Vec<Real> {
        let tr = &self.tr;
        let l = self.nlev();
        let (g, r) = (self.planet.gravity, self.planet.gas_constant);
        let ps = tr.synthesis(&self.curr.lnps);
        let tl = tr.synthesis(&self.curr.tmp[l - 1]);
        let sig_l = self.levels.full[l - 1];
        let tref = self.planet.t_ref;
        (0..tr.ngrid())
            .map(|q| {
                let p_s = (ps[q] as f64).exp();
                let phis = self.phis_grid[q] as f64;
                if phis.abs() < 1e-3 {
                    return p_s as Real;
                }
                let zs = phis / g;
                let mut gamma = 6.5e-3;
                let mut tstar = (tl[q] as f64 + tref) * (1.0 / sig_l).powf(r * gamma / g);
                if tstar < 255.0 {
                    tstar = 0.5 * (tstar + 255.0);
                }
                let t0 = tstar + gamma * zs;
                if t0 > 290.5 {
                    if tstar > 290.5 {
                        gamma = 0.0;
                        tstar = 0.5 * (290.5 + tstar);
                    } else {
                        gamma = (290.5 - tstar) / zs;
                    }
                }
                let beta = phis / (r * tstar);
                let ab = r * gamma / g * beta;
                (p_s * (beta * (1.0 - 0.5 * ab + ab * ab / 3.0)).exp()) as Real
            })
            .collect()
    }
}
