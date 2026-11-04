# Progress log

Format per SPEC.md §10.6. Newest entry at the bottom.

## Status

| Milestone | State |
|---|---|
| M0 Skeleton | done (CI to be confirmed on first push) |
| M1 Spectral transforms | done |
| M2 Shallow water | done |
| M3–M6 | not started |

## Versions

- Rust 1.97.0 (cloud session), edition 2021.
- `godot` crate 0.5.5 with feature `api-4-5` (runs on Godot ≥ 4.5). Godot binary not yet
  installed or tested (M3).

## Dependencies beyond the SPEC.md §2 allow-list

None.

---

## 2026-11-04 — M0 skeleton

**Done**

- Cargo workspace with `toyclim-core`, `toyclim-headless` (binary `toyclim`) and
  `toyclim-godot` (cdylib, stub `ClimateModel` node; the only `unsafe` is gdext's
  required `unsafe impl ExtensionLibrary`). Core and headless use `#![forbid(unsafe_code)]`.
- `Real` alias with `f32` feature; model time is always `f64` seconds (`Seconds`) so long
  runs keep clock accuracy in f32 builds.
- `config.rs`: TOML config with duration strings (`"30min"`, `"1day"`, `"2.4h"`, `"1e3s"`,
  bare numbers = seconds), planet preset + overrides (`rotation_factor` as an alternative
  to `rotation_rate`), diffusion and RAW-filter blocks with SPEC defaults,
  `deny_unknown_fields` so typos fail loudly. The SPEC §6 example parses in a unit test.
- `io/npy.rs`: .npy v1.0 writer/reader (`<f4`, `<f8`, C order) with round-trip and
  dtype-mismatch tests. `io/meta.rs`: `meta.json` writer (grid, σ, planet, config echo,
  git hash embedded at build time by `build.rs`).
- `spectral/gauss.rs` + `grid.rs`: Gaussian latitudes/weights (Newton from Tricomi
  guesses), grid sizes from the §4.1 table (with a fallback rule for other T), global
  integral. This already covers M1 test 1.
- `toyclim run` (dummy model: f = sin φ + cos²φ cos 3(λ − Ωt)), `toyclim list-cases`;
  `toyclim bench` is a stub until M2.
- `tools/plot_field.py` (map / polar / zonal), `tools/requirements.txt`.
- `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, tests, a smoke run and a plot.
- Godot project skeleton (`godot/`, `toyclim.gdextension`) and `run` script.

**Measured**

- Σ w_j − 2 < 1e-14 for N_lat ∈ {1, 2, 3, 32, …, 256}; μ^n integrated to < 1e-13 for
  n ≤ 2N_lat − 1, N_lat ∈ {32, 64, 128, 256}.
- Sphere area via the grid integral at T42: relative error < 1e-14.
- `cargo test --workspace`: 13 tests pass (12 core unit, 1 headless end-to-end).
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`: clean.
- `cargo build -p toyclim-core --features f32`: builds.

**Figures**: `docs/figures/M0/dummy_analytic_{map,polar,zonal}.png`.

**Open questions**: see below.

**Next step**: M1 — Legendre functions (§4.2), packing, FFT wrapper, transforms and
operators, tests 2–6, criterion benchmarks.

## 2026-11-04 (night) — M1 spectral transforms

**Done**: `spectral/{packing,legendre,fft,transform,ops}.rs` per §4.1–4.4.
- Triangular m-major packing (`Layout::idx`), P̄ and H tables in f64 cast to `Real`,
  stored per m as `[j][ℓ−m]` for the northern half only (equatorial symmetry).
- `Transform`: synthesis/analysis, generic Legendre kernels with P̄ and H terms (so U/V,
  div/curl and gradients share one code path), `grad`, `uv_from_psi_chi`,
  `uv_from_vort_div`, `div_curl`, global integral. Parallel over m (Legendre) and rows
  (FFT); fixed summation order → bitwise identical across thread counts (tested).
- `realfft` 3.5 (allowed), `rayon` (allowed), `criterion` 0.5 dev-dependency (allowed).
- `util::SplitMix64`: deterministic RNG for tests and perturbations (no `rand` dependency).
- Clippy `unnecessary_cast` is allowed crate-wide in core: with the `Real` alias, `as f64`
  is a no-op in f64 builds but required in f32 builds.

**Measured** (f64, max relative errors):

| Test | Result | Criterion |
|---|---|---|
| 1 Σw − 2, μⁿ | < 1e-14 / < 1e-13 | 1e-14 / 1e-13 |
| 2 orthonormality T170 (m = 0,1,2,17,85,120,169,170) | ≤ 2.4e-14 | 1e-13 |
| 3 round trip T21/T42/T85/T170 | 2.1e-15 / 9.4e-15 / 2.9e-14 / 7.6e-14 | 1e-12 |
| 4 Laplacian via grad+div (6 (ℓ,m) at T42) | ≤ 7.7e-15 | 1e-10 |
| 5 wind round trip T21/T42/T85 (div, curl) | 3.2e-15 / 6.8e-15 / 3.3e-14 | 1e-10 |
| 6 solid-body rotation T42 | 2.3e-16 | 1e-12 |

Benchmarks (`cargo bench -p toyclim-core --bench transforms`, cloud VM, 4 vCPU):

