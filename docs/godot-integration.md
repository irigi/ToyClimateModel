# Godot integration (M3, Rust side done; Godot side pending)

Status (2026-11-04): the engine-side interface is implemented and tested in Rust; the
GDExtension wrapper compiles against the stock Godot 4.3 API, but **nothing has been run in
Godot yet**. Integration with the owner's custom double-precision Godot is the first laptop
task (see "Laptop checklist" in `docs/progress.md`).

## Layers

```
Godot (GDScript UI, shaders)
   │  ClimateModel node (toyclim-godot, gdext)          ← thin, no logic
   │     refresh() / get_field_image() / run() ...
   ▼
toyclim_core::runtime::ModelRunner                       ← worker thread, pacing, snapshots
   │  Command channel ─────►  model thread  ─────► Mutex<Arc<Snapshot>> (double buffer)
   ▼
toyclim_core::model::Model (ShallowWater, Primitive …)   ← numerics
```

The UI never blocks on the model: `refresh()` swaps in the latest published snapshot
(an `Arc` clone under a very short lock). The model thread publishes at most every
`publish_interval` (default 1/30 s) while running, and after every command.

Snapshots carry only the fields requested with `set_fields`, regridded from the Gaussian
grid to an equirectangular image (default 2·N_lon × N_lon, row 0 at the north edge, column 0
at 0°E, bilinear). For 3D fields all levels are included; pick one with `level`.

## `ClimateModel` API (GDScript view)

| Method | Meaning |
|---|---|
| `version() -> String` | |
| `load_config(path, image_w = 0, image_h = 0) -> bool` | build the model from a TOML case, start the worker paused; ≤ 0 sizes → default |
| `get_last_error() -> String` | error of the last `load_config` |
| `run()`, `pause()` | continuous stepping on/off |
| `step(n)` | n steps, then pause |
| `set_steps_per_second(sps)` | pacing; 0 = as fast as possible |
| `set_fields(PackedStringArray)` | fields carried by snapshots |
| `save_restart(dir)` | restart directory; result in a later snapshot (`last_save`, not yet exposed) |
| `refresh() -> bool` | pin the latest snapshot (call once per frame); true if new |
| `get_step()`, `get_time_days()`, `is_running()`, `get_steps_per_second()` | |
| `get_model_error() -> String` | e.g. blow-up message; the model stops itself |
| `get_diagnostics() -> Dictionary` | mass, energy, …, `max_courant` |
| `get_available_fields() -> PackedStringArray` | all fields the model offers |
| `get_level_count(name) -> int` | levels of a snapshot field |
| `get_field_range(name, level) -> Vector2` | (min, max) for colour scaling |
| `get_field_image(name, level) -> Image` | FORMAT_RF, ready for `ImageTexture.update()` |
| `get_field_data(name, level) -> PackedFloat32Array` | same data, flat |

Example frame loop:

```gdscript
@onready var model := ClimateModel.new()
var tex: ImageTexture

func _ready():
    add_child(model)
    if not model.load_config(ProjectSettings.globalize_path("res://../cases/galewsky.toml"), 0, 0):
        push_error(model.get_last_error())
    model.set_fields(PackedStringArray(["vor", "h"]))
    model.set_steps_per_second(0)
    model.run()

func _process(_dt):
    if model.refresh():
        var img := model.get_field_image("vor", 0)
        if img:
            if tex == null: tex = ImageTexture.create_from_image(img)
            else: tex.update(img)
            var r := model.get_field_range("vor", 0)
            $Globe.material_override.set_shader_parameter("field", tex)
            $Globe.material_override.set_shader_parameter("vmin", r.x)
            $Globe.material_override.set_shader_parameter("vmax", r.y)
```

## Building for the owner's Godot (4.3, double precision)

gdext's `double-precision` feature cannot use the bundled API descriptions
(godotengine/godot#86346); it needs the API JSON of the exact Godot build:

```bash
godot4 --headless --dump-extension-api        # in some scratch dir → extension_api.json
```

```bash
GODOT4_GDEXTENSION_JSON=/path/to/extension_api.json \
  cargo build -p toyclim-godot --release --no-default-features --features double
```

The default build (`cargo build -p toyclim-godot`) targets the stock single-precision 4.3
API; that is what CI compiles. `godot/toyclim.gdextension` sets
`compatibility_minimum = 4.5` from M0 — **change it to 4.3** when integrating (left as is
until it can be tested). Inside the wrapper all vectors use gdext's `real`, so the same
source compiles for both precisions.

## Alternative: file bridge (SpaceTrains style)

SpaceTrains runs its simulation as a separate process and exchanges JSON files with Godot.
toyclim can do the same without changing the core: a `toyclim bridge` subcommand would own a
`ModelRunner`, write `snapshot.json` + `<field>.f32` + `snapshot.seq` (write-aside and
rename) and poll a `command.json`. Not implemented; worth it only if the GDExtension route
gives trouble with the custom Godot build. The gdext route is preferred: no copies through
files, no process lifetime management.

## What the Godot side still needs (plan)

1. Build with `--features double` against the owner's API JSON; fix
   `toyclim.gdextension` (`compatibility_minimum = 4.3`, library paths).
2. `godot --headless --path godot --quit-after 600` with a script that loads
   `cases/williamson6.toml`, runs, and prints step/time each second → M3 acceptance.
3. Scene: `Globe` = `MeshInstance3D` with `SphereMesh` (radial segments 128, rings 64) and a
   `ShaderMaterial`; `godot/shaders/field_globe.gdshader` (provided, untested) samples the
   field texture by UV (the sphere's UV is equirectangular; check the u = 0 seam and
   orientation: u = 0 should be 0°E, v = 0 north), a 1D colormap texture
   (`GradientTexture1D`), `vmin`/`vmax` uniforms.
4. Flat map: a `TextureRect` with the same shader in `canvas_item` mode or a second
   material; toggle visibility.
5. Orbit camera: `Camera3D` on a pivot `Node3D`; mouse drag rotates the pivot, wheel zooms.
6. UI (`Control`, GDScript only): field `OptionButton` from `get_available_fields()`, level
   `SpinBox` (`get_level_count`), range mode auto/fixed, run/pause, steps-per-second slider,
   step button, labels for time, mass, energy, max Courant, model error.
7. Later: wind particles (sample `u`, `v` images), cross-section panel, several planets.

Fields per model (`get_available_fields()` lists what a given configuration offers):
- shallow water: `h, eta, hs, vor, div, u, v, pv`;
- primitive equations, 3D (one level per σ level, level 0 = top): `u, v, t, vor, div, phi`,
  and with moisture `q, rh`;
- primitive equations, 2D: `ps, phis, orog, tas, t850, u850, v850, t500, phi500, u250,
  vor850`; with moisture `pw, precip` (+ `precip_ls`, `precip_conv`, `cape`, `evap`, `shf`,
  `lhf`); with gray radiation `olr, asr, sw_sfc_down, lw_sfc_down`; with a surface model
  `ts, surface_heat, land, bucket`.

Good default layers for a worldbuilding view: `ts` or `tas` (temperature), `precip`
(×3600 → mm/h), `pw` (moisture), `u850`/`v850` (winds, also for particles), `ps`,
`orog` + `land` (static overlay).
