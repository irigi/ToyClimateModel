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
| `cases/` | TOML run configs |
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
cargo run --release --bin toyclim -- run --config cases/dummy.toml --out out/dummy
```

Output goes to `out/<run>/`: `meta.json` (grid, planet, config echo, git hash),
`fields/<field>_<step:06>.npy` (float32, Gaussian grid, `[lat][lon]` or
`[lev][lat][lon]`, row 0 = northernmost) and `diagnostics.csv`.

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

Copy `cases/testworld_t21.toml`, point `[surface] heightmap` at your file, adjust
`[planet]` and `[orbit]` (year, obliquity, eccentricity, instellation, `tidally_locked`), add
`[[stations]]` where you want weather statistics, then:

```bash
cargo run --release --bin toyclim -- run --config cases/myworld.toml --out out/myworld
```

```bash
python tools/station_stats.py out/myworld precip --scale 3600 --unit mm/h --thresholds 0.1,2,20 --by-month
```

```bash
python tools/koppen.py out/myworld
```

`tools/moist_stats.py` (precipitation/evaporation), `tools/zonal_mean.py` (circulation),
`tools/tidally_locked.py` and `tools/plot_field.py` give more views. Physics available so
far: dry Held–Suarez-type forcings, moist physics (large-scale condensation, simplified
Betts–Miller convection, surface fluxes, boundary layer), gray radiation, slab ocean, land
with a bucket model. See `docs/progress.md` for what is validated and what is not.

## Viewer

Needs Godot 4.3 or newer. By default `./run` targets a **double-precision** Godot build
(the owner's): it dumps that binary's API once, builds `toyclim-godot` against it, copies
the library to `godot/bin/` and opens the viewer. For a stock Godot download use
`./run --single`. The binary is `$GODOT4_BINARY`, else `~/.local/bin/godot4`, else
`godot4`/`godot` on `PATH`.

```bash
./run -- --case=galewsky --field=vor --run
```

Viewer options after `--`: `--case=<name in cases/ or path>`, `--field=`, `--level=`,
`--run`, `--sps=<steps per second>`, `--map`, `--screenshot=<png> --after=<seconds>`.
Keys: space run/pause, M map/globe; drag rotates, wheel zooms. Headless check:
`godot4 --headless --path godot --quit-after 600` (after `./run --build-only`).
