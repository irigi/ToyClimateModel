//! toyclim-core: a laptop-scale spectral general circulation model.
//!
//! Units are SI throughout (see `SPEC.md` §3). Model time is always kept in `f64`
//! seconds, independent of the `Real` precision, so long runs do not lose clock accuracy.
#![forbid(unsafe_code)]

pub mod config;
pub mod grid;
pub mod io;
pub mod planet;
pub mod spectral;

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
