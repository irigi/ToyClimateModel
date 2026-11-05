# Progress log

## READ THIS FIRST — night session 2026-11-04/05 summary

Everything is on branch `claude/busy-gauss-sqdo0s`, committed in small steps.
`scripts/check` runs fmt + clippy + all tests (≈ 1 min in release).

**State**
- M0–M2 accepted (numbers below). M3: Rust side done (worker thread + snapshots + gdext
  `ClimateModel`), Godot side not run (your double-precision 4.3 build needs
  `--features double` with the API JSON, see `docs/godot-integration.md`).
- M4 accepted (tests 1–3 pass; JW06 recorded at T85). M5/M6 implemented and smoke-tested at
  T21; the long acceptance runs are in the laptop checklist below.
- M7 (moisture) implemented per your go-ahead; and, being "bold", first steps of M8
  (orbit, gray radiation), M9 (slab ocean, heightmap continents, bucket, ice albedo, q-flux)
  and M11 tools (Köppen, downscaling with lapse rate and Smith–Barstad orographic rain,
  stochastic point rain, layer export). Please confirm or redirect.
- Also: global energy-budget test of the coupled model (closes to 1.1 W/m²), experimental
  diagnostic clouds (opt-in), slab-ocean heat transport.
- Your main deliverables now work end to end on a T21 smoke run of a procedural world
  (`cases/testworld_t21.toml`): hourly station series → rain/wind/temperature
  distributions per calendar month (`tools/station_stats.py --by-month`,
  `tools/point_rain.py`), Köppen maps on the model grid and downscaled to the heightmap
  (`docs/figures/M9/`), exported 16-bit PNG layers.

**Decisions I made that you may want to revisit** (details in the entries below)
1. Global dry-mass fixer in the PE model (needed for M4 test 3; standard in spectral models).
2. JW06 L26 top interface set to σ = 0 (σ model requirement).
3. TJ16 surface moisture flux: kept their tendency (eq. 8), diagnosed the applied flux
   (eq. 10 is inconsistent with eq. 8 by a factor ≈ 2).
4. Physics evaluated at the n−1 leapfrog level; moisture is a passive-ish tracer (no
   virtual temperature in the dynamics).
5. Land = cells with ≥ 50% land in the heightmap; simple bucket; ice/snow albedo is a
   temperature switch (no thermodynamic sea ice yet).
6. Point-rain generator parameters are uncalibrated placeholders.

**Known limitations / honest caveats**
- Smoke runs are short (≤ 455 days) and at T21; their climates are plausible, not tuned.
  The gray model has no clouds (M10), so no cloud feedbacks; grid-point storms appear with
  large-scale condensation.
- The original Williamson and Held–Suarez PDFs I could reach were scans; constants were
  cross-checked against secondary sources (noted per case).

**First things to do on the laptop**: see "Laptop checklist" at the end of this file
(Godot integration first, then the acceptance runs and the T42 world runs).

---

Format per SPEC.md §10.6. Newest entry at the bottom.

## Status

| Milestone | State |
|---|---|
| M0 Skeleton | done (CI to be confirmed on first push) |
| M1 Spectral transforms | done |
| M2 Shallow water | done |
| M3 Godot viewer | Rust side done (runner + gdext wrapper compile); Godot side pending on laptop |
| M4 Primitive equations | done (tests 1–3 pass, test 4 recorded at T85) |
| M5 Held–Suarez / tidally locked | implemented; cloud smoke runs at T21 L10 qualitatively right; acceptance runs (T42 L20, 800–1200 days) pending on laptop |
| M6 Performance/robustness | determinism + restart tests pass; f32 builds; budget met in cloud; viewer PE level selection in Rust API; f32 HS jet check pending (laptop) |
| M7 Moisture | implemented (tracer, fixers, TJ16 moist HS physics, SBM convection); cloud smoke runs OK; T42 runs pending on laptop |
| M8 Radiation/insolation | first step: orbit (Kepler, obliquity, diurnal/daily mean, tidal locking), gray two-stream radiation, calendar |
| M9 Surface | first step: slab ocean, heightmap orography (filtered), land with bucket hydrology, ice/snow albedo switch |
| M10 Clouds | experimental diagnostic RH clouds in the gray scheme (opt-in, untuned) |
| M11 Downscaling/biomes | first step (Python): Köppen, lapse-rate downscaling, Smith–Barstad orographic rain, point-rain generator, layer export |
| M12 Validation | not started |

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

