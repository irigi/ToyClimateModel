//! Grid-point semi-Lagrangian tracer transport (SPEC.md §9.8 remedy for the tracer fixer
//! trigger: "horizontal semi-Lagrangian transport of tracers on the Gaussian grid
//! (shape-preserving cubic interpolation, mass fixer), vertical in flux form").
//!
//! Three time levels, consistent with the leapfrog dynamics and with the column physics
//! evaluated at n−1 (`physics_step`): the physics-updated field Q* = q^{n−1} + 2Δt·P^{n−1}
//! is carried along trajectories that end at the grid points at n+1 and pass through the
//! midpoint at n, with the winds of level n (Ritchie 1987, Mon. Wea. Rev. 115, 608–619,
//! midpoint form on the sphere: the midpoint x_m solves x_m ∝ x − Δt v(x_m)/a, iterated three
//! times; the departure point is x_d = 2(x·x_m)x_m − x, the reflection of the arrival point
//! through the midpoint on the great circle). Winds are interpolated bilinearly as
//! Cartesian components, which are smooth across the poles.
//!
//! Interpolation of Q*: tensor-product cubic Lagrange (4 longitudes × 4 Gaussian latitudes,
//! non-uniform weights in latitude), clipped to the range of the 2 × 2 surrounding points
//! (quasi-monotone, Bermejo & Staniforth 1992, Mon. Wea. Rev. 120, 2622–2632). Rows beyond
//! the outermost Gaussian latitudes are the rows on the other side of the pole, shifted by
//! 180° in longitude (the same physical points).
//!
//! Vertical: ∂q/∂t = −σ̇ ∂q/∂σ written as −[∂(σ̇q)/∂σ − q ∂σ̇/∂σ]/Δσ with face values from a
//! MUSCL reconstruction (van Leer limiter, Courant-number correction), sub-stepped so that
//! the Courant number of every step is at most 0.5; σ̇ = 0 at the top and the surface.
//! The horizontal and vertical steps are applied one after the other.
//!
//! The global tracer mass is restored by the caller with a multiplicative fixer.

use super::state::Grid3;
use crate::grid::GaussianGrid;
use crate::Real;
use rayon::prelude::*;
use std::f64::consts::PI;

/// Halo rows on each side of the Gaussian rows.
const HALO: usize = 2;
/// Midpoint iterations.
const ITERATIONS: usize = 3;
/// Largest vertical Courant number per sub-step.
const MAX_VERTICAL_COURANT: f64 = 0.5;

pub(crate) struct SemiLagrangian {
    nlat: usize,
    nlon: usize,
    /// Latitudes of the extended rows j = −HALO..nlat+HALO (index j + HALO), decreasing.
    lat_ext: Vec<f64>,
    /// Unit vectors of the grid points.
    xyz: Vec<[f64; 3]>,
    sinlon: Vec<f64>,
    coslon: Vec<f64>,
    sinlat: Vec<f64>,
    coslat: Vec<f64>,
    radius: f64,
}

/// Where a point falls: extended row j0 (lat_ext[j0] ≥ φ ≥ lat_ext[j0 + 1]) and longitude
/// index i0 with fractional parts.
#[derive(Clone, Copy)]
struct Loc {
    j0: isize,
    fy: f64,
    i0: isize,
    fx: f64,
    lat: f64,
}

impl SemiLagrangian {
    pub fn new(grid: &GaussianGrid, radius: f64) -> Self {
        let (nlat, nlon) = (grid.nlat, grid.nlon);
        assert!(
            nlon % 2 == 0,
            "semi-Lagrangian transport needs an even nlon"
        );
        let mut lat_ext = Vec::with_capacity(nlat + 2 * HALO);
        for j in -(HALO as isize)..(nlat + HALO) as isize {
            let phi = if j < 0 {
                PI - grid.lat[(-1 - j) as usize]
            } else if j >= nlat as isize {
                -PI - grid.lat[2 * nlat - 1 - j as usize]
            } else {
                grid.lat[j as usize]
            };
            lat_ext.push(phi);
        }
        let mut xyz = Vec::with_capacity(nlat * nlon);
        for j in 0..nlat {
            for i in 0..nlon {
                let (la, lo) = (grid.lat[j], grid.lon[i]);
                xyz.push([la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]);
            }
        }
        SemiLagrangian {
            nlat,
            nlon,
            lat_ext,
            xyz,
            sinlon: grid.lon.iter().map(|l| l.sin()).collect(),
            coslon: grid.lon.iter().map(|l| l.cos()).collect(),
            sinlat: grid.lat.iter().map(|l| l.sin()).collect(),
            coslat: grid.lat.iter().map(|l| l.cos()).collect(),
            radius,
        }
    }

