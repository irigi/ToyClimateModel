//! Spectral transform machinery (SPEC.md §4).

pub mod fft;
pub mod gauss;
pub mod legendre;
pub mod ops;
pub mod packing;
pub mod transform;

#[cfg(test)]
mod tests;

pub use packing::{Complex, Layout};
pub use transform::Transform;
