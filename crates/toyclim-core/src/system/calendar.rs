//! Calendar for statistics (SPEC.md §9.3.1).

use serde::{Deserialize, Serialize};

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
    use crate::system::orbit::Orbit;

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
