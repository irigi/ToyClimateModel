//! Durations in configs: strings with a unit (`"30min"`, `"1day"`, `"2.4h"`, `"1200s"`) or
//! bare numbers in seconds.

use super::ConfigError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A span of model time, stored in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Duration(pub f64);

impl Duration {
    /// Length in seconds.
    pub fn seconds(self) -> f64 {
        self.0
    }
}

impl FromStr for Duration {
    type Err = ConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        // The unit is the trailing alphabetic suffix, so exponents like "1e3s" work.
        let num = t.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let unit = &t[num.len()..];
        let value: f64 = num
            .trim()
            .parse()
            .map_err(|_| ConfigError::Duration(s.to_string()))?;
        let scale = match unit.trim().to_ascii_lowercase().as_str() {
            "" | "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
            "min" | "mins" | "minute" | "minutes" => 60.0,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
            "d" | "day" | "days" => 86400.0,
            _ => return Err(ConfigError::Duration(s.to_string())),
        };
        if !value.is_finite() || value < 0.0 {
            return Err(ConfigError::Duration(s.to_string()));
        }
        Ok(Duration(value * scale))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f64),
            Str(String),
        }
        match Raw::deserialize(d)? {
            Raw::Num(x) if x.is_finite() && x >= 0.0 => Ok(Duration(x)),
            Raw::Num(x) => Err(serde::de::Error::custom(format!("invalid duration {x}"))),
            Raw::Str(s) => s.parse().map_err(serde::de::Error::custom),
        }
    }
}

impl Serialize for Duration {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.0)
    }
}