    fn locate(&self, lat: f64, lon: f64) -> Loc {
        // rows are decreasing in latitude; first row with lat_ext <= lat, minus one
        let k = self.lat_ext.partition_point(|&y| y > lat);
        let j0 = (k as isize - 1).clamp(0, (self.lat_ext.len() - 2) as isize);
        let (y0, y1) = (self.lat_ext[j0 as usize], self.lat_ext[j0 as usize + 1]);
        let fy = (y0 - lat) / (y0 - y1);
        let x = lon.rem_euclid(2.0 * PI) / (2.0 * PI) * self.nlon as f64;
        let i0 = x.floor();
        Loc {
            j0: j0 - HALO as isize,
            fy,
            i0: i0 as isize,
            fx: x - i0,
            lat,
        }
    }

    /// Flat index of extended row `j`, longitude index `i` (wrapped; across the pole the
    /// row is mirrored and shifted by half a circle).
    #[inline]
    fn index(&self, j: isize, i: isize) -> usize {
        let (nlat, nlon) = (self.nlat as isize, self.nlon as isize);
        let (jj, shift) = if j < 0 {
            (-1 - j, nlon / 2)
        } else if j >= nlat {
            (2 * nlat - 1 - j, nlon / 2)
        } else {
            (j, 0)
        };
        (jj * nlon + (i + shift).rem_euclid(nlon)) as usize
    }

    fn bilinear(&self, f: &[f64], loc: &Loc) -> f64 {
        let (j, i) = (loc.j0, loc.i0);
        let a = f[self.index(j, i)] * (1.0 - loc.fx) + f[self.index(j, i + 1)] * loc.fx;
        let b = f[self.index(j + 1, i)] * (1.0 - loc.fx) + f[self.index(j + 1, i + 1)] * loc.fx;
        a * (1.0 - loc.fy) + b * loc.fy
    }

    /// Cubic Lagrange interpolation clipped to the 2 × 2 surrounding values.
    fn cubic(&self, f: &[Real], loc: &Loc) -> Real {
        let t = loc.fx;
        let wx = [
            -t * (t - 1.0) * (t - 2.0) / 6.0,
            (t + 1.0) * (t - 1.0) * (t - 2.0) / 2.0,
            -(t + 1.0) * t * (t - 2.0) / 2.0,
            (t + 1.0) * t * (t - 1.0) / 6.0,
        ];
        let base = (loc.j0 + HALO as isize) as usize;
        let ys = [
            self.lat_ext[base - 1],
            self.lat_ext[base],
            self.lat_ext[base + 1],
            self.lat_ext[base + 2],
        ];
        let mut wy = [1.0f64; 4];
        for (a, w) in wy.iter_mut().enumerate() {
            for (b, &yb) in ys.iter().enumerate() {
                if a != b {
                    *w *= (loc.lat - yb) / (ys[a] - yb);
                }
            }
        }
        let mut v = 0.0;
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for (dj, w_y) in wy.iter().enumerate() {
            let j = loc.j0 - 1 + dj as isize;
            let mut row = 0.0;
            for (di, w_x) in wx.iter().enumerate() {
                let i = loc.i0 - 1 + di as isize;
                let x = f[self.index(j, i)] as f64;
                row += w_x * x;
                if (dj == 1 || dj == 2) && (di == 1 || di == 2) {
                    lo = lo.min(x);
                    hi = hi.max(x);
                }
            }
            v += w_y * row;
        }
        v.clamp(lo, hi) as Real
    }

