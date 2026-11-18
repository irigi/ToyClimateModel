# toyclim

A laptop-scale, qualitatively plausible spectral general circulation model for
worldbuilding, including exoplanets (tidally locked, slow and fast rotators).
Rust core, Godot viewer. See [`SPEC.md`](SPEC.md) for the design and milestones and
[`docs/progress.md`](docs/progress.md) for the current state, and
[`docs/design-decisions.md`](docs/design-decisions.md) for the owner's scoping decisions.

## Layout

| Path | What |
|---|---|
| `crates/toyclim-core` | numerics, models, physics, I/O (no Godot) |
| `crates/toyclim-headless` | the `toyclim` binary: run cases, benchmarks |
| `crates/toyclim-godot` | GDExtension for the viewer |
| `godot/` | Godot project (viewer) |
| `cases/` | TOML run configs: `core/` (Part I tests), `idealized/` (Held–Suarez family, TJ16), `bench/` (published comparators, e.g. the APE aquaplanet gate), `worlds/`, `scm/` (single-column cases) |
| `tools/` | Python analysis and plotting |
| `docs/` | progress log and acceptance figures |

## Build and test

Requires a recent stable Rust toolchain (tested with 1.97).

```bash
cargo build --release
```

```bash
cargo test --workspace
```

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
```

Long acceptance tests are `#[ignore]`d:

```bash
cargo test --release -- --ignored
```

Single precision build: add `--features toyclim-core/f32`.

## Run a case

```bash
cargo run --release --bin toyclim -- list-cases
```

```bash
cargo run --release --bin toyclim -- run --config cases/core/dummy.toml --out out/dummy
```

Output goes to `out/<run>/`: `meta.json` (grid, planet, config echo, git hash),
`fields/<field>_<step:06>.npy` (float32, Gaussian grid, `[lat][lon]` or
`[lev][lat][lon]`, row 0 = northernmost) and `diagnostics.csv`.

## Single-column model and scorecards

```bash
cargo run --release --bin toyclim -- scm --config cases/scm/dry_land_diurnal.toml --out out/scm/dry_land
```

```bash
python tools/scm_plot.py out/scm/dry_land -o out/scm/dry_land.png
```

A gate run is scored with `tools/scorecard.py <run> --suite aquaplanet` (writes
`docs/scorecards/<milestone>_<suite>_<date>.json`).

## Plot

```bash
pip install -r tools/requirements.txt
```

```bash
python tools/plot_field.py out/dummy analytic --kind map
```

`--kind` is `map`, `polar` or `zonal`; `--step N` picks an output step (default: last).

## Make a world (current state)

```bash
python tools/make_planet.py worlds/myworld.npy --seed 7 --land 0.35
```

Copy `cases/worlds/testworld_t21.toml` (config schema 2, SPEC.md §9.3.2) to
`cases/worlds/myworld.toml`, point `[surface] geography` at your file, and adjust the
system:

- `[star]` (T_eff, luminosity, mass in solar units) and `[orbit]` (`semi_major_axis` in AU,
  eccentricity, obliquity, `spin = "free" | "synchronous" | "resonant"`): the instellation,
  the year and, for locked spins, the rotation rate are derived. Explicit `instellation`,
  `year` or `[planet] rotation_rate` values are accepted only within 1% of the derived ones
  (or with `allow_inconsistent = true`).
- `[planet]` (radius, gravity, `rotation_period` for free rotation, negative = retrograde)
  and `[atmosphere]` (surface pressure, gas composition → R_d and c_p).
- `[[stations]]` where you want weather statistics.

World physics (typed blocks): `[radiation] scheme = "gray"`, `[boundary_layer] scheme =
"kprofile"` (bulk surface fluxes with Louis stability and Charnock roughness, K-profile
boundary layer solved implicitly with the surface energy balance), `[convection] scheme =
"sbm" | "dry" | "none"`, large-scale condensation when `[moisture]` is present, and
`[dynamics] energy_fixer = true`. Near-surface fields `tas`, `huss`, `uas`, `vas`, `sfcwind`,
`gust` are true 2 m / 10 m diagnostics; `t_low`, `u_low`, `v_low`, `wind_low` are the lowest
model level; `psl` is sea-level pressure. `[dynamics] tracer_transport = "semi_lagrangian"`
carries humidity on the grid (the moist world cases use it). Precipitation has an ice phase:
`prsn` is snowfall, `melt` the snow melted on the way down. `[radiation] reflectance` reflects
a fraction of the incoming sunlight at the top (a stand-in for clouds until P4; 0.23 in the
world cases).

Surface (`[surface]`, `[surface.ocean]`, `[surface.land]`, `[surface.sea_ice]`): every cell has
fractions of open ocean, sea ice and land (from the heightmap). Slab ocean or fixed SST;
zero-layer sea ice (`initial_thickness` for an ice-covered start); 5-layer soil, bucket and
snowpack on land; albedos per tile in two bands. Fields: `ts` (cell-mean skin temperature),
`tos` (mixed layer), `sic` (ice concentration of the ocean part), `sit` (ice thickness per
ocean area, m), `snw` (land snow, kg m⁻²), `mrso` / `bucket` (soil water), `mrro` (runoff),
`ice_cap_flux`, `land`. `tools/scorecard.py out/<run>` prints global means and the energy and
water closures of atmosphere and surface.

Settings outside the supported envelope (SPEC.md §9.2 P-5) are logged as warnings. Then:

```bash
cargo run --release --bin toyclim -- run --config cases/worlds/myworld.toml --out out/myworld
```

```bash
python tools/station_stats.py out/myworld precip --scale 3600 --unit mm/h --thresholds 0.1,2,20 --by-month
```

```bash
python tools/koppen.py out/myworld
```

`tools/moist_stats.py` (precipitation/evaporation), `tools/zonal_mean.py` (circulation),
`tools/tidally_locked.py` and `tools/plot_field.py` give more views. Physics available so
far: dry Held–Suarez-type forcings, the TJ16 moist Held–Suarez bundle, and world physics
(gray radiation, K-profile boundary layer with the coupled surface solve, dry adjustment,
simplified Betts–Miller convection, large-scale condensation with ice and snowfall), slab or
fixed-SST ocean with sea ice, land with soil layers, a bucket and snow. See `docs/progress.md` for what is validated and what is not.

## Viewer

Needs Godot 4.3 or newer. By default `./run` targets a **double-precision** Godot build
(the owner's): it dumps that binary's API once, builds `toyclim-godot` against it, copies
the library to `godot/bin/` and opens the viewer. For a stock Godot download use
`./run --single`. The binary is `$GODOT4_BINARY`, else `~/.local/bin/godot4`, else
`godot4`/`godot` on `PATH`.

```bash
./run -- --case=galewsky --field=vor --run
```

Viewer options after `--`: `--case=<name anywhere under cases/, or a path>`, `--field=`, `--level=`,
`--run`, `--sps=<steps per second>`, `--map`, `--screenshot=<png> --after=<seconds>`.
Keys: space run/pause, M map/globe; drag rotates, wheel zooms. Headless check:
`godot4 --headless --path godot --quit-after 600` (after `./run --build-only`).
