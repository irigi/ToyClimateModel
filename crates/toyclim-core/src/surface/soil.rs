//! Soil heat conduction (SPEC.md §9.4.5): up to [`NSOIL`] layers, implicit diffusion, zero
//! flux at the bottom; the land tile's surface temperature is the top layer's.
//!
//! Layer thicknesses: d₁ = 0.25·√(2κ_s/ω_day) clamped to [0.01, 0.2] m (0.03 m without a
//! diurnal cycle), each layer three times thicker than the one above. Volumetric heat
//! capacity 2.0e6 J m⁻³ K⁻¹ and conductivity 1.0 W m⁻¹ K⁻¹ **[knobs]** (SPEC.md values).
//! A single layer of prescribed heat capacity (no conduction) is kept for the P1
//! single-column stability tests (`[surface.land] heat_capacity`).

/// Maximum number of soil layers.
pub const NSOIL: usize = 5;

/// Soil column parameters (the same for every land cell).
#[derive(Debug, Clone)]
pub struct SoilColumn {
    /// Number of layers used (1 or [`NSOIL`]).
    pub n: usize,
    /// Heat capacity per area of each layer \[J m⁻² K⁻¹\].
    pub c: [f64; NSOIL],
    /// Conductance between layers k and k+1 \[W m⁻² K⁻¹\].
    pub kint: [f64; NSOIL],
    /// Layer thicknesses \[m\] (diagnostic).
    pub dz: [f64; NSOIL],
}

impl SoilColumn {
    /// Five layers for volumetric heat capacity `cv`, conductivity `k` and solar-day
    /// angular frequency `omega_day` (None: no diurnal cycle).
    pub fn layered(cv: f64, k: f64, omega_day: Option<f64>) -> Self {
        let d1 = match omega_day {
            Some(w) if w > 0.0 => (0.25 * (2.0 * k / cv / w).sqrt()).clamp(0.01, 0.2),
            _ => 0.03,
        };
        let mut s = SoilColumn {
            n: NSOIL,
            c: [0.0; NSOIL],
            kint: [0.0; NSOIL],
            dz: [0.0; NSOIL],
        };
        let mut d = d1;
        for i in 0..NSOIL {
            s.dz[i] = d;
            s.c[i] = cv * d;
            d *= 3.0;
        }
        for i in 0..NSOIL - 1 {
            s.kint[i] = k / (0.5 * (s.dz[i] + s.dz[i + 1]));
        }
        s
    }

    /// One layer of heat capacity `c` \[J m⁻² K⁻¹\] (prototype land, SCM stability tests).
    pub fn single(c: f64) -> Self {
        let mut s = SoilColumn {
            n: 1,
            c: [0.0; NSOIL],
            kint: [0.0; NSOIL],
            dz: [0.0; NSOIL],
        };
        s.c[0] = c;
        s
    }

    /// Downward elimination of the backward-Euler conduction over `dt` for the unknown
    /// flux F into the top: T₁⁺ = A + F/Q. Returns (A, Q). `a`, `b` are scratch.
    fn eliminate(
        &self,
        t: &[f64; NSOIL],
        dt: f64,
        a: &mut [f64; NSOIL],
        b: &mut [f64; NSOIL],
    ) -> (f64, f64) {
        let n = self.n;
        // from the bottom: T_k⁺ = a_k + b_k T_{k−1}⁺
        for k in (1..n).rev() {
            let up = self.kint[k - 1];
            let (dn, (an, bn)) = if k + 1 < n {
                (self.kint[k], (a[k + 1], b[k + 1]))
            } else {
                (0.0, (0.0, 0.0))
            };
            let ck = self.c[k] / dt;
            let den = ck + up + dn * (1.0 - bn);
            a[k] = (ck * t[k] + dn * an) / den;
            b[k] = up / den;
        }
        let c1 = self.c[0] / dt;
        let (dn, an, bn) = if n > 1 {
            (self.kint[0], a[1], b[1])
        } else {
            (0.0, 0.0, 0.0)
        };
        let q = c1 + dn * (1.0 - bn);
        ((c1 * t[0] + dn * an) / q, q)
    }

    /// Linearisation of the heat uptake for the surface solve over `dt`: the flux into the
    /// soil is G = g0 + g1 ΔT_s, ΔT_s = T₁⁺ − T₁. Returns (g0, g1).
    pub fn uptake(&self, t: &[f64; NSOIL], dt: f64) -> (f64, f64) {
        let (mut a, mut b) = ([0.0; NSOIL], [0.0; NSOIL]);
        let (aa, q) = self.eliminate(t, dt, &mut a, &mut b);
        (q * (t[0] - aa), q)
    }

    /// Advance the layer temperatures over `dt` with the flux `f` into the top
    /// \[W m⁻²\]. Conserves Σ C_k T_k + f dt exactly up to round-off.
    pub fn step(&self, t: &mut [f64; NSOIL], dt: f64, f: f64) {
        let (mut a, mut b) = ([0.0; NSOIL], [0.0; NSOIL]);
        let (aa, q) = self.eliminate(t, dt, &mut a, &mut b);
        t[0] = aa + f / q;
        for k in 1..self.n {
            t[k] = a[k] + b[k] * t[k - 1];
        }
    }

    /// Heat content Σ C_k T_k \[J m⁻²\].
    pub fn heat(&self, t: &[f64; NSOIL]) -> f64 {
        (0..self.n).map(|k| self.c[k] * t[k]).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_layers_and_conservation() {
        let s = SoilColumn::layered(2.0e6, 1.0, Some(2.0 * std::f64::consts::PI / 86400.0));
        // d1 = 0.25 √(2 · 5e-7 / 7.27e-5) ≈ 0.029 m, total ≈ 3.5 m
        assert!((s.dz[0] - 0.0293).abs() < 2e-4, "{:?}", s.dz);
        assert!((s.dz.iter().sum::<f64>() - 3.54).abs() < 0.03);
        let mut t = [290.0, 288.0, 285.0, 284.0, 283.0];
        let h0 = s.heat(&t);
        let (g0, g1) = s.uptake(&t, 1800.0);
        // the linearisation predicts the step: T₁⁺ − T₁ = (F − g0)/g1
        let f = 120.0;
        let mut t2 = t;
        s.step(&mut t2, 1800.0, f);
        assert!(((t2[0] - t[0]) - (f - g0) / g1).abs() < 1e-12);
        assert!((s.heat(&t2) - h0 - f * 1800.0).abs() < 1e-6);
        // zero flux: heat conserved, profile relaxes
        for _ in 0..1000 {
            s.step(&mut t, 3600.0, 0.0);
        }
        assert!((s.heat(&t) - h0).abs() < 1e-5 * h0.abs());
        assert!((t[0] - t[4]).abs() < 0.5, "{t:?}");
    }

    #[test]
    fn single_layer_is_a_slab() {
        let s = SoilColumn::single(1.0e3);
        let (g0, g1) = s.uptake(&[300.0; NSOIL], 3600.0);
        assert_eq!(g0, 0.0);
        assert!((g1 - 1.0e3 / 3600.0).abs() < 1e-15);
    }
}
