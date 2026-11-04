//! Orbit, insolation geometry and calendar (SPEC.md §5.5, M8).
//!
//! Time t = 0 is the northern vernal equinox (solar longitude L_s = 0) unless
//! `start_solar_longitude` says otherwise. The substellar point moves west at the
//! synodic rate; for a tidally locked planet it is fixed.

use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

/// Orbital and stellar parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orbit {
    /// Orbital period \[s\].
    pub year: f64,
    pub eccentricity: f64,
    /// Obliquity \[rad\].
    pub obliquity: f64,
    /// Solar longitude of perihelion ϖ \[rad\] (Earth ≈ 283°).
    pub perihelion_longitude: f64,
    /// Solar longitude at t = 0 \[rad\].
    pub start_solar_longitude: f64,
    /// Stellar flux at the semi-major axis \[W m⁻²\].
    pub instellation: f64,
    /// Tidally locked: substellar longitude fixed at `substellar_lon`.
    pub tidally_locked: bool,
    /// Substellar longitude at t = 0 (or always, if tidally locked) \[rad\].
    pub substellar_lon: f64,
    /// Resolve the diurnal cycle (instantaneous zenith angle); false = daily means.
    pub diurnal: bool,
    /// Sidereal rotation rate Ω \[s⁻¹\] (copied from the planet).
    pub rotation_rate: f64,
}

/// Sun position at one time.
#[derive(Debug, Clone, Copy)]
pub struct SunPosition {
    /// Solar longitude L_s \[rad\].
    pub solar_longitude: f64,
    /// Declination δ \[rad\].
    pub declination: f64,
    /// Flux at the planet \[W m⁻²\] (instellation × (a/r)²).
    pub flux: f64,
    /// Substellar longitude \[rad\].
    pub substellar_lon: f64,
}

impl Orbit {
    /// Earth-like default orbit for a planet rotating at `rotation_rate`.
    pub fn earth_like(rotation_rate: f64) -> Self {
        Orbit {
            year: 365.25 * 86400.0,
            eccentricity: 0.0167,
            obliquity: 23.44f64.to_radians(),
            perihelion_longitude: 282.9f64.to_radians(),
            start_solar_longitude: 0.0,
            instellation: 1361.0,
            tidally_locked: false,
            substellar_lon: PI,
            diurnal: true,
            rotation_rate,
        }
    }

    fn anomalies_from_solar_longitude(&self, ls: f64) -> f64 {
        // true anomaly ν = L_s − ϖ → mean anomaly
        let e = self.eccentricity;
        let nu = ls - self.perihelion_longitude;
        let ecc = 2.0 * (((1.0 - e) / (1.0 + e)).sqrt() * (nu / 2.0).tan()).atan();
        ecc - e * ecc.sin()
    }

    /// Solar longitude and r/a at time t.
    fn kepler(&self, t: f64) -> (f64, f64) {
        let e = self.eccentricity;
        let m0 = self.anomalies_from_solar_longitude(self.start_solar_longitude);
        let m = m0 + TAU * t / self.year;
        let mut ecc = m;
        for _ in 0..30 {
            let d = (ecc - e * ecc.sin() - m) / (1.0 - e * ecc.cos());
            ecc -= d;
            if d.abs() < 1e-14 {
                break;
            }
        }
        let nu = 2.0
            * ((1.0 + e).sqrt() * (ecc / 2.0).sin()).atan2((1.0 - e).sqrt() * (ecc / 2.0).cos());
        (
            (nu + self.perihelion_longitude).rem_euclid(TAU),
            1.0 - e * ecc.cos(),
        )
    }

    /// Sun position at time t \[s\].
    pub fn sun(&self, t: f64) -> SunPosition {
        let (ls, r) = self.kepler(t);
        let declination = (self.obliquity.sin() * ls.sin()).asin();
        let substellar_lon = if self.tidally_locked {
            self.substellar_lon
        } else {
            // planet turns east at Ω; the sun advances along its orbit by L_s(t) − L_s(0)
            (self.substellar_lon - self.rotation_rate * t + (ls - self.start_solar_longitude))
                .rem_euclid(TAU)
        };
        SunPosition {
            solar_longitude: ls,
            declination,
            flux: self.instellation / (r * r),
            substellar_lon,
        }
    }

    /// Length of the solar day \[s\] (infinite for a tidally locked planet).
    pub fn solar_day(&self) -> f64 {
        let n = TAU / self.year;
        let syn = self.rotation_rate - n;
        if self.tidally_locked || syn == 0.0 {
            f64::INFINITY
        } else {
            TAU / syn.abs()
        }
    }