    /// Horizontal departure-point interpolation of one level: `src` advected by the winds
    /// (`u`, `v` = cos φ-weighted U, V of level n) over `tau`, written to `dst`.
    pub fn horizontal(&self, src: &[Real], u: &[Real], v: &[Real], tau: f64, dst: &mut [Real]) {
        let (nlat, nlon) = (self.nlat, self.nlon);
        let n = nlat * nlon;
        // Cartesian wind components on the grid
        let mut vel = [vec![0.0; n], vec![0.0; n], vec![0.0; n]];
        for j in 0..nlat {
            let (sl, cl) = (self.sinlat[j], self.coslat[j]);
            for i in 0..nlon {
                let p = j * nlon + i;
                let (ue, vn) = (u[p] as f64 / cl, v[p] as f64 / cl);
                let (sx, cx) = (self.sinlon[i], self.coslon[i]);
                vel[0][p] = -ue * sx - vn * sl * cx;
                vel[1][p] = ue * cx - vn * sl * sx;
                vel[2][p] = vn * cl;
            }
        }
        let h = 0.5 * tau / self.radius;
        dst.par_chunks_mut(nlon).enumerate().for_each(|(j, row)| {
            for (i, out) in row.iter_mut().enumerate() {
                let x = self.xyz[j * nlon + i];
                let mut xm = x;
                for _ in 0..ITERATIONS {
                    let loc = self.locate(xm[2].clamp(-1.0, 1.0).asin(), xm[1].atan2(xm[0]));
                    let w = [
                        self.bilinear(&vel[0], &loc),
                        self.bilinear(&vel[1], &loc),
                        self.bilinear(&vel[2], &loc),
                    ];
                    let mut y = [x[0] - h * w[0], x[1] - h * w[1], x[2] - h * w[2]];
                    let norm = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt();
                    y.iter_mut().for_each(|c| *c /= norm);
                    xm = y;
                }
                let d = x[0] * xm[0] + x[1] * xm[1] + x[2] * xm[2];
                let xd = [
                    2.0 * d * xm[0] - x[0],
                    2.0 * d * xm[1] - x[1],
                    2.0 * d * xm[2] - x[2],
                ];
                let loc = self.locate(xd[2].clamp(-1.0, 1.0).asin(), xd[1].atan2(xd[0]));
                *out = self.cubic(src, &loc);
            }
        });
    }
}

/// Vertical transport of one column `q` (top to bottom) by σ̇ at the half levels between
/// full levels (`sdot[k]` between k and k + 1; the last entry, at the surface, is ignored),
/// over `tau`. `sigma` are the full levels and `ds` the layer thicknesses Δσ.
pub(crate) fn vertical(
    q: &mut [f64],
    sdot: &[f64],
    sigma: &[f64],
    ds: &[f64],
    tau: f64,
    scratch: &mut Vec<f64>,
) {
    let l = q.len();
    if l < 2 {
        return;
    }
    // Courant numbers of the faces, measured in the upwind layer
    let mut cmax = 0.0f64;
    for k in 0..l - 1 {
        let up = if sdot[k] > 0.0 { k } else { k + 1 };
        cmax = cmax.max(sdot[k].abs() * tau / ds[up]);
    }
    if cmax == 0.0 {
        return;
    }
    let nsub = (cmax / MAX_VERTICAL_COURANT).ceil().max(1.0) as usize;
    let dt = tau / nsub as f64;
    scratch.clear();
    scratch.resize(2 * l + 1, 0.0);
    let (slope, flux) = scratch.split_at_mut(l);
    for _ in 0..nsub {
        for k in 0..l {
            slope[k] = if k == 0 || k == l - 1 {
                0.0
            } else {
                let dl = (q[k] - q[k - 1]) / (sigma[k] - sigma[k - 1]);
                let dr = (q[k + 1] - q[k]) / (sigma[k + 1] - sigma[k]);
                if dl * dr > 0.0 {
                    2.0 * dl * dr / (dl + dr)
                } else {
                    0.0
                }
            };
        }
        // flux[k + 1] = σ̇ q at the face between k and k + 1; flux[0] = flux[l] = 0
        flux[0] = 0.0;
        flux[l] = 0.0;
        for k in 0..l - 1 {
            let w = sdot[k];
            let face = if w > 0.0 {
                let c = w * dt / ds[k];
                q[k] + 0.5 * (1.0 - c) * ds[k] * slope[k]
            } else {
                let c = -w * dt / ds[k + 1];
                q[k + 1] - 0.5 * (1.0 - c) * ds[k + 1] * slope[k + 1]
            };
            flux[k + 1] = w * face;
        }
        for k in 0..l {
            let w_below = if k + 1 < l { sdot[k] } else { 0.0 };
            let w_above = if k > 0 { sdot[k - 1] } else { 0.0 };
            let conv = (flux[k + 1] - flux[k]) - q[k] * (w_below - w_above);
            q[k] -= dt * conv / ds[k];
        }
    }
}

