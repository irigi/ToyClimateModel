# Godot integration (M3, done 2026-11-08)

The viewer runs inside the owner's double-precision Godot 4.3 (custom build,
`~/src/godot-4.3-stable`, `~/.local/bin/godot4`). `./run` builds the extension against that
binary and starts it; see README "Viewer" for options. The scene is built in code in
`godot/main.gd` (globe, flat map, orbit camera, UI); shaders in `godot/shaders/`.

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
(godotengine/godot#86346); it needs the API JSON of the exact Godot build. `./run` does this:

```bash
(cd target/godot-api && godot4 --headless --dump-extension-api)   # once per binary
GODOT4_GDEXTENSION_JSON=$PWD/target/godot-api/extension_api.json \
  cargo build -p toyclim-godot --release --no-default-features --features double
cp target/release/libtoyclim_godot.so godot/bin/
```

`api-custom-json` needs no libclang. The default build (`cargo build -p toyclim-godot`)
targets the stock single-precision 4.3 API; that is what CI compiles (`./run --single`).
`godot/toyclim.gdextension` loads `res://bin/libtoyclim_godot.so` for both debug and
release tags, because the Godot editor binary counts as a debug build. Inside the wrapper
all vectors use gdext's `real`, so the same source compiles for both precisions.

Godot changes the working directory to `godot/`; the wrapper resolves relative input paths
in a case (heightmap) next to the config file and its parent folders.

## Alternative: file bridge (SpaceTrains style)

SpaceTrains runs its simulation as a separate process and exchanges JSON files with Godot.
toyclim can do the same without changing the core: a `toyclim bridge` subcommand would own a
`ModelRunner`, write `snapshot.json` + `<field>.f32` + `snapshot.seq` (write-aside and
rename) and poll a `command.json`. Not implemented; worth it only if the GDExtension route
gives trouble with the custom Godot build. The gdext route is preferred: no copies through
files, no process lifetime management.

## Viewer: done and still to do

Done: globe (`field_globe.gdshader`, 30° graticule) and flat map (`field_map.gdshader`),
1D colormaps (viridis-like; RdBu-like with symmetric range for signed fields), orbit
camera, case/field/level selectors, auto or typed range, run/pause/step/speed, diagnostics
and model error, `--screenshot` for checks without a person at the screen. SphereMesh
UVs: UV.x grows eastward from +Z, UV.y = 0 at the north pole, matching the snapshot
images (row 0 north, column 0 at 0°E), so no flip or offset.

Later: wind particles (sample `u`, `v` images), σ value of the selected level, coastline
overlay from `land`/`orog`, cross-section panel, several planets side by side.

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
