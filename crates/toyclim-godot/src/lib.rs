//! GDExtension wrapper exposing toyclim to Godot (SPEC.md §7, docs/godot-integration.md).
//!
//! `ClimateModel` is a `Node` that owns a [`ModelRunner`] (model on a worker thread). All
//! methods return immediately; field data come from the latest snapshot. Targets the
//! owner's Godot 4.3 double-precision build (features `api-4-3`, `double-precision`).
//!
//! Status: compiles, not yet run inside Godot (first task on the laptop).

// The gdext derive macro expands to `base: base` on newer clippy (1.99+).
#![allow(clippy::redundant_field_names)]

use godot::classes::image::Format;
use godot::classes::Image;
use godot::prelude::*;
use std::sync::Arc;
use toyclim_core::config::Config;
use toyclim_core::runtime::{Command, ModelRunner, Snapshot};

struct ToyclimExtension;

// SAFETY: required by gdext to register the extension entry point; this is the only
// `unsafe` in the workspace and it is confined to this crate.
#[gdextension]
unsafe impl ExtensionLibrary for ToyclimExtension {}

/// A climate model running on a worker thread.
#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct ClimateModel {
    base: Base<Node>,
    runner: Option<ModelRunner>,
    /// Snapshot pinned by `refresh()`, so one frame reads one consistent state.
    snap: Option<Arc<Snapshot>>,
    last_error: GString,
}

#[godot_api]
impl ClimateModel {
    /// Version string of the linked toyclim core.
    #[func]
    fn version(&self) -> GString {
        GString::from(concat!("toyclim ", env!("CARGO_PKG_VERSION")))
    }

    /// Load a TOML case (absolute path or relative to the working directory) and start the
    /// worker, paused. `image_width`/`image_height` ≤ 0 → 2·N_lon × N_lon.
    /// Returns false on error (see `get_last_error`).
    #[func]
    fn load_config(&mut self, path: GString, image_width: i64, image_height: i64) -> bool {
        self.runner = None;
        self.snap = None;
        let result = Config::from_file(std::path::Path::new(&path.to_string()))
            .map_err(|e| e.to_string())
            .and_then(|cfg| {
                let model = toyclim_core::build::build_model(&cfg)?;
                let json = serde_json::to_value(&cfg).map_err(|e| e.to_string())?;
                let size = (image_width > 0 && image_height > 0)
                    .then_some((image_width as usize, image_height as usize));
                Ok(ModelRunner::start(model, json, size))
            });
        match result {
            Ok(r) => {
                self.runner = Some(r);
                self.last_error = GString::new();
                true
            }
            Err(e) => {
                self.last_error = GString::from(e.as_str());
                false
            }
        }
    }

    #[func]
    fn get_last_error(&self) -> GString {
        self.last_error.clone()
    }

    fn send(&self, c: Command) {
        if let Some(r) = &self.runner {
            r.send(c);
        }
    }

    #[func]
    fn run(&mut self) {
        self.send(Command::Run);
    }
    #[func]
    fn pause(&mut self) {
        self.send(Command::Pause);
    }
    /// Take `n` steps, then pause.
    #[func]
    fn step(&mut self, n: i64) {
        self.send(Command::Step(n.max(0) as u64));
    }
    /// Target model steps per wall second while running; 0 = as fast as possible.
    #[func]
    fn set_steps_per_second(&mut self, sps: f64) {
        self.send(Command::SetStepsPerSecond(sps));
    }
    /// Fields carried by snapshots, e.g. ["h", "vor"]. Fewer fields = cheaper snapshots.
    #[func]
    fn set_fields(&mut self, names: PackedStringArray) {
        let v = names.as_slice().iter().map(|s| s.to_string()).collect();
        self.send(Command::SetFields(v));
    }
    #[func]
    fn save_restart(&mut self, dir: GString) {
        self.send(Command::SaveRestart(dir.to_string().into()));
    }

    /// Pin the latest snapshot; call once per frame before the getters below.
    /// Returns true if the snapshot changed since the previous call.
    #[func]
    fn refresh(&mut self) -> bool {
        let Some(r) = &self.runner else { return false };
        let s = r.snapshot();
        let changed = self.snap.as_ref().is_none_or(|old| !Arc::ptr_eq(old, &s));
        self.snap = Some(s);
        changed
    }

    #[func]
    fn get_step(&self) -> i64 {
        self.snap.as_ref().map_or(0, |s| s.step as i64)
    }
    /// Model time in days.
    #[func]
    fn get_time_days(&self) -> f64 {
        self.snap.as_ref().map_or(0.0, |s| s.time / 86400.0)
    }
    #[func]
    fn is_running(&self) -> bool {
        self.snap.as_ref().is_some_and(|s| s.running)
    }
    #[func]
    fn get_steps_per_second(&self) -> f64 {
        self.snap
            .as_ref()
            .map_or(0.0, |s| s.steps_per_second_actual)
    }
    /// Model error (e.g. blow-up), empty if none.
    #[func]
    fn get_model_error(&self) -> GString {
        self.snap
            .as_ref()
            .and_then(|s| s.error.clone())
            .map_or(GString::new(), |e| GString::from(e.as_str()))
    }
    #[func]
    fn get_diagnostics(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(s) = &self.snap {
            for (k, v) in &s.diagnostics {
                d.set(k.as_str(), *v);
            }
        }
        d
    }
    #[func]
    fn get_available_fields(&self) -> PackedStringArray {
        let mut a = PackedStringArray::new();
        if let Some(s) = &self.snap {
            for f in &s.available_fields {
                a.push(f.as_str());
            }
        }
        a
    }
    /// Number of vertical levels of a field in the current snapshot (0 if absent).
    #[func]
    fn get_level_count(&self, name: GString) -> i64 {
        self.field(&name).map_or(0, |f| f.nlev as i64)
    }
    /// (min, max) of a field level in the current snapshot.
    #[func]
    fn get_field_range(&self, name: GString, level: i64) -> Vector2 {
        self.field(&name)
            .and_then(|f| f.range.get(level as usize).copied())
            .map_or(Vector2::ZERO, |(a, b)| Vector2::new(a as real, b as real))
    }
    /// Field level as an equirectangular FORMAT_RF image (row 0 = north, column 0 = 0°E),
    /// ready for `ImageTexture.update()`. Null if the field is not in the snapshot.
    #[func]
    fn get_field_image(&self, name: GString, level: i64) -> Option<Gd<Image>> {
        let f = self.field(&name)?;
        let lev = (level.max(0) as usize).min(f.nlev.saturating_sub(1));
        let bytes: Vec<u8> = f.level(lev).iter().flat_map(|x| x.to_le_bytes()).collect();
        Image::create_from_data(
            f.width as i32,
            f.height as i32,
            false,
            Format::RF,
            &PackedByteArray::from(bytes.as_slice()),
        )
    }
    /// Same data as a flat PackedFloat32Array (for custom processing in GDScript).
    #[func]
    fn get_field_data(&self, name: GString, level: i64) -> PackedFloat32Array {
        match self.field(&name) {
            Some(f) => {
                let lev = (level.max(0) as usize).min(f.nlev.saturating_sub(1));
                PackedFloat32Array::from(f.level(lev))
            }
            None => PackedFloat32Array::new(),
        }
    }
}

impl ClimateModel {
    fn field(&self, name: &GString) -> Option<&toyclim_core::runtime::SnapshotField> {
        let n = name.to_string();
        self.snap.as_ref()?.fields.iter().find(|f| f.name == n)
    }
}