/// Full transport of one tracer: horizontal for every level, then vertical for every
/// column. `src` is Q*, `u`, `v` the cos φ-weighted winds and `sdl` σ̇ below each full
/// level (all of level n); the result goes to `dst`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn transport(
    sl: &SemiLagrangian,
    src: &Grid3,
    u: &Grid3,
    v: &Grid3,
    sdl: &Grid3,
    sigma: &[f64],
    ds: &[f64],
    tau: f64,
    dst: &mut Grid3,
) {
    let l = src.nlev();
    let n = src.level_len();
    for k in 0..l {
        sl.horizontal(&src[k], &u[k], &v[k], tau, &mut dst[k]);
    }
    let nlon = sl.nlon;
    let rows: Vec<Vec<f64>> = (0..sl.nlat)
        .into_par_iter()
        .map(|j| {
            let (mut col, mut sd, mut scratch) = (vec![0.0; l], vec![0.0; l], Vec::new());
            let mut out = vec![0.0; l * nlon];
            for i in 0..nlon {
                let p = j * nlon + i;
                for k in 0..l {
                    col[k] = dst[k][p] as f64;
                    sd[k] = sdl[k][p] as f64;
                }
                vertical(&mut col, &sd, sigma, ds, tau, &mut scratch);
                for k in 0..l {
                    out[k * nlon + i] = col[k];
                }
            }
            out
        })
        .collect();
    for (j, row) in rows.iter().enumerate() {
        for k in 0..l {
            for i in 0..nlon {
                dst[k][j * nlon + i] = row[k * nlon + i] as Real;
            }
        }
    }
    debug_assert_eq!(n, sl.nlat * sl.nlon);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> GaussianGrid {
        GaussianGrid::for_truncation(21)
    }

    /// Williamson et al. (1992) test 1: a cosine bell of radius a/3 carried once around the
    /// globe by solid-body rotation; returns (relative l2 error, peak, min, max).
    fn bell(m: usize, alpha: f64, dt: f64) -> (f64, f64, f64, f64) {
        let g = GaussianGrid::for_truncation(m);
        let a = 6.371e6;
        let sl = SemiLagrangian::new(&g, a);
        let (nlat, nlon) = (g.nlat, g.nlon);
        let n = nlat * nlon;
        let u0 = 2.0 * PI * a / (12.0 * 86400.0);
        let (mut u, mut v, mut q) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        let (lc, pc, r0) = (1.5 * PI, 0.0f64, a / 3.0);
        for j in 0..nlat {
            for i in 0..nlon {
                let (la, lo) = (g.lat[j], g.lon[i]);
                let p = j * nlon + i;
                u[p] = (u0
                    * (la.cos() * alpha.cos() + la.sin() * lo.cos() * alpha.sin())
                    * la.cos()) as Real;
                v[p] = (-u0 * lo.sin() * alpha.sin() * la.cos()) as Real;
                let r = a
                    * (pc.sin() * la.sin() + pc.cos() * la.cos() * (lo - lc).cos())
                        .clamp(-1.0, 1.0)
                        .acos();
                q[p] = if r < r0 {
                    0.5 * (1.0 + (PI * r / r0).cos())
                } else {
                    0.0
                } as Real;
            }
        }
        let steps = (12.0 * 86400.0 / dt).round() as usize;
        let mut cur = q.clone();
        let mut next = vec![0.0 as Real; n];
        for _ in 0..steps {
            sl.horizontal(&cur, &u, &v, dt, &mut next);
            std::mem::swap(&mut cur, &mut next);
        }
        let l2 = (cur
            .iter()
            .zip(&q)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<Real>()
            / q.iter().map(|b| b * b).sum::<Real>())
        .sqrt() as f64;
        let peak = cur.iter().cloned().fold(0.0, Real::max) as f64;
        let mn = cur.iter().cloned().fold(Real::INFINITY, Real::min) as f64;
        (l2, peak, mn, q.iter().cloned().fold(0.0, Real::max) as f64)
    }

    /// Solid-body rotation straight over the poles (α = π/2) and along the equator: the
    /// bell returns with no new extrema (no undershoot below 0, no overshoot), and the
    /// error over the poles is close to the error along the equator (the pole treatment
    /// adds no error of its own). Numbers are printed for the record.
    #[test]
    fn solid_body_rotation_over_the_poles() {
        for (m, dt) in [(21, 3600.0), (21, 4.0 * 3600.0), (42, 1800.0), (42, 3600.0)] {
            let (lp, pp, mnp, mx) = bell(m, PI / 2.0, dt);
            let (le, pe, mne, _) = bell(m, 0.0, dt);
            eprintln!(
                "T{m}, dt {:.1} h: over the poles l2 {lp:.3} peak {pp:.3}; along the equator \
                 l2 {le:.3} peak {pe:.3}",
                dt / 3600.0
            );
            assert!(mnp >= 0.0 && mne >= 0.0 && pp <= mx + 1e-12 && pe <= mx + 1e-12);
            assert!(lp < 1.5 * le + 0.05, "pole path l2 {lp} vs equator {le}");
        }
    }

    /// A constant field stays constant (interpolation weights sum to one, the vertical
    /// step with the q ∂σ̇/∂σ term leaves a uniform column alone).
    #[test]
    fn constant_field_is_preserved() {
        let g = grid();
        let sl = SemiLagrangian::new(&g, 6.371e6);
        let n = g.nlat * g.nlon;
        let u: Vec<Real> = (0..n)
            .map(|p| (20.0 * ((p % 7) as f64).sin()) as Real)
            .collect();
        let v: Vec<Real> = (0..n)
            .map(|p| (15.0 * ((p % 5) as f64).cos()) as Real)
            .collect();
        let q = vec![0.01 as Real; n];
        let mut out = vec![0.0 as Real; n];
        sl.horizontal(&q, &u, &v, 1800.0, &mut out);
        assert!(out.iter().all(|&x| (x - 0.01).abs() < 1e-14));
        let sigma = [0.1, 0.3, 0.5, 0.7, 0.9];
        let ds = [0.2; 5];
        let sdot = [1e-5, -3e-5, 2e-5, 4e-5, 0.0];
        let mut col = [0.01; 5];
        vertical(&mut col, &sdot, &sigma, &ds, 3600.0, &mut Vec::new());
        assert!(col.iter().all(|&x| (x - 0.01).abs() < 1e-15), "{col:?}");
    }

    /// Vertical step: a non-negative column stays within its initial range under a
    /// divergent σ̇ and a time step with Courant number ≈ 3 (sub-stepping), and uniform
    /// downward motion moves a moist layer downward.
    #[test]
    fn vertical_is_bounded_and_moves_downward() {
        let sigma = [0.05, 0.2, 0.4, 0.6, 0.8, 0.95];
        let ds = [0.1, 0.2, 0.2, 0.2, 0.2, 0.1];
        let sdot = [1e-5, 3e-5, -2e-5, 4e-5, 2e-5, 0.0];
        let mut col = [0.0, 0.0, 1e-3, 1e-2, 0.0, 1e-2];
        vertical(&mut col, &sdot, &sigma, &ds, 6.0 * 3600.0, &mut Vec::new());
        assert!(
            col.iter().all(|&x| (0.0..=1e-2 + 1e-15).contains(&x)),
            "{col:?}"
        );
        let mut col = [0.0, 1e-2, 0.0, 0.0, 0.0, 0.0];
        vertical(
            &mut col,
            &[2e-5; 6],
            &sigma,
            &ds,
            3600.0 * 3.0,
            &mut Vec::new(),
        );
        assert!(col[2] > 0.0 && col[1] < 1e-2, "{col:?}");
    }
}