    /// Top-of-atmosphere insolation \[W m⁻²\] at (lat, lon): instantaneous if `diurnal`
    /// (or tidally locked), otherwise the daily mean.
    pub fn insolation(&self, sun: &SunPosition, lat: f64, lon: f64) -> f64 {
        let (sd, cd) = (sun.declination.sin(), sun.declination.cos());
        if self.diurnal || self.tidally_locked {
            let mu = lat.sin() * sd + lat.cos() * cd * (lon - sun.substellar_lon).cos();
            sun.flux * mu.max(0.0)
        } else {
            let x = (-lat.tan() * sun.declination.tan()).clamp(-1.0, 1.0);
            let h0 = x.acos();
            sun.flux / PI * (h0 * lat.sin() * sd + lat.cos() * cd * h0.sin())
        }
    }

    /// Cosine of the stellar zenith angle (instantaneous; for daily means the
    /// insolation-weighted value) — used for the slant path of shortwave absorption.
    pub fn cos_zenith(&self, sun: &SunPosition, lat: f64, lon: f64) -> f64 {
        if self.diurnal || self.tidally_locked {
            (lat.sin() * sun.declination.sin()
                + lat.cos() * sun.declination.cos() * (lon - sun.substellar_lon).cos())
            .max(0.0)
        } else {
            0.5
        }
    }
}

/// A calendar for statistics: a year (the orbital period) split into named months of
/// equal length, starting at t = 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    /// Year length \[s\].
    pub year: f64,
    /// Solar day \[s\] (informational; may be infinite → null in JSON).
    pub solar_day: Option<f64>,
    pub months: Vec<String>,
}

impl Calendar {
    pub fn new(year: f64, solar_day: f64, months: Vec<String>) -> Self {
        let months = if months.is_empty() {
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        } else {
            months
        };
        Calendar {
            year,
            solar_day: solar_day.is_finite().then_some(solar_day),
            months,
        }
    }

    /// Month index of time t.
    pub fn month(&self, t: f64) -> usize {
        let f = (t / self.year).rem_euclid(1.0);
        ((f * self.months.len() as f64) as usize).min(self.months.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circular_orbit_declination_and_flux() {
        let mut o = Orbit::earth_like(7.292e-5);
        o.eccentricity = 0.0;
        // quarter year after the equinox: northern solstice
        let s = o.sun(o.year / 4.0);
        assert!((s.declination - o.obliquity).abs() < 1e-9);
        assert!((s.flux - 1361.0).abs() < 1e-9);
        assert!(o.sun(0.0).declination.abs() < 1e-12);
    }

    #[test]
    fn kepler_perihelion_flux() {
        let o = Orbit::earth_like(7.292e-5);
        // flux varies by (1±e)^-2 over the year
        let fl: Vec<f64> = (0..365).map(|d| o.sun(d as f64 * 86400.0).flux).collect();
        let (mx, mn) = (
            fl.iter().cloned().fold(0.0, f64::max),
            fl.iter().cloned().fold(1e9, f64::min),
        );
        assert!((mx / 1361.0 - 1.0 / (1.0 - 0.0167f64).powi(2)).abs() < 1e-3);
        assert!((mn / 1361.0 - 1.0 / (1.0 + 0.0167f64).powi(2)).abs() < 1e-3);
    }

    #[test]
    fn daily_mean_matches_average_of_diurnal() {
        let mut o = Orbit::earth_like(7.292e-5);
        o.eccentricity = 0.0;
        let s = o.sun(o.year / 4.0);
        for lat in [-1.2f64, -0.3, 0.0, 0.5, 1.3] {
            let n = 2000;
            let mean: f64 = (0..n)
                .map(|i| o.insolation(&s, lat, TAU * i as f64 / n as f64))
                .sum::<f64>()
                / n as f64;
            let mut od = o.clone();
            od.diurnal = false;
            let dm = od.insolation(&s, lat, 0.0);
            assert!((mean - dm).abs() < 0.5, "lat {lat}: {mean} vs {dm}");
        }
    }

    #[test]
    fn solar_day_and_calendar() {
        let o = Orbit::earth_like(7.292115e-5);
        assert!((o.solar_day() - 86400.0).abs() < 30.0, "{}", o.solar_day());
        let c = Calendar::new(o.year, o.solar_day(), vec![]);
        assert_eq!(c.month(0.0), 0);
        assert_eq!(c.month(o.year * 0.99), 11);
        assert_eq!(c.month(o.year * 1.5), 6);
    }
}
