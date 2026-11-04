//! GDExtension wrapper exposing toyclim to the Godot viewer (SPEC.md §7).
//!
//! M0 stub: registers a `ClimateModel` node so the Godot project can load the
//! extension. The worker thread, snapshots and regridding arrive in M3.

// The gdext derive macro expands to `base: base` on newer clippy (1.99+).
#![allow(clippy::redundant_field_names)]

use godot::prelude::*;

struct ToyclimExtension;

// SAFETY: required by gdext to register the extension entry point; this is the only
// `unsafe` in the workspace and it is confined to this crate.
#[gdextension]
unsafe impl ExtensionLibrary for ToyclimExtension {}

/// Owns a toyclim model (from M3 on, on a worker thread).
#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct ClimateModel {
    base: Base<Node>,
}

#[godot_api]
impl ClimateModel {
    /// Version string of the linked toyclim core.
    #[func]
    fn version(&self) -> GString {
        GString::from(concat!("toyclim ", env!("CARGO_PKG_VERSION")))
    }
}
