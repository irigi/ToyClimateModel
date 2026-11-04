//! toyclim-core: a laptop-scale spectral general circulation model.
//!
//! Units are SI throughout (see `SPEC.md` §3). Model time is always kept in `f64`
//! seconds, independent of the `Real` precision, so long runs do not lose clock accuracy.
#![forbid(unsafe_code)]
// Casts like `x as f64` are no-ops in the default build but needed with `--features f32`.
#![allow(clippy::unnecessary_cast)]

pub mod build;
pub mod config;
pub mod grid;
pub mod io;
pub mod model;
pub mod planet;
pub mod shallow_water;
pub mod spectral;
pub mod time;
pub mod util;

pub use spectral::{Complex, Layout, Transform};

/// Floating-point type used for model fields. `f64` by default; the `f32` cargo feature
/// switches it.
#[cfg(not(feature = "f32"))]
pub type Real = f64;
/// Floating-point type used for model fields. `f64` by default; the `f32` cargo feature
/// switches it.
#[cfg(feature = "f32")]
pub type Real = f32;

/// Model time in seconds. Always `f64`.
pub type Seconds = f64;
