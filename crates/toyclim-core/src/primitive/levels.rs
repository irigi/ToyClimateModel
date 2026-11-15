//! Diagnostic 3-D fields on model levels and their interpolation to pressure levels.

use super::Primitive;
use crate::Real;

/// 2D fields interpolated to pressure levels.
pub const PRESSURE_FIELDS: &[&str] = &["t850", "u850", "v850", "t500", "phi500", "u250", "vor850"];

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

    /// A 3D field on model levels (`[lev][lat][lon]`) by name: u, v, vor, div, t, q, rh, phi.
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
                let tq = s.tracers.first()?;
                (0..l).for_each(|k| out.extend(tr.synthesis(&tq[k])));
            }
            "rh" => {
                let cond = self.condensable.as_ref()?;
                let tq = s.tracers.first()?;
                let ps: Vec<Real> = tr.synthesis(&s.lnps).iter().map(|x| x.exp()).collect();
                for k in 0..l {
                    let q = tr.synthesis(&tq[k]);
                    let t = tr.synthesis(&s.tmp[k]);
                    for p in 0..n {
                        let pr = self.levels.full[k] * ps[p] as f64;
                        let qs = cond.qsat(&self.planet, (t[p] + tref) as f64, pr);
                        out.push((q[p] as f64 / qs) as Real);
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
}