## 2026-11-04 (night) — Stations and M3 (Rust side)

**Stations** (owner-approved SPEC §6 addition): `[[stations]]`, `station_every`,
`station_fields`; bilinear `PointSampler`; series kept in memory and rewritten at each
output step; a restart into the same directory reloads the series. Verified: a run
stopped at day 2 and restarted gives byte-identical station series, fields and
diagnostics to an uninterrupted run (integration test `stations_and_restart_are_bitwise`).
`tools/station_stats.py` prints exceedance frequencies, e.g. W5 at T42, u at 30°N over 4 days:
P(u ≥ 20 m/s) = 0.00 upstream, 0.06 at the summit, 0.36 in the lee (figure
`docs/figures/M2/stations_w5_u.png`).

**M3, Rust side**
- `runtime.rs`: `ModelRunner` (worker thread; commands Run/Pause/Step(n)/SetStepsPerSecond/
  SetFields/SetPublishInterval/SaveRestart/Shutdown; `Mutex<Arc<Snapshot>>` double buffer;
  snapshots carry requested fields regridded to equirectangular f32 with per-level ranges,
  diagnostics, error state). Tests: step/run/pause semantics, pacing.
- `grid::LatLonRegridder` (default 2·N_lon × N_lon, linear in latitude).
- `toyclim-godot`: `ClimateModel` node wrapping the runner (API table in
  `docs/godot-integration.md`), `get_field_image` returns a FORMAT_RF `Image`.
