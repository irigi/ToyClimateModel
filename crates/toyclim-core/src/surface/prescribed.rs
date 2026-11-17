//! Prescribed ocean (AMIP-style boundary condition, SPEC.md §9.6 P5): monthly SST and
//! sea-ice concentration read from files and interpolated in time.
//!
//! - Time: the year (the calendar's, t = 0 at its start) is split into 12 equal months;
//!   the monthly values sit at mid-month and are interpolated linearly, cyclically.
//! - Open water takes the SST with an infinite heat capacity (as `fixed = true`).
//! - Sea ice covers the prescribed concentration of the ocean area with floes of fixed
//!   thickness, 2 m in the northern and 1 m in the southern hemisphere by default: the
//!   CICE prescribed-ice mode ("the sea ice thickness is reset to 2 m in the northern
//!   hemisphere and 1 m in the southern at every timestep", CICE documentation for CESM
//!   1.0, Hunke & Lipscomb, www2.cesm.ucar.edu/models/cesm1.0/cice/doc/node22.html). The
//!   CAM6 Scientific Guide §8 gives 2 m / 0.5 m for its own prescribed ice; the value is a
//!   knob in `docs/tuning.md`. The ice skin temperature comes from the zero-layer energy
//!   balance of [`super::sea_ice`] (conduction to a base at the freezing point, surface
//!   melt above T_melt); melt and conduction energy go to the prescribed ocean, and snow
//!   falling on the ice joins the ocean reservoir (no snow on prescribed ice).
//!
//! Like the fixed SST, this is not energy-conserving for the system: the ocean takes any
//! flux. The atmosphere's budget and the water budget (ocean as a reservoir) still close.

/// Monthly SST and sea-ice concentration per grid point.
#[derive(Debug, Clone)]
pub struct PrescribedOcean {
    /// `[month][point]` SST \[K\] and concentration \[0, 1\].
    pub sst: Vec<Vec<f64>>,
    pub sic: Vec<Vec<f64>>,
    /// Floe thickness per point \[m\] (by hemisphere).
    pub floe: Vec<f64>,
    /// Year length \[s\].
    pub year: f64,
}

impl PrescribedOcean {
    /// From flat `[12][npoint]` arrays; `sic` None → no sea ice. `lat` per point \[rad\].
    pub fn new(
        sst: &[f64],
        sic: Option<&[f64]>,
        lat: &[f64],
        thickness: [f64; 2],
        year: f64,
    ) -> Result<Self, String> {
        let n = lat.len();
        if sst.len() != 12 * n {
            return Err(format!(
                "prescribed SST has {} values, expected 12 × {n}",
                sst.len()
            ));
        }
        if let Some(s) = sic {
            if s.len() != 12 * n {
                return Err(format!(
                    "prescribed sea ice has {} values, expected 12 × {n}",
                    s.len()
                ));
            }
        }
        if sst.iter().any(|x| !x.is_finite() || *x < 100.0) {
            return Err("prescribed SST must be finite and in kelvin".into());
        }
        if thickness.iter().any(|&h| h <= 0.0) {
            return Err("prescribed ice thickness must be positive".into());
        }
        let split = |a: &[f64]| a.chunks(n).map(|c| c.to_vec()).collect::<Vec<_>>();
        let sic = match sic {
            Some(s) => split(
                &s.iter()
                    .map(|&x| {
                        if x.is_finite() {
                            x.clamp(0.0, 1.0)
                        } else {
                            0.0
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            None => vec![vec![0.0; n]; 12],
        };
        Ok(PrescribedOcean {
            sst: split(sst),
            sic,
            floe: lat
                .iter()
                .map(|&l| if l >= 0.0 { thickness[0] } else { thickness[1] })
                .collect(),
            year,
        })
    }

    /// Months (m0, m1) and the weight of m1 at time t: linear between mid-months.
    pub fn weights(&self, t: f64) -> (usize, usize, f64) {
        let x = (t / self.year).rem_euclid(1.0) * 12.0 - 0.5;
        let f = x.floor();
        let m0 = (f as i64).rem_euclid(12) as usize;
        (m0, (m0 + 1) % 12, x - f)
    }

    /// SST and concentration of point p at time t.
    pub fn at(&self, p: usize, w: (usize, usize, f64)) -> (f64, f64) {
        let (m0, m1, a) = w;
        let lerp = |v: &Vec<Vec<f64>>| (1.0 - a) * v[m0][p] + a * v[m1][p];
        (lerp(&self.sst), lerp(&self.sic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ocean() -> PrescribedOcean {
        // two points (north, south); SST = 270 + month, sic = month / 11
        let sst: Vec<f64> = (0..12).flat_map(|m| [270.0 + m as f64; 2]).collect();
        let sic: Vec<f64> = (0..12).flat_map(|m| [m as f64 / 11.0; 2]).collect();
        PrescribedOcean::new(&sst, Some(&sic), &[0.5, -0.5], [2.0, 1.0], 360.0).unwrap()
    }

    #[test]
    fn mid_month_values_and_cyclic_interpolation() {
        let o = ocean();
        // mid-January (t = 15 of a 360 s year) → January exactly
        let (t, a) = o.at(0, o.weights(15.0));
        assert!((t - 270.0).abs() < 1e-12 && a.abs() < 1e-12);
        // 1 January: halfway between December and January
        let (t, _) = o.at(0, o.weights(0.0));
        assert!((t - 0.5 * (281.0 + 270.0)).abs() < 1e-12, "{t}");
        // end of the year wraps the same way; one year later repeats
        let (t2, _) = o.at(1, o.weights(360.0));
        assert!((t2 - t).abs() < 1e-12);
        // mid-June is month 5
        let (t, a) = o.at(1, o.weights(165.0));
        assert!((t - 275.0).abs() < 1e-12 && (a - 5.0 / 11.0).abs() < 1e-12);
        assert_eq!(o.floe, vec![2.0, 1.0]);
    }

    #[test]
    fn rejects_wrong_shapes_and_celsius() {
        assert!(PrescribedOcean::new(&[280.0; 23], None, &[0.0, 0.0], [2.0, 1.0], 1.0).is_err());
        assert!(PrescribedOcean::new(&[20.0; 24], None, &[0.0, 0.0], [2.0, 1.0], 1.0).is_err());
        let o = PrescribedOcean::new(&[280.0; 24], None, &[0.0, 0.0], [2.0, 1.0], 1.0).unwrap();
        assert_eq!(o.at(1, o.weights(0.3)).1, 0.0);
    }
}
