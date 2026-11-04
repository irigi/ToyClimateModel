# toyclim

A laptop-scale, qualitatively plausible spectral general circulation model for
worldbuilding, including exoplanets (tidally locked, slow and fast rotators).
Rust core, Godot viewer. See [`SPEC.md`](SPEC.md) for the design and milestones and
[`docs/progress.md`](docs/progress.md) for the current state.

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

## Viewer

Install Godot 4.5 or newer, then:

```bash
./run
```

This builds `toyclim-godot` in release mode and opens `godot/`. Set `GODOT=/path/to/godot`
if the binary is not on `PATH`. (Until M3 the viewer only checks that the extension loads.)