- Owner uses a custom **double-precision Godot 4.3** (read from irigi/SpaceTrains: built with
  `scons precision=double`, editor at `~/.local/bin/godot4`; SpaceTrains talks to it through
  a file bridge, not gdext). gdext 0.5 refuses `double-precision` with the bundled API JSON
  (godot#86346) → feature `double` = `api-custom-json` + `double-precision`; default
  features build against stock 4.3 for CI. irigi/TheLongRun was not accessible from this
  session (no GitHub access), so only SpaceTrains was used as reference.
- `godot/shaders/field_globe.gdshader`: untested sketch.

**Not done (laptop)**: anything that needs a Godot binary.

## 2026-11-04 (night) — M4 dry primitive-equation core

**Done**
- `primitive/vertical.rs`: Lorenz σ levels, Simmons–Burridge α_k, full levels
  ln σ_k = ln σ_{k+1/2} − α_k, matrices G, τ and B = Gτ + R T̄ 1 wᵀ.
- `primitive/mod.rs`: ζ, δ, T', ln p_s; §5.2 tendencies (D_k, σ̇, ω/p, vertical
  advection, Q_k, flux-form temperature advection −∇·(vT') + T'δ); §5.3 semi-implicit
  solve with precomputed S_ℓ⁻¹ (Gauss–Jordan) for Δt and Δt/2; RAW filter; ∇⁸ on ζ, δ, T';
  restart (both levels); output fields u, v, t, vor, div, phi (3D), ps, phis and pressure
  levels t850, u850, v850, t500, phi500, u250, vor850 (linear in ln p).
- Column-physics interface (§5.4) in final shape: `ColumnIn` (lat, lon, column index, σ
  full/half, p_s, u, v, T, tracer block, Φ_s, time, dt), `ColumnOut` (du, dv, dT, tracer
  tendencies, per-scheme diagnostic slots); schemes add to the outputs in a fixed order,
  columns in parallel. Physics is evaluated at the n−1 level (stable for damping).
- **Global dry-mass fixer** (config `mass_fixer`, default true): after each step the ℓ = 0
  coefficient of ln p_s is shifted so ∫p_s dA equals its initial value. Without it the 30-day
  JW06 run drifts by 2.9e-7 (ln p_s models conserve ∫ln p_s, not ∫p_s); with it 1e-15.
  This is a model feature (CAM's Eulerian core has one), not a change of the criterion.
- Performance: per-row column kernel with longitude innermost (was level-innermost:
  23 ms → a few ms per step at T42 L26); levels in parallel for the transforms.

**Discrepancy (paper vs our σ model)**: JW06's L26 top interface is η = 0.002194; our σ
discretization needs σ_{1/2} = 0, so the top interface is set to 0 (top layer slightly
thicker). Everything else follows the paper (constants in `cases/jw06.toml`).

**Measured**
- Test 1, resting isothermal (250 K, T̄ = 300 K), T21 L10, 10 days: max|u| = **0** (< 1e-10).
- Test 2, linear gravity waves, Ω = 0, T21 L10, Δt = 10 min, 30 days, amplitude 0.01 K,
  single vertical mode excited at (ℓ, m = 0):

  | mode | ℓ | ω measured | rel. error vs time-discrete normal mode | vs continuous | amplitude ratio (last/first 5 d) |
  |---|---|---|---|---|---|
  | 0 (external, Lamb) | 3 | 1.8297e-4 s⁻¹ | −1.5e-4 | −4.2e-3 | 0.93 |
  | 1 | 5 | 1.3673e-4 | −8.3e-5 | −2.3e-3 | 0.96 |
  | 2 | 8 | 1.0381e-4 | −5.0e-5 | −1.3e-3 | 0.85 |
  | 4 | 10 | 4.7472e-5 | −7.2e-6 | −2.8e-4 | 0.44 |

  "Time-discrete" = atan(ωΔt)/Δt, the exact frequency of the semi-implicit leapfrog for a
  normal mode of frequency ω = √(Λ_ℓ λ_n). No growth (decay is ∇⁸ + RAW). **Pass** (< 1%).
- Test 3, adiabatic 30 days, JW06 perturbed, T42 L26, Δt 20 min: mass **−9.8e-16** (pass),
  total energy −6.2e-5, axial AM +2.2e-6 (recorded).
- Test 4, JW06 T85 L26, Δt 10 min:
  - unperturbed, 10 days: max |Δ[u]| = **0.141 m/s** (< 0.5: pass); mass 1.4e-15,
    energy −9.3e-9, AM −7.6e-8.
  - perturbed min p_s: day 7 **986.1 hPa**, day 9 **940.8 hPa**, day 11 **922.7 hPa**
    (T42: 986.8 / 949.4 / 923.7). JW06 Fig. 6/7 show the converged day-9 minimum in the
    low 940s hPa; our T85 is in that range. Figures: `docs/figures/M4/jw06_t85_{ps,t850}_day{07,09,11}.png`.
    Small Gibbs ripples from the spectral Φ_s are visible at low latitudes (also noted for
    spectral cores in JW06).
- Performance (cloud, 4 vCPU): JW06 T42 L26 ≈ 38 ms/step on 1 thread, 16 ms on 4 threads;
  T85 L26 ≈ 100 ms/step on 4 threads (11 days in 163 s).

Tests: `resting_atmosphere`, `linear_gravity_wave_external`, `jw_steady_state_short`,
`pe_restart_is_bitwise`, `pe_thread_independent`, `vertical::*` (normal);
`linear_gravity_waves`, `conservation_30_days`, `jw_steady_state_t85` (`--ignored`).

## 2026-11-04 (night) — M5 forcing, online climatology, M6 robustness

**M5 implemented**
- `physics/held_suarez.rs`: `HeldSuarez` (all constants configurable in `[forcing]`) and
  `TidallyLocked` (T_n = 250 K, ΔT_d = 70 K, λ_ss = 180°, cos⁴φ → max(0, cos φ cos(λ−λ_ss))⁴).
  Damping times as durations (`tau_friction = "1day"` etc.). Unknown keys are rejected.
- Cases: `held_suarez.toml` (T42 L20, Δt 20 min, 1200 d), `held_suarez_omega{025,2}.toml`
  (800 d; Ω×2 at T85), `tidally_locked{,_omega01,_omega002}.toml` (T42 L20, 800 d),
  quick-look `*_t21.toml` versions.
- **Online climatology** (`[climatology] start, every, maps`): zonal means and second
  moments `[lev][lat]` (u, v, t, uu, vv, tt, uv, vt and eddy covariances) plus full time-mean
  maps, accumulated after spin-up and written to `out/<run>/climatology/`; continues
  bitwise after a restart. A 1200-day T42 L20 run therefore needs no daily 3D output.
- `tools/zonal_mean.py` (cross sections + all HS acceptance metrics, Hadley-cell edge from
  the mean meridional streamfunction, KE trend), `tools/tidally_locked.py` (maps + metrics).
- Pressure-level fields and global kinetic energy in `diagnostics.csv`.

**Cloud smoke runs (NOT the acceptance configuration)**
- HS T21 L10, 300 days, statistics days 100–300 (481 samples): jets 29.7 m/s (NH) and
  29.3 m/s (SH) near 45–52°; lowest-level [u] −3.9 m/s in the tropics and +3.9 m/s in
  midlatitudes; hemispheric asymmetry 2.2 m/s; Hadley cells to ≈ 36°; KE trend +6.5% over
  200 days (spin-up still visible at day 100). With only 10 levels the jet maximum sits at
  the top full level σ = 0.05 in the NH; the formal criterion needs T42 L20.
  Figure `docs/figures/M5/hs_t21l10_300d_zonal_mean.png` (qualitatively HS94 Fig. 1–2).
- Tidally locked T21 L10, 200 days, statistics days 100–200:

  | Ω/Ω⊕ | equatorial [u] above σ = 0.5 (mean) | T850 day–night | T500 day–night |
  |---|---|---|---|
  | 1 | +8.2 m/s (superrotation) | 7.9 K | 7.1 K |
  | 0.1 | +16.1 m/s | 6.8 K | 2.4 K |
  | 0.02 | +7.4 m/s | 3.8 K | **0.6 K** (weak temperature gradient) |

  At Ω⊕/50 the 850 hPa flow converges on the substellar point (day–night overturning).
  Figures `docs/figures/M5/tl_t21l10_omega{1.0,0.1,0.02}.png`.

**M6**
- Integration test `pe_bitwise_across_threads_and_restart` (normal, ≈ 8 s release): HS-forced
  T21 L10 with stations and climatology, 10 days; output byte-identical for 1 vs 4 threads,
  and for a 6 + 4 day restart (with different thread counts) vs an uninterrupted run.
- `f32` feature: library, headless and test code compile; CI builds f32 and runs W6 for 2 days.
  Precision-specific unit tests assert f64 tolerances and are not meant to pass in f32.
- Budget: HS T42 L20 Δt 20 min = **0.67 model days/s on 4 vCPU** (cloud VM, `nproc` = 4) → one
  model year ≈ 9 min. The SPEC target is ≤ 10 min on 8 threads of the i7-11850H.
- Viewer: `ModelRunner` snapshots carry all levels of 3D fields; `get_field_image(name, level)`.

## 2026-11-04 (night) — M7 moisture (owner go-ahead: "Start M7, let's be bold")

**Done**
- Tracers in the PE core: `PeState.tracers[tracer][lev]` (spectral), flux-form advection
  −∇·(v q) + q δ − σ̇ ∂q/∂σ (same vertical scheme as T), explicit leapfrog, ∇⁸, RAW;
  tracer columns/tendencies through the column-physics interface; restart arrays
  `{prev,curr}_tracer{i}_{k}`.
- **Negative-value fixer** (after every step): negative grid values set to 0 and positive
  values rescaled so the global tracer mass ∫Σq Δσ p_s/g dA is unchanged, then re-analysed.
- **Condensable as data** (`physics/moist.rs::Condensable`; water preset; `[moisture]`
  overrides L, R_v, e₀, T₀) — interface ready for other species (design decision).
- `moist_held_suarez` forcing = Thatcher & Jablonowski (2016) "MITC": HS relaxation with
  T_eq(equator) = 294 K, ΔT_y = 65 K; large-scale condensation (no re-evaporation, rain
  out immediately); time-implicit bulk surface fluxes over the TJ16 SST; boundary-layer
  diffusion of θ and q (implicit tridiagonal, K_E profile with p_pbl = 850 hPa,
  p_strato = 100 hPa); HS Rayleigh friction for momentum. Constants transcribed from the
  paper into `cases/moist_held_suarez.toml`.
- Optional **simplified Betts–Miller convection** (Frierson 2007; `convection = "sbm"`,
  τ = 2 h, RH_ref = 0.7, energy-conserving ΔT closure; shallow-convection variants not
  implemented). Parcel ascent: dry adiabat to saturation, then RK4 moist adiabat.
- Fields: `q`, `rh` (3D), `pw` (precipitable water), physics diagnostics `precip_ls`,
  `precip_conv`, `cape`, `evap`, `shf`, `lhf`; `precip` = sum of `precip_*`. All usable at
  stations (rain statistics) and in the climatology.
- Tools: `tools/moist_stats.py` (zonal P, E, P−E, global budget), `station_stats.py --scale
  3600 --unit mm/h` for rain rates.

**Discrepancy found in TJ16**: eq. (8) adds moisture C_E|v_a|(q_s − q_a)/z_a to the lowest
layer, where z_a is the height of the lowest *full* level (≈ half the layer depth), while
eq. (10) diagnoses E = ρ_a C_E |v_a| (q_s − q_a). The two differ by ρ_a z_a g/Δp ≈ 1/2. We
keep eq. (8) (paper wins for the tendencies) and diagnose the flux actually applied
(Δp/g · Δq/Δt). Before this fix P/E was 2.01; after it **1.006**.

**Deviations**: no virtual-temperature effects in the dynamics; σ = η L26 (JW06) levels.

**Measured** (cloud smoke runs, T21 L26, Δt 20 min, 120 days, statistics days 40–120):

| | TJ16 (LSC only) | + SBM convection |
|---|---|---|
| global precipitation | 1.66 mm/day | 2.01 mm/day |
| global evaporation | 1.65 mm/day | 2.00 mm/day |
| P/E | 1.006 | 1.004 |
| equatorial zonal-mean precipitation | 6.2 mm/day | 6.7 mm/day |
| precipitable water | 20.9 mm | 20.8 mm |
| jets | 51–52 m/s at 30°, σ 0.19 | 54 m/s at 30°, σ 0.16–0.19 |

TJ16 report 2.10 mm/day global-mean precipitation for their MITC and jets of ≈ 65 m/s at
150 hPa (CAM5-SE, higher resolution), so the T21 numbers are plausible.
Station rain statistics (hourly, 80 days), equator / 25°N / 45°N, with SBM:
P(≥ 0.1 mm/h) = 0.39 / 0.06 / 0.38, P(≥ 1 mm/h) = 0.049 / 0.001 / 0.000,
P(≥ 2 mm/h) = 0.011 / 0 / 0, max 4.0 / 1.6 / 1.0 mm/h. These are grid-cell (≈ 600 km at T21)
area means; point statistics need the stochastic downscaling planned for M11.

Tests: `moist::*` (q_sat, implicit diffusion conservation, condensation conserves
c_p T + L q), integration test `moist_bitwise_and_sane` (1 vs 4 threads, restart, rain > 0).
Figures: `docs/figures/M7/`.

## 2026-11-04 (night) — M8/M9 first steps and M11 tools (beyond the M7 go-ahead)

The owner said "Start M7, let's be bold"; I continued along the roadmap into first steps
of M8, M9 and M11 because they are what makes the owner's main deliverables (local weather
statistics, climate/biome maps of a world with continents) possible. Everything is
opt-in by config; the dry and TJ16 configurations are unchanged. **Please confirm or
redirect.**

**M8 (radiation, insolation)**
- `orbit.rs`: Kepler orbit (eccentricity, perihelion), obliquity, declination, (a/r)² flux,
  substellar longitude moving at the synodic rate (fixed if tidally locked; a synchronous
  rotator on an eccentric orbit librates naturally), instantaneous (diurnal) or daily-mean
  insolation, solar day; `Calendar` (year split into named months). Tests: solstice
  declination = obliquity, perihelion/aphelion flux ratio (1 ± e)⁻², daily mean equals
  the average of the diurnal cycle (< 0.5 W m⁻²), Earth solar day 86400 ± 30 s.
- `physics/radiation.rs`: gray two-stream LW after Frierson et al. (2006) (τ₀ = 6 at the
  equator, 1.5 at the poles, f_l = 0.1; optional water-vapour term `tau_q`), SW absorption
  τ_sw = 0.22 σ² on the slant path, surface albedo; exact per-layer integration. Tests:
  column energy budget closes to 1e-6 W m⁻²; isothermal column has OLR = σT⁴.
- Column interface extended (no change for existing schemes): `t_surface`, `insolation`,
  `cos_zenith`, `surface_albedo`, `wetness`, `surface_heat` (accumulated net flux into the
  surface).
- `gray_aquaplanet` forcing: gray radiation + TJ16 moist physics + SBM + Rayleigh drag.
- Calendar written to `meta.json`; monthly climatology maps (`monthly = true`);
  `station_stats.py --by-month`.

**M9 (surface)**
- Slab ocean (`[surface] mixed_layer_depth`, default 20 m) with prognostic T_s from the
  net surface heat flux; land from a heightmap (`[surface] heightmap = "x.npy"`):
  conservative box-average onto Gaussian cells (land fraction, elevation), orography
  Φ_s with a spectral filter exp(−c (ℓ(ℓ+1)/M(M+1))²) (factor 0.1 at ℓ = M), hydrostatic
  initial p_s over mountains, land heat capacity (2e5 J m⁻² K⁻¹), land albedo, bucket
  hydrology (150 mm, β = min(1, W/0.75 W_max), runoff above capacity), ice/snow albedo
  switch (0.6 below 271.15 K over ocean, 273.15 K over land). Cells with land fraction
  ≥ 0.5 are land.
- `tools/make_planet.py`: random spherical-harmonic heightmaps with a target land fraction
  (`worlds/testworld.npy`, seed 3, 30% land).
- Integration test `land_world_runs_and_restarts` (restart bitwise, bucket within bounds).

**M11 tools (Python, post-processing)**
- `koppen.py` (Peel et al. 2007 rules; "summer" = warmest 6-month window per cell, so it
  works for any calendar); unit-checked on Singapore/Cairo/London/Moscow-like climates.
- `downscale.py`: monthly T and P to the heightmap resolution, lapse-rate correction
  (6.5 K/km) against the model orography, optional Smith & Barstad (2004) linear orographic
  precipitation on 40° tiles with monthly 850 hPa winds (checked on a Gaussian ridge:
  windward maximum upstream, dry lee), Köppen map at heightmap resolution.
- `point_rain.py`: stochastic point-rain statistics from grid-cell station series
  (raining fraction 1 − exp(−R/R_c), gamma intensities); **uncalibrated** placeholders.
- `export_layers.py`: 16-bit PNG temperature/precipitation layers and Köppen RGBA/codes
  with a JSON legend.

**Bug fixed on the way**: SBM parcel ascent detected the lifting condensation level only at
sub-step ends, so the parcel profile depended on the sub-step count by up to 0.35 K. Now the
LCL is located by bisection; one RK4 sub-step agrees with 32 to 0.001 K (8× cheaper too).

## 2026-11-05 (night, cont.) — energy budget check, M10 experimental clouds, q-flux

- **Global energy-budget integration test** (`coupled_energy_budget_closes`, T21 L10, gray
  aquaplanet with slab, 30 days, diagnostics every step): d/dt ∫(c_p T + Φ_s + E + L q) dm
  matches ⟨ASR − OLR − F_sfc⟩ within **1.1 W m⁻²** (−22.2 vs −23.3 W m⁻²); with clouds
  −14.5 vs −15.6 W m⁻². The residual is the frictional/diffusive dissipation not returned
  as heat. This validates the signs and bookkeeping of radiation, surface fluxes,
  condensation and the slab together.
- Found and fixed while writing it: the diagnostics set grew after step 0 (`surface_heat`
  appeared only after the first physics call), which would have written rows with more
  columns than the CSV header. Names are now fixed for the whole run and the runner
  refuses a changing set. New diagnostics: `latent_energy` (∫ L q dm), `surface_heat`.
- **M10 (experimental, opt-in)**: diagnostic RH clouds in the gray scheme
  (`clouds = true`; fraction ((RH − 0.8)/0.2)², SW reflection 0.25·c per layer, LW optical
  depth 3·c per layer, `cloud_cover` diagnostic). Untuned: with clouds, set an ocean-like
  `albedo` (≈ 0.1) and expect to tune.
- **Ocean heat transport** for the slab (`ocean_heat_transport = Q0`, Merlis et al. 2013 form,
  φ0 = 16°, zero ocean mean); Q0 = 50 W m⁻² in the T42 world cases.
- Calendar alignment: Earth-like T42 cases start on 1 January (`start_solar_longitude = 280`).
- Note: the T21 smoke runs `gray_t21` and `testworld_t21` were continued from restarts with
  a newer binary (ice–albedo switch and the SBM LCL fix took effect from day 365); their
  numbers are illustrative only.

---

## Laptop checklist (things the cloud session could not do)

1. **Godot integration (early):** follow `docs/godot-integration.md` §"What the Godot side
   still needs" — dump `extension_api.json` from the double-precision editor, build with
   `--features double`, fix `godot/toyclim.gdextension` (compatibility_minimum 4.3), run the
   headless check, then build the globe/map/UI scene.
2. Re-check Williamson test 6 and Galewsky constants against the original papers.
3. **Acceptance runs** (each in the background, `--release`; analyse with the tools):

   ```bash
   cargo build --release
   RAYON_NUM_THREADS=8 target/release/toyclim run --config cases/held_suarez.toml --out out/hs
   python tools/zonal_mean.py out/hs -o docs/figures/M5/hs_t42l20_zonal_mean.png
   for c in tidally_locked tidally_locked_omega01 tidally_locked_omega002; do
     target/release/toyclim run --config cases/$c.toml --out out/$c
     python tools/tidally_locked.py out/$c -o docs/figures/M5/$c.png; done
   for c in held_suarez_omega025 held_suarez_omega2; do
     target/release/toyclim run --config cases/$c.toml --out out/$c
     python tools/zonal_mean.py out/$c -o docs/figures/M5/$c.png; done
   ```

   M5 criteria (jets 26–34 m/s at 38–50°, σ 0.15–0.35; surface easterlies/westerlies;
   asymmetry < 3 m/s; KE trend < 5% over the last 1000 days) are printed by
   `zonal_mean.py`. Also time one model year (`toyclim bench --config cases/held_suarez.toml
   --length 365day`) on 8 threads → M6 budget.
4. M6 f32 check: build with `--features f32` (`cargo build --release -p toyclim-headless
   --features f32 --target-dir target/f32`), rerun HS, compare jets and record the speedup.
5. Compare HS with SpeedyWeather/Isca output placed in `reference/` (and later PlaSim).
6. M7: `cases/moist_held_suarez.toml` (T42 L26, 1080 days, 6-month spin-up as in TJ16), with
   and without `convection = "sbm"`; `tools/moist_stats.py`, `tools/zonal_mean.py`.
7. Worlds (overnight runs, the owner's main use):
   - `cases/gray_aquaplanet.toml` (T42 L26, 10 years) → `moist_stats.py`, `koppen.py`;
   - `cases/testworld.toml` (T42 L26, 4 years, continents from `worlds/testworld.npy`) →
     `koppen.py`, `downscale.py out/testworld worlds/testworld.npy --orographic`,
     `export_layers.py`, `station_stats.py ... --by-month`, `point_rain.py --by-month`;
   - `cases/tidally_locked_gray.toml` (T42 L26, 3 years) → `tidally_locked.py`-style maps
     (needs maps t850/u850/v850 in its climatology block if wanted).

---

## Open questions for the owner

1. ~~License~~ — resolved: MIT (LICENSE added).
2. Owner scoping answers are in `docs/design-decisions.md`. Two of them extend the SPEC and
   need a go-ahead before implementation: station time-series output (proposed for M2,
   SPEC §6 addition) and a procedural terrain milestone (after M9).