| | 1 thread | 4 threads |
|---|---|---|
| synthesis T42 | 94 µs | 103 µs |
| analysis T42 | 132 µs | 107 µs |
| synthesis T85 | 343 µs | 183 µs |
| analysis T85 | 664 µs | 237 µs |

**Next**: M2 shallow water.

## 2026-11-04 (night) — M2 shallow-water model

**Done**
- `shallow_water/`: ζ, δ, φ' spectral state; semi-implicit leapfrog per §5.1, RAW filter
  (skipped on the forward first step), implicit ∇⁸ hyperdiffusion after each step
  (first step uses the Δt factor, leapfrog steps 2Δt); conservation diagnostics (mass,
  energy, potential enstrophy, axial AM); restart state (both levels) and bitwise restart test.
- `cases.rs`: Williamson 2 (both α), 5, 6, Galewsky (perturbed/unperturbed), rest.
  Configs in `cases/` with citations.
- `model.rs`: `Model` trait shared by the runner and (later) the viewer.
  `build.rs`: model from config. `io/restart.rs`: restart dirs (`.npy` re/im + JSON).
- `toyclim run` now runs real models: fields + `diagnostics.csv` at every output step, CFL in
  the log, NaN/Inf → `restart_crash/` dump and error exit, final `restart/`;
  `--restart DIR` continues a run, `--length` overrides the run length;
  `toyclim bench` reports model days per wall second.
- Performance: serial fast path for FFT and Legendre on grids below 128×256 (rayon overhead
  dominated at T42); even/odd paired kernels without bounds checks.

**Discrepancy with SPEC.md (paper wins)**: Williamson test 2 with α ≠ 0 rotates the Coriolis
parameter with the flow, f = 2Ω(−cos λ cos φ sin α + sin φ cos α); with the unrotated f the
state is not steady (l2 = 0.23 at day 5). Implemented as in the paper via an optional
Coriolis field in `SwInitial`.

**Papers**: the Williamson report on OSTI is a scanned PDF without a text layer and the Galewsky
PDF host returned 503. Test 2 and 5 constants were checked against the restatement in
Shipton et al. (arXiv:1707.00855); test 6 and Galewsky use the SPEC.md values (the standard
ones). To re-check against the originals on the laptop.

**Measured** (f64, Earth constants)

Williamson 2, 5 days, Δt = 30 min (normalized h errors; α = 0 / α = π/2 − 0.05 identical to 3 digits):

| T | l1 | l2 | l∞ | mass drift |
|---|---|---|---|---|
| 21 | 2.2e-7 | 2.6e-7 | 5.6e-7 | 7e-16 |
| 42 | 9.4e-10 | 1.1e-9 | 2.4e-9 | 7e-16 |
| 85 | 3.5e-12 | 4.1e-12 | 8.9e-12 | 9e-16 |

(The error is the hyperdiffusion acting on ℓ = 2; it shrinks with M because the damping is
scaled by [ℓ(ℓ+1)/M(M+1)]⁴.) Criterion l2 < 1e-5: **pass**.

Williamson 5, 15 days: T42 (Δt 20 min) mass 4e-16, energy 2.1e-6, enstrophy 1.1e-4;
T85 (Δt 10 min) mass 0, energy **3.1e-7** (< 1%: pass), enstrophy 1.3e-5. Axial AM changes
by 0.79% (mountain torque, expected). Stable: pass.

Williamson 6, T42, Δt 20 min, 14 days: balance cross-check 9.6e-14 (< 1e-8: pass); wave 4
retained (|ζ_5^4| is 8× the largest other non-zonal coefficient); energy drift 1.3e-5,
enstrophy drift 2.0e-3 (< 1%: pass); **phase speed 11.28 °/day** vs nondivergent 12.20
(−7.5%, within 15%: pass; divergent RH waves are known to be slower than the
nondivergent formula).

Galewsky T85, Δt 8 min: unperturbed max|Δζ|/max|ζ| at day 6 = **8.1e-3** (< 1%: pass);
perturbed: rolled up into vortices by day 6 (figure), 4–5 vortices between 0 and 360°E,
qualitatively as Galewsky Fig. 4.

Performance (`toyclim bench`, cloud VM, 4 vCPU): T42 SW Δt 20 min: **14.0 model days/s**
single-threaded (≈ 21 at Δt 30 min; criterion ≥ 10: pass), 13.7 with 4 threads (too
small to parallelize). T85 Δt 8 min: 1.05 (1 thread), 1.29 (4 threads) model days/s.

Tests: `williamson2_t42`, `williamson5_t42`, `williamson6_t42`, `rossby_haurwitz_balance`,
`rest_stays_at_rest`, `restart_is_bitwise` (normal); `williamson2_convergence`,
`williamson5_t85`, `galewsky_unperturbed_t85` (`--ignored`, each < 10 s in release).

**Figures** (`docs/figures/M2/`): `w5_t85_eta_day{05,10,15}.png`, `w6_t42_h_day{0,14}.png`,
`galewsky_t85_vor_day6.png`.

---

## Open questions for the owner

1. ~~License~~ — resolved: MIT (LICENSE added).
2. Owner scoping answers are in `docs/design-decisions.md`. Two of them extend the SPEC and
   need a go-ahead before implementation: station time-series output (proposed for M2,
   SPEC §6 addition) and a procedural terrain milestone (after M9).
