//! JSON metadata sidecars (`meta.json`).

use crate::grid::GaussianGrid;
use crate::planet::Planet;
use serde::Serialize;
use std::path::Path;

/// Contents of `out/<run>/meta.json`.
#[derive(Debug, Serialize)]
pub struct RunMeta<'a> {
    /// Crate version of toyclim-core.
    pub toyclim_version: &'static str,
    /// Git commit of the build, or "unknown".
    pub git_hash: &'a str,
    /// "f64" or "f32".
    pub precision: &'static str,
    pub model: &'a str,
    pub truncation: usize,
    /// Grid description (μ, weights, latitudes and longitudes in radians).
    pub grid: &'a GaussianGrid,
    /// Full σ levels (empty for 2D models).
    pub sigma: &'a [f64],
    pub planet: &'a Planet,
    /// Seconds of model time between output steps.
    pub output_interval_s: f64,
    /// Echo of the parsed configuration.
    pub config: serde_json::Value,
    /// Calendar for statistics (from the orbit), if any.
    pub calendar: Option<crate::orbit::Calendar>,
}

impl RunMeta<'_> {
    /// Precision string of this build.
    pub fn precision() -> &'static str {
        if std::mem::size_of::<crate::Real>() == 4 {
            "f32"
        } else {
            "f64"
        }
    }

    /// Write pretty-printed JSON to `path`.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, s)
    }
}
