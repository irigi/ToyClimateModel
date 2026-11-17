# Progress log

## READ THIS FIRST — plan revised 2026-11-12

**The plan is `SPEC.md`.** Part I (M0–M6) is accepted. Part II (§9) defines Phase 2,
milestones **P0–P11**, and replaces the old sketch of M7–M12. Continue at the lowest unmet
milestone in SPEC.md §9.6 — currently **P1** (P0 done 2026-11-15; P1 blocked on one APE gate row, question 6; owner decision open, question 3).

- The physics, surface and tools built on 2026-11-04/05 as "M7–M11 first steps" are a
  **prototype** (SPEC.md §9.1): they run and conserve energy and water, but have no
  acceptance criteria and the T42 world runs show defects (grid-point storms over tropical
  land, Dsa-dominated mid-latitude continents, ice-cap climate over open ocean from 55°S,
  latitude-prescribed optical depth). Entries below that describe them are history, not
  accepted results.
- P0–P9 are authorised by the owner; P10 (procedural terrain) and P11 (extended envelope)
  need a go-ahead.
- `scripts/check` runs fmt + clippy + all tests (≈ 1 min in release).

---

Format per SPEC.md §10 item 10. Newest entry at the bottom.

## Status

| Milestone | State |
|---|---|
| M0 Skeleton | done (CI to be confirmed on first push) |
| M1 Spectral transforms | done |
| M2 Shallow water | done |
| M3 Godot viewer | done 2026-11-08 on the laptop: headless check passes in the owner's double-precision Godot 4.3; globe/map/UI viewer |
| M4 Primitive equations | done (tests 1–3 pass, test 4 recorded at T85) |
| M5 Held–Suarez / tidally locked | done 2026-11-12: HS T42 L20 1200 d meets all jet criteria; tidally locked and rotation sweeps recorded |
| M6 Performance/robustness | budget met on the laptop (1 model year in 290 s on 8 threads); f32 HS: jets OK, hemispheric asymmetry 3.19 m/s > 3 (open question 3) |
| M7–M11 "first steps" (moisture, gray radiation, slab/land, clouds, Python tools) | **prototype**, superseded by Phase 2 (SPEC.md §9.1); numbers in the entries below are history |
| P0 Groundwork (refactor, config v2, regress script, f32 question) | done 2026-11-15 (branch `p0-groundwork`): regress byte-identical, derivations tested, files < 700 lines; f32 asymmetry data for the owner (question 3) |
| P1 Surface exchange, boundary layer, sequential physics, SCM (detail) | 2026-11-16: criteria 1–4, 6, 7 met; APE gate fails only the 35–60° westerlies row at T21, T42 and T63 (open question 6; owner: P2 proceeds, retest after P3/P4); §9.8 tracer-fixer trigger fired (2.3%/day) and was resolved by grid-point semi-Lagrangian humidity transport (T42: 0.12%/day) |
| P2 Land, snow, sea ice, precipitation phase | in progress (branch `p2-surface`): acceptance 1–4 met; 5–6 runs in progress; 7 blocked (netCDF libraries, ask the owner); defect 1 diagnosed |
| P3 Radiation (multi-band two-stream, semi-gray) | not started |
| P4 Clouds, Earth-like energy balance | not started |
| P5 Earth gate (prescribed ocean) | not started |
| P6 Ocean (Ekman, reduced-gravity layer), coupled Earth | not started |
| P7 Exoplanet gates | not started |
| P8 Local statistics, downscaling, biomes, exports | not started |
| P9 Viewer | not started |
| P10 Procedural terrain | needs go-ahead |
| P11 Extended envelope | needs go-ahead |

## Versions

- Rust 1.97.0 (cloud session), edition 2021.
- `godot` crate 0.5.5. Default features: `api-4-3` (stock single-precision 4.3+, CI).
  Laptop: feature `double` (`api-custom-json` + `double-precision`) against the owner's
  custom build *Godot v4.3.stable.custom_build.77dcf97d8* (`scons precision=double`,
  `~/src/godot-4.3-stable`, `~/.local/bin/godot4`), tested 2026-11-08.
- Laptop toolchain (2026-11-08): Rust 1.96.0, Python 3.12.3 with numpy 2.4, matplotlib 3.10,
  scipy 1.17; i7-11850H, 16 threads.

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

## 2026-11-05 (night, cont.) — f32 smoke check, winds, CI lint fix

- **f32 vs f64**, HS T21 L10 300 days (stats days 100–300), cloud VM 4 vCPU:
  jets 29.75/29.72 m/s (f32) vs 30.01/29.13 m/s (f64), all at 41–53° (f64 NH maximum at the
  top level σ = 0.05 again, see M5 note); lowest-level [u] tropics −4.11 vs −4.01 m/s,
  midlatitudes 4.02 vs 4.10 m/s; asymmetry 2.67 vs 3.03 m/s. Wall time 26.9 s vs 31.4 s
  (**speedup 1.17×**). The formal M6 check (T42 L20 jets in f32) stays on the laptop list.
- Near-surface wind fields `uas`, `vas`, `sfcwind` (lowest model level ≈ 60 m for L26, not
  reduced to 10 m); the world cases record precip, tas, sfcwind, uas, vas, ts, ps at stations.
- CI uses the latest stable Rust (1.99), whose clippy flags `chunks_exact(2)`; replaced by
  `as_chunks::<2>()` (stable since 1.88). `scripts/check` also runs clippy with any newer
  installed toolchain.

## 2026-11-08 — M3 on the laptop: Godot integration and viewer (branch `m3-godot-integration`)

First session on the owner's laptop (i7-11850H, 16 threads; other heavy jobs were running,
so only light builds and short runs). Git: `main` created from the cloud branch and made the
GitHub default; work now goes on one branch per segment, merged by PR.

**Done**
- Extension built against the owner's double-precision Godot 4.3 (API JSON dumped from
  that binary; gdext `api-custom-json` needs no libclang). Compiles with Rust 1.96.
- `./run` finds Godot like SpaceTrains (`$GODOT4_BINARY`, `~/.local/bin/godot4`, PATH),
  dumps the API JSON once per binary (`target/godot-api/`), builds with `--features double`
  (or `--single`), copies the library to `godot/bin/` (both the editor, which is a "debug"
  build, and release builds load that file), then starts the viewer. `--build-only` too.
- `toyclim.gdextension`: `compatibility_minimum = 4.3`, library `res://bin/…`;
  `project.godot`: features 4.3, `max_fps = 60` (so `--quit-after 600` ≈ 10 s).
- `toyclim-godot`: relative input paths in a case (`[surface] heightmap`) that do not exist
  from the working directory are resolved next to the config file and its parents (Godot
  changes the working directory to `godot/`).
- Viewer (`godot/main.gd`, scene built in code): globe (SphereMesh 128×64, 30° graticule)
  and flat map (M), orbit camera, case/field/level selectors, auto or typed colour range,
  diverging colormap for signed fields, run/pause/step, speed, diagnostics, model error.
  Command line after `--`: `--case= --field= --level= --run --sps= --map
  --screenshot=<png> --after=<s>`. Orientation checked against Godot's SphereMesh source
  (UV.x grows eastward from +Z, UV.y = 0 at the north pole), so no texture flip.

**Measured**
- M3 acceptance: `godot4 --headless --path godot --quit-after 600` → exit 0, Williamson 6
  (T42, Δt 20 min) advanced 9210 steps = 127.9 model days in ≈ 10 s, ≈ 900–1250 steps/s
  printed each second (machine loaded, load average ≈ 15).
- Also loaded and stepped headless: `held_suarez_t21` (≈ 60–130 steps/s) and
  `testworld_t21` (heightmap found via the path fix, ≈ 10–20 steps/s).
- Windowed (X11, gl_compatibility) screenshots: Galewsky vorticity at day 5.4 shows the
  expected roll-up into vortices; HS T21 temperature on the globe; testworld `tas`.

**Figures**: `docs/figures/M3/galewsky_vor_map.png`, `held_suarez_t21_globe.png`,
`testworld_t21_tas_globe.png`.

**Open questions**: none new.

**Next**: laptop checklist item 3 (M5 acceptance runs, T42 L20) when the machine is free;
viewer extras (wind particles, σ value of the selected level) later.

---

## 2026-11-12 — Overnight acceptance and world runs on the laptop (branch `m5-laptop-long-runs`)

Laptop checklist items 3, 4, 6 and 7 run unattended with `scripts/night_runs` (new): three
lanes × 4 threads (12 of 16), analysis tools run after each case, then timing and the
`--ignored` tests. Started 2026-11-11 20:29, finished 02:54; 27 of 28 steps rc = 0. Logs in
`out/night/` (not committed). New case `cases/moist_held_suarez_sbm.toml` (T42 L26 moist HS +
SBM convection, otherwise identical to `moist_held_suarez.toml`). Commit b7d4e0a for all runs.

**M5 acceptance — HS T42 L20, 1200 days** (`zonal_mean.py`, samples from day 200)

| quantity | measured | criterion |
|---|---|---|
| NH / SH jet | 33.72 m/s at 43.3°, σ 0.225 / 32.20 m/s at −43.3°, σ 0.225 | 26–34 m/s, 38–50°, σ 0.15–0.35 ✅ |
| lowest-level [u] | tropics −5.66, midlatitudes +4.31 m/s | easterlies / westerlies ✅ |
| hemispheric asymmetry | 2.13 m/s | < 3 m/s ✅ |
| KE trend | −1.52 % (mean 6.80e20 J) | < 5 % ✅ |

Hadley cells 9.85e10 / 9.75e10 kg/s at ±12.6°, edge ≈ 29°. Wall 1737 s (4 threads, machine shared).

**Rotation sweep (HS, 800 days)**: Ω/Ω⊕ = 0.25 (T42): jets 51.9 / 52.8 m/s at ±54.4°, Hadley
1.71e11 kg/s, edge 38–40°. Ω/Ω⊕ = 2 (T85, Δt 10 min, 11887 s wall): jets 25.7 / 24.5 m/s at
27.3° / −24.5°, Hadley 5.1e10 / 4.7e10 kg/s, edge 19–20°. One jet per hemisphere in all
three runs; jets move poleward and the Hadley cell widens as Ω decreases, as expected. (KE
trends 10.3 % / 6.6 % over the 600-day window; the < 5 % criterion is for the 1200-day HS run.)

**Tidally locked, T42 L20, 800 days** (`tidally_locked.py`)

| Ω/Ω⊕ | equatorial [u] above σ 0.5 (mean) | t850 day−night | t500 day−night |
|---|---|---|---|
| 1 | +22.2 m/s, max 36.7 at σ 0.32 (superrotating ✅) | 9.1 K | 6.0 K |
| 0.1 | +17.4 m/s | 6.1 K | 2.3 K |
| 0.02 | +7.5 m/s | 3.7 K | 0.3 K |

At slow rotation the free-troposphere contrast is much weaker than near the surface (weak
temperature gradient regime) ✅.

**M6**: `toyclim bench --config cases/held_suarez.toml --length 365day` on 8 threads:
f64 289.6 s (1.26 model days/s, 11.0 ms/step) ✅ budget ≤ 10 min; f32 220.9 s (8.4 ms/step,
speedup 1.31×). f32 HS 1200 d: jets 32.64 m/s at 43.3° σ 0.225 / 32.20 m/s at −43.3°, surface
−5.65 / +4.55 m/s, KE trend +1.06 % — but hemispheric asymmetry **3.19 m/s > 3 m/s** ❌
(open question 3).
`cargo test --release -- --ignored` (8 threads): 6/6 pass in 79 s (williamson2 convergence,
Galewsky unperturbed T85, Williamson 5 T85, PE conservation 30 d, linear gravity waves,
JW steady state T85).

**M7 — moist HS T42 L26, 1080 days** (samples from day 180)

| | P = E (mm/day) | max zonal P | PW | jets | asymmetry | KE trend |
|---|---|---|---|---|---|---|
| TJ16 large-scale only | 1.909 | 10.07 at 1.4° | 19.72 mm | 58.3 / 57.9 m/s at ~30°, σ 0.19 | 1.77 | 0.50 % |
| + SBM convection | 2.168 | 12.07 at 1.4° | 20.05 mm | 60.2 / 61.1 m/s at 29.3°, σ 0.16 | 1.08 | 0.88 % |

Wall 3162 s / 5690 s.

**Worlds (M8/M9 first steps)**
- Gray aquaplanet, 10 years (19372 s wall): P = E = 3.45 mm/day, PW 27.6 mm, zonal P max
  7.77 mm/day at 9.8° (seasonal ITCZ). Köppen: Aw 20 %, Cfa 18 %, Cfb 17 %, Am 12 %, BSh 5 %, D 7 %.
- Testworld, 4 years (7790 s): P = E = 3.10 mm/day, PW 25.9 mm, zonal P max 7.06 mm/day at
  12.6°. Köppen (grid): Aw 15 %, BWh 10 %, BSh 9 %, Cfa 8 %, Csa 7 %, Cfb 7 %, Am 7 %, Dsa 5 %.
  Downscaled map, exported layers (`out/testworld/layers`), monthly station distributions of
  precip/sfcwind/tas and point-rain statistics all produced.
- Tidally locked gray, 3 years (6008 s): run OK, `moist_stats.py` OK; `tidally_locked.py`
  fails because the case's climatology has no `t500` map (as the checklist warned).

**Figures**: `docs/figures/M5/` (hs_t42l20_zonal_mean, hs_t42l20_f32_zonal_mean,
held_suarez_omega025/2, tidally_locked, _omega01, _omega002), `docs/figures/M7/`
(mhs_t42_*, mhs_sbm_t42_*), `docs/figures/M9/` (gray_aquaplanet_*, testworld_t42_*,
tidally_locked_gray_moist).

**Not done / next**
- The figures were produced by the tools but not inspected in this session (token budget);
  look at them before reading too much into the world climates.
- Add t500/t850/u850/v850 maps to `cases/tidally_locked_gray.toml` and rerun (~1.7 h).
- Laptop checklist items 2 (paper constants) and 5 (SpeedyWeather/Isca reference) remain.

**Open questions**: 3 (new, below).

---

## 2026-11-12 — Architecture and plan audit; SPEC.md Part II written

**Milestone**: none (planning). No code changed.

**Done**
- Audit of the plan, the physics/surface code and the 2026-11-12 world-run figures.
- `SPEC.md` rewritten from §9 on: Phase 2 principles (spread rule, tuning governance,
  supported envelope), target architecture (module layout, config schema v2 with star/orbit
  derivations, sequential physics driver), physics specifications (radiation, surface
  exchange, boundary layer with implicit surface coupling, phase of precipitation, land,
  snow, sea ice, albedo, clouds, wind-driven ocean, downscaling), validation infrastructure
  (single-column model, scorecards, reference data, regression script), milestones P0–P11
  with acceptance criteria, conditional work items, review triggers. §10 (protocol) and §11
  (references) updated. `CLAUDE.md` updated to point at it.

**Figures looked at** (existing, `docs/figures/M9/`)
- `testworld_t42_koppen.png`, `testworld_t42_downscaled.png`: tropical land is a cell-scale
  mosaic of Af/Aw and BWh; 40–60°N is mostly Dsa; EF over open ocean poleward of ≈ 55°S and
  over land poleward of ≈ 70°N.
- `testworld_t42_moist.png`: time-mean precipitation has isolated single-cell maxima up to
  ≈ 40 mm/day over equatorial land; zonal-mean P peaks at ≈ 7 mm/day near 12°N.
- `tidally_locked_gray_moist.png`: precipitation concentrated just east of the substellar
  point (max ≈ 55 mm/day), dry night side; plausible in shape.
- `gray_aquaplanet_zonal_mean.png`: jets ≈ 30 m/s near 30°, σ ≈ 0.25; Hadley cells
  ≈ 9e10 kg/s; looks like a normal gray aquaplanet.

**Caveat**: formulas, constants and citations in SPEC.md §9 and §11 were written from
memory; each must be checked against its paper when first used (SPEC.md P-3).

**Open questions**: 3 (f32, now P0 item 1); 4 (new, below).

**Next step**: P0 (SPEC.md §9.6), starting with `scripts/regress` so the refactor can be
proved result-neutral.

---

## 2026-11-15 — P0 Groundwork (branch `p0-groundwork`)

**Milestone**: P0 (SPEC.md §9.6). All acceptance criteria met; the f32 item is data for the
owner (open question 3).

**Done**
- `scripts/regress` (§9.5.4): `--against <ref>` builds the reference commit in a worktree,
  runs the six P0 cases with both builds (1 and 8 threads, binaries snapshotted), and
  compares every output file byte for byte (`scripts/regress_compare.py`; JSON sidecars
  without git hash and config echo). Cases are found by file stem, so moved and migrated
  cases are compared. `scripts/regress_ref_patches/<case>.sed` documents intended output
  changes (one: two added climatology maps). Without `--against`: the milestone ratchet
  (all `--ignored` tests, HS T42 1200 d with `tools/zonal_mean.py --hs-criteria`).
- `docs/tuning.md`: every prototype parameter with default, status and source.
- `tools/tidally_locked.py`: missing maps are skipped, substellar longitude from the
  config; the gray tidally locked cases now write `v850` and `t500`.
- Laptop checklist 2: Williamson test 6 (OSTI report version, §3.6, eqs. 135–143) and
  Galewsky et al. (2004, eqs. 2–4, plus the 2006 corrigendum) — all constants agree with
  the code; citations in the case files.
- Restructure (§9.3.1): `primitive/` split into mod (395 lines), state (flat `[lev][point]`
  `Field3`, also for the spectral state), tendencies, implicit, tracers, levels, output,
  restart, setup; `physics/driver.rs` (column loop with buffers allocated once, per-worker
  scratch, **diagnostics registry** with typed ids — no `diagnostic_names()` per column, no
  prefix matching per step); `surface/` (slab, land bucket, albedo switch, geography,
  q-flux); `system/` (orbit, calendar, star, derive); `config/` (blocks, duration, v2,
  envelope); `atmosphere.rs`. The previous-state clone is gone (state buffers recycled),
  as are the per-step tendency and grid buffers. Largest core file: 478 lines.
- **Config schema 2** (§9.3.2): `[star]`, `[orbit]` (`semi_major_axis`, `spin` free /
  synchronous / resonant), `[atmosphere]` (composition → R_d, c_p; JANAF 300 K heat
  capacities, checked), typed physics blocks, `[surface.ocean|land|sea_ice]`, named level
  sets (`"L26"`), default Δt per resolution. Overrides more than 1% off the derived value
  are rejected unless `allow_inconsistent = true`. Settings outside the envelope (P-5) are
  logged as warnings. The derived values are echoed as `config.system` in `meta.json`.
- Cases reorganised into `core/`, `idealized/`, `bench/` (empty), `worlds/`; the six world
  cases migrated to schema 2 with their old numbers as checked overrides (365.25-day year,
  R_d = 287.04; the tidally locked case keeps S = 900 W m⁻² and P = 6.1 d with a
  TRAPPIST-1-like star whose L is 2% above Agol et al. 2021 — stated in the file).
  `toyclim list-cases` and the viewer find cases in subfolders.
- `toyclim bench` prints a per-phase profile (`Model::set_profiling`, results unaffected).

**Measured**
- `scripts/regress --against 0f7d33a` (pre-P0), commit 0f216f1: **PASS**, all 6 cases ×
  {1, 8} threads byte-identical (120–338 files each), new build thread-independent.
  The later performance commit ff76cc6 was compared on HS, TJ16+SBM and test world T21
  at 1 and 8 threads against those outputs: identical.
- Derivation tests (`system::derive`, `config::v2::tests`): Earth S = 1361 W m⁻², year
  365.256 d (to 1e-4); TRAPPIST-1e (Agol et al. 2021: L = 5.53e-4 L☉, M = 0.0898 M☉,
  a = 0.02925 AU) year 6.098 d (paper 6.0996 d), S = 880 W m⁻² (paper 0.646 S⊕ = 879;
  within 5% of 900); SPEC input a = 0.0293 AU also passes; inconsistent overrides rejected.
- Performance, i7-11850H, 8 threads, quiet machine, before (0f7d33a) → after (ff76cc6):
  HS T42 L20 (Δt 20 min) **11.4 → 5.53 ms/step (2.06×)**, one model year ≈ 145 s;
  test world T42 L26 (Δt 15 min) 27.6 → 20.1 ms/step (1.37×; prototype column physics
  is 58% of the step). One thread, HS: 73 → 50 ms/step.
- f32 study (open question 3): HS T42 L20 2200 d, seeds 1/2: asymmetry f64 1.52 / 1.79,
  f32 4.24 / 0.87 m/s; jets 32.3–33.9 m/s at ±43°, σ 0.225 (f32 s1 SH at −46°); KE trend
  over the last 1000 d −1.8 / −0.8 / −4.7 / +0.7 %. Wall 4.9e3 s (f64) / 4.5e3 s (f32),
  3 threads each, machine shared. The f64 seed-1 fields at day 1200 are bit-identical to
  the accepted M5 run.
- Viewer: `godot4 --headless --path godot --quit-after 600 -- --case=testworld_t21 --run`
  exits 0 and advances (schema-2 case found by bare name in `worlds/`); default case too.

**Figures** (looked at)
- `docs/figures/P0/tidally_locked_gray_t21.png`: T21 one year; T850 peaks ≈ 285 K just east
  of the substellar point, poles ≈ 238 K; upper-tropospheric superrotation with twin
  ≈ 45 m/s maxima at ±30°, σ ≈ 0.4; tool output: equatorial [u] above σ 0.5 = 18.4 m/s,
  day−night contrast 12.9 K at 850 hPa, 7.7 K at 500 hPa.
- `docs/figures/P0/hs_t42l20_2200d_{f64,f32}_s{1,2}.png`: four textbook HS climates (jets,
  eddy T variance at σ ≈ 0.8 near 40–45°, Hadley cells ≈ 9e10 kg/s); f32 seed 1 has a
  visibly broader SH jet on its poleward flank.

**Deviations from SPEC.md, recorded**
- Typed physics blocks lower to the prototype bundle (`[forcing] type =
  "gray_aquaplanet"`) instead of the bundle expanding to blocks; interim scheme names
  `boundary_layer.scheme = "tj16"` and `clouds.scheme = "rh_prototype"`. P1 inverts this
  when the sequential driver arrives.
- The envelope (P-5) is enforced by warnings, not refusal.
- Allocations inside the prototype schemes remain until the P1 `ColumnScheme` interface
  with a per-worker `Workspace`; the driver itself no longer allocates per column.

**Tuning changes**: none (register created).

**Open questions**: 3 (updated with the f32 data), 5 (new: Courant trigger of §9.8).

**Next step**: P1 (sequential driver, surface exchange, boundary layer, SCM, aquaplanet
gate).

**Ratchet** (`scripts/regress`, ff76cc6, 8 threads): `--ignored` tests 6/6 pass (48 s);
HS T42 L20 1200 d: jets 33.72 / 32.20 m/s at ±43.3°, σ 0.225, lowest-level [u] −5.66 /
+4.31 m/s, asymmetry 2.13 m/s, KE trend −1.52% — identical to the accepted M5 numbers.
**PASS**.

---

## 2026-11-16 — P1 Surface exchange, boundary layer, sequential physics, SCM (branch `p1-surface-pbl`)

**Milestone**: P1 (SPEC.md §9.6). Criteria 1–4, 6 and 7 met; criterion 5 (APE Control
aquaplanet) meets 5 of its 6 rows. The row "lowest-level westerlies at 35–60°" **fails** at
T21 and T42. The criterion is not weakened; P1 is blocked on that row (open question 6).

**Done**
- Sequential (time-split) physics (§9.3.3): `ColumnScheme` trait with a per-worker
  `Workspace` and `ColumnState`. Each scheme sees the state its predecessors left
  (`add_*` rates, `set_*` adjustments); the dynamics gets the summed tendency. Held–Suarez
  runs through the same path and stays byte-identical.
- Coupled implicit PBL–surface solve: downward elimination of the K-profile diffusion gives
  x_L⁺ = A + B·J; each tile (ocean, sea ice, land; `MAX_TILES` 3) solves its linearised
  energy balance (σT⁴ and q_sat linearised); the area-mean fluxes are back-substituted. The
  linearised change of surface LW emission goes to space (`olr`).
- Louis (1979) surface exchange with Charnock roughness over the ocean, and 2 m / 10 m
  diagnostics by Monin–Obukhov interpolation. Frierson, Held & Zurita-Gotor (2006)
  K-profile boundary layer (Ri_c = 1, f_b = 0.1). Also: frictional heating, pairwise
  enthalpy-conserving dry adjustment, implicit sponge (off by default), and an energy fixer
  for hyperdiffusion KE loss.
- World cases: `[boundary_layer] scheme = "kprofile"`, `[dynamics] energy_fixer = true`;
  no Rayleigh friction. Typed physics blocks now build the chain directly
  (`physics::build_world`).
- Budget accumulators (TOA, surface, precipitation, evaporation, diffusion KE, fixer) are
  written to `diagnostics.csv` and restarts. New output fields: `t_low`, `u_low`, `v_low`,
  `wind_low`, `psl` (see deviations).
- `toyclim scm` single-column model with cases in `cases/scm/`, `tools/scm_plot.py`, and
  unit tests for acceptance 1–3.
- `tools/scorecard.py` (world metrics, aquaplanet suite, JSON in `docs/scorecards/`).
  `cases/bench/neale_hoskins2000/ape_control{,_t21}.toml` (APE Control: SST
  27(1 − sin²(3φ/2)) °C for |φ| < 60°, else 0 °C; S₀ = 1365, perpetual equinox,
  no sea ice). The T21 gate is in the `scripts/regress` ratchet.
- TJ16 moist HS runs in the paper's sequential order (SBM → LSC → surface fluxes → BL →
  HS), so M7 was re-recorded (below).

**Measured**
- SCM conservation (acceptance 1): each scheme alone and the full chain, over dry, moist
  and stiff land and over the ocean, 96 steps each. Max |energy residual| 4e-9 W m⁻²
  (limit 1e-6); water residual within 1e-9 kg m⁻² s⁻¹.
- Stiff land (acceptance 2; C = 1e3 J m⁻² K⁻¹, 30 m/s, Δt_p 1 h, 10 days): no 2Δt
  oscillation.
- Diurnal cycle (acceptance 3): dry land at 20°, equinox, 2 m range **8.6 K**; at night the
  sensible heat flux is −6.3 W m⁻² and θ increases with height (stable). 50 m ocean: 2 m
  range **0.73 K**.
- Neutral C_D at 10 m, 10 m/s with Charnock (acceptance 4): **1.44e-3**.
- Regression (acceptance 6), `scripts/regress --against main`, 1 and 8 threads: HS T21
  **identical**. The moist HS, gray aquaplanet, test world and tidally locked cases differ
  by design and are thread-independent. The obsolete P0 reference patch was removed (main
  already has those maps).
- Viewer (acceptance 7): `godot4 --headless --path godot --quit-after 600 --
  --case=testworld_t21 --run` loads the schema-2 world with the new physics and advances
  1856 steps (25.8 days).
- **APE Control T42 L26** (acceptance 5), Δt 15 min, 1461 days, statistics years 2–4 (4385
  samples), 4163 s wall on 8 threads (shared with the M7 runs for the first 2/3).
  Scorecard: `docs/scorecards/P1_aquaplanet_2026-11-16.json`.

  | Row | Value | Result |
  |---|---|---|
  | tropical lowest-level easterlies | mean −5.17 m/s (\|φ\| < 15°); max within 10° −2.32 | PASS |
  | westerlies at every latitude 35–60° | westerly 24°–48° N/S; easterly from 49° (NH) / 48.8° (SH); min −4.17 m/s at 60° | **FAIL** |
  | peak lowest-level westerlies 5–12 m/s | 7.79 / 7.79 m/s at ±34.9° | PASS |
  | P = E within 1% | 4.765 vs 4.764 mm/day (rel. 1.3e-4) | PASS |
  | atmospheric energy closure < 0.5 W m⁻² | 0.012 W m⁻² | PASS |
  | storm index G ≤ 2.0 | 1.17 | PASS |

  Recorded: Hadley cells 19.7 / 19.4e10 kg/s, maxima at ±9.8°, edges ±23.6°. Jets 42.4 /
  42.2 m/s at ±32.1°, σ 0.226. Zonal precipitation peaks at 9.95 mm/day (4.2°S), with
  midlatitude maxima of about 6 mm/day near ±33°. PW 14.7 mm; T_s 286.3 K; OLR 226.7 W m⁻².
  Water closure 6e-9 kg m⁻² s⁻¹. Hyperdiffusion KE loss 1.49 W m⁻² (returned by the fixer).
  KE trend over the last 1000 days 2.05%.
- APE Control T21 (the ratchet gate; `out/p1/ape_t21_b`): same pattern. Westerly peaks
  6.35 / 5.88 m/s at ±30.5°; easterly from 41°, min −7.10 m/s; closure 0.12 W m⁻²; G 1.37.
  Hadley 15.9 / 15.0e10 kg/s, edge 17.5°.
- **§9.8 tracer-fixer trigger fired**: the negative-humidity fixer moves **2.29%** of the
  global vapour mass per day at T42 (1.25% at T21), against the 1%/day trigger. That is
  about 0.34 mm/day, ~7% of the precipitation. The remedy (grid-point semi-Lagrangian
  tracer transport) is the next work item.
- Courant: the log shows up to 2.14 with the polar 1/cos φ metric; not applied (open
  question 5).
- M7 re-record (TJ16 sequential order), T42 L26, 1080 days, statistics from day 180
  (old → new):
  - LSC only:
    - P = E 1.909 → 1.978 mm/day;
    - max zonal P 10.07 → 10.79 mm/day;
    - PW 19.72 → 20.31 mm;
    - jets 58.3/57.9 → 58.8/58.6 m/s at 32.1°, σ 0.163;
    - asymmetry 1.77 → 1.91 m/s;
    - KE trend 0.50 → 0.08%;
    - 3730 s wall.
  - SBM:
    - P = E 2.168 → 2.254 mm/day;
    - max zonal P 12.07 → 11.30 mm/day;
    - PW 20.05 → 20.70 mm;
    - jets 60.2/61.1 → 62.7/63.1 m/s at 29.3°, σ 0.163;
    - asymmetry 1.08 → 1.62 m/s;
    - KE trend 0.88 → 0.71%;
    - 5405 s wall.

**Figures** (looked at)
- `docs/figures/P1/scm_diurnal.png`: days 7–10 of the three SCM cases.
  - Dry land: T_s 310–329 K and 2 m air 310–319 K. The daytime sensible heat flux peaks
    near 480 W m⁻² and is slightly negative at night. The PBL depth swings between about
    1.1 km and 5.7 km.
  - Ocean: T_s varies by about 0.2 K and the 2 m air by about 0.8 K; latent heat flux
    60–80 W m⁻².
  - Stiff land: a smooth diurnal cycle with no 2Δt noise.
  - The columns have no large-scale cooling, so they drift warm and grow very deep mixed
    layers. That is acceptable for a stability and diurnal test, not for a climate.
- `docs/figures/P1/ape_t42_zonal_mean.png`:
  - a single merged jet near 32° (42 m/s, σ 0.23);
  - eddy temperature variance peaking near 40° (σ 0.6 and 0.25);
  - Hadley cells to ±24°;
  - weak Ferrel cells near 35°;
  - surface easterlies poleward of about 50°.
- `docs/figures/P1/ape_t42_moist.png`:
  - equatorial rain band near 10 mm/day;
  - subtropical dry zones of about 3.5 mm/day at ±18°, where P − E reaches −3.5;
  - storm-track rain of about 6 mm/day at ±33°;
  - zonally uniform rain bands without grid-point storms.
- `docs/figures/P1/m7_moist_hs{,_sbm}_t42_{zonal_mean,moist}.png`: TJ16-like climates. They
  show a narrow equatorial rain band of about 11 mm/day, storm tracks near ±38–40°, Hadley
  cells of 12–16e10 kg/s, and surface easterlies poleward of about 50°.

**Deviations from SPEC.md, recorded**
- APE cases use `tau_sw = 0`, the FHZ06 configuration (see `docs/tuning.md`). With the
  prototype τ_sw = 0.22 along the slant path, the T21 run warmed the polar upper
  troposphere at low sun. Polar SCM runs (SBM, dry, none, τ_sw 0) traced the warming to
  this absorption. World cases keep 0.22 until P3.
- `psl` uses an ECMWF/Trenberth-style reduction written from memory. The source document
  could not be fetched, so the field is marked unverified in `levels.rs`.
- SBM shallow convection (§9.4.4 option) is not implemented; deep SBM as in the prototype.
- The hyperdiffusion energy fixer is on in all P1 world and benchmark cases.

**Tuning changes**: none. New knobs are at their published defaults (`docs/tuning.md`).

**Open questions**: 6 (new: APE 35–60° westerlies).

**Next step**:
1. Grid-point semi-Lagrangian tracer transport (the §9.8 trigger fired).
2. Rerun the APE gate.
3. The owner's decision on question 6.

**Ratchet**: `scripts/regress --skip-hs --skip-gates` (6634482): `--ignored` tests PASS. HS T42 is covered by the byte-identity with main (HS T21, identical outputs through the new driver). The P1 T21 gate in the full ratchet fails the same 35–60° row as recorded above.

---

## Laptop checklist (things the cloud session could not do)

1. ~~**Godot integration**~~ — done 2026-11-08 (M3 entry below).
2. ~~Re-check Williamson test 6 and Galewsky constants against the original papers~~ — done
   2026-11-15 (P0 entry): all constants agree with the code.
3. ~~**Acceptance runs**~~ — done 2026-11-12 (entry above). (each in the background, `--release`; analyse with the tools):

   ```bash
   cargo build --release
   RAYON_NUM_THREADS=8 target/release/toyclim run --config cases/idealized/held_suarez.toml --out out/hs
   python tools/zonal_mean.py out/hs -o docs/figures/M5/hs_t42l20_zonal_mean.png
   for c in tidally_locked tidally_locked_omega01 tidally_locked_omega002; do
     target/release/toyclim run --config $(find cases -name $c.toml) --out out/$c
     python tools/tidally_locked.py out/$c -o docs/figures/M5/$c.png; done
   for c in held_suarez_omega025 held_suarez_omega2; do
     target/release/toyclim run --config $(find cases -name $c.toml) --out out/$c
     python tools/zonal_mean.py out/$c -o docs/figures/M5/$c.png; done
   ```

   M5 criteria (jets 26–34 m/s at 38–50°, σ 0.15–0.35; surface easterlies/westerlies;
   asymmetry < 3 m/s; KE trend < 5% over the last 1000 days) are printed by
   `zonal_mean.py`. Also time one model year (`toyclim bench --config cases/idealized/held_suarez.toml
   --length 365day`) on 8 threads → M6 budget.
4. ~~M6 f32 check~~ — done 2026-11-12 (asymmetry 3.19 m/s, open question 3): build with `--features f32` (`cargo build --release -p toyclim-headless
   --features f32 --target-dir target/f32`), rerun HS, compare jets and record the speedup.
5. Compare HS with SpeedyWeather/Isca output placed in `reference/` (and later PlaSim).
6. ~~M7~~ (done 2026-11-12): `cases/idealized/moist_held_suarez.toml` (T42 L26, 1080 days, 6-month spin-up as in TJ16), with
   and without `convection = "sbm"`; `tools/moist_stats.py`, `tools/zonal_mean.py`.
7. Worlds (overnight runs, the owner's main use) — done 2026-11-12 except the tidally locked gray maps:
   - `cases/worlds/gray_aquaplanet.toml` (T42 L26, 10 years) → `moist_stats.py`, `koppen.py`;
   - `cases/worlds/testworld.toml` (T42 L26, 4 years, continents from `worlds/testworld.npy`) →
     `koppen.py`, `downscale.py out/testworld worlds/testworld.npy --orographic`,
     `export_layers.py`, `station_stats.py ... --by-month`, `point_rain.py --by-month`;
   - `cases/worlds/tidally_locked_gray.toml` (T42 L26, 3 years) → `tidally_locked.py`-style maps
     (needs maps t850/u850/v850 in its climatology block if wanted).

---

## 2026-11-16 (evening) — P1: semi-Lagrangian humidity transport, T63 resolution test

**Done**:
- §9.8 remedy for the tracer-fixer trigger: grid-point semi-Lagrangian transport of the
  humidity (`primitive/semi_lagrangian.rs`, `[dynamics] tracer_transport =
  "semi_lagrangian"`). It uses:
  - Ritchie (1987) midpoint departure points on the sphere;
  - clipped 4×4 cubic Lagrange interpolation (Bermejo & Staniforth 1992) with pole halo
    rows;
  - vertical flux-form MUSCL with a van Leer limiter;
  - a global multiplicative mass fixer.

  All moist world and APE cases use it. The spectral path is unchanged and byte-identical
  (Held–Suarez, TJ16).
- The APE gate was rerun with the new transport at T21 and T42, and at T63 with spectral
  transport as the owner's option (b). The data is under open question 6.

**Measured** (APE Control, years 2–4):

| run | fixer | energy closure | P = E | G | min u 35–60° |
|---|---|---|---|---|---|
| T21 SL | 0.17 %/day (was 1.25) | 0.10 W/m² | 3e-5 | 1.30 | −7.06 m/s |
| T42 SL | 0.12 %/day (was 2.29) | 0.006 W/m² | 8e-5 | 1.19 | −3.69 m/s |
| T63 spectral | 2.42 %/day | 0.008 W/m² | 7e-5 | 1.15 | −2.55 m/s |

The T63 run took 5.2 h wall on 8 threads, sharing the machine with other runs.

**Notes**:
- The scorecard's "jet" metric is the largest zonal-mean u anywhere. At T21 SL that is a
  46 m/s equatorial stratospheric wind at σ 0.037, not the tropospheric jet. No pass/fail
  row uses this metric.

**Tuning changes**: none.

**Open questions**: 6 (updated with the T63 data).

**Next step**: P2 (branch `p2-surface`), with the 35–60° row open.

## 2026-11-16 (night) — P2 in progress: land-world crash fixed, budgets, defect 1 diagnosed (branch `p2-surface`)

**Done**: phase and fall of precipitation and the tiled surface (5-layer soil, bucket,
snow, zero-layer sea ice, two-band albedo; commits b84b4ce…59db440, described in the commit
messages and `docs/tuning.md`).

**Crash of the 3-year T21 land world at day 841, now fixed (f59dd7e).**
- **Cause:** a positive feedback in the land melt cap. The cell (52.6°N) is 97 % sea ice
  and 3 % land, and light snow kept a trace on the land tile after the sun warmed it above
  T_melt.
  - The surface solve capped a snow-covered tile at T_melt even when it started above it,
    and evaluated the fluxes at T_melt.
  - The surplus the trace of snow could not take went into the soil, so the soil warmed
    further, and the gap to T_melt grew each step.
  - The skin went 278 → ~700 → ~6000 K, the bucket reached −1.8e5 kg/m², and the state
    became NaN at step 60562.
- **Fix:**
  - The melt is at most L_f S/Δt_p (`Tile::melt_max`). Beyond it the tile warms past T_max,
    with the melt as a fixed sink.
  - Snow fallen during the step and left on a skin above T_melt melts with the top soil
    layer's heat.
- **Test:** the SCM test `trace_snow_on_warm_land_stays_bounded` (old code: 9.4e7 K, then
  NaN; fixed: peak 284.9 K).
- **Verification:** a restart from day 835 runs through to day 850, with peak T_s 320 K
  and no negative bucket.
- **Runs redone with the fixed binary:** the land world and the T42 test world (stopped
  after ~1.3 h). The buggy outputs are kept in `out/p2/buggy/`, not in the repo.

**Measured** (`cases/bench/p2_budget/land_world_t21.toml`, 1096 days, window days 30–1096):

| P2 acceptance | criterion | result |
|---|---|---|
| 2. system water budget | ≤ 1e-3 mm/day | 7.3e-4 mm/day |
| 3. system energy closure | ≤ 0.5 W/m² | 0.11 W/m² |
| 4. bitwise restart and thread independence | identical | `land_world_runs_and_restarts` passes; the 3-D restart from day 835 reproduced the crash step exactly |

Climate of the land world, for reference: T_s 279.5 K, P 2.81 / E 2.80 mm/day, planetary
albedo 0.35, net TOA −3.1 W/m² (still cooling in year 3), runoff 0.24 mm/day, G 1.52,
fixer 0.20 %/day.

**Defect 1, grid-point storms over tropical land (§9.1.1):** T42 test world, 455 days,
climatology days 90–455. The defect is measured over tropical land (|φ| < 30°, land
fraction > 0.5) as the number of cells with time-mean P > 30 mm/day and their maximum. The
global G is also given; in five of the six runs its maximum is an ocean cell at (−15°,
174°E) with ~3 mm/day next to drier neighbours.

| run | change from the prototype | land cells > 30 mm/day | max land P | global G |
|---|---|---|---|---|
| E0 | prototype (main 65a5f5d) | 6 | 38 mm/day | 11.1 |
| E0a | (a) bucket never dry (capacity 1e6) | 4 | 48 mm/day | 2.0 |
| E0b | (b) land heat capacity ×10 (2e6) | 13 | 50 mm/day | 10.5 |
| E0d | (d) SBM τ 2 h → 4 h | 12 | 48 mm/day | 7.5 |
| E1 | P1 code: sequential physics (c) + implicit PBL–surface solve | 0 | 24 mm/day | 3.8 |
| E2 | P2 code (this branch) | 0 | 20 mm/day | 2.0 |

- **(a)** A wet bucket removes the dry-ocean contrast behind the large global G, but not
  the land storms.
- **(b) and (d)** change neither.
- **Verdict:** the storms disappear only with the P1 physics. That points to **(c)**,
  convection and large-scale condensation acting on the same input state. (c) is not
  separated from the implicit surface solve that came in the same milestone. (b)'s
  small-heat-capacity half alone does not explain it.

**Tuning changes**: none.

**Open questions**: none new. P2 acceptance 7 needs Python netCDF libraries; I will ask the
owner before installing them.

**Next step**:
1. Evaluate the T42 test world, 15 years, slab 50 m (acceptance 5: equilibrium, G,
   Köppen EF over ice-free ocean, sea ice only where `tos` is at freezing).
2. Evaluate the bistability runs (acceptance 6), then the instellation sweep in 2 % steps.
3. Acceptance 7, after the owner's answer on netCDF.

## Open questions for the owner

1. ~~License~~ — resolved: MIT (LICENSE added).
2. Owner scoping answers are in `docs/design-decisions.md`. Two of them extend the SPEC and
   need a go-ahead before implementation: station time-series output (proposed for M2,
   SPEC §6 addition) and a procedural terrain milestone (after M9).
3. **f32 Held–Suarez asymmetry** (2026-11-12): in f32 the jets and surface winds meet the M5
   criteria, but the max hemispheric asymmetry of [u] is 3.19 m/s (f64: 2.13 m/s) against
   < 3 m/s. With a 1000-day window this is plausibly sampling variability rather than a
   precision bug. Criterion not weakened. Options: (a) accept as recorded, (b) rerun f32
   longer (e.g. 2200 days, ~30 min) or with another perturbation seed and report both.
   **P0 result (2026-11-15)**: 2200-day runs, seeds 1 and 2, f32 and f64 (statistics days
   200–2200, 6-hourly): asymmetry f64 1.52 / 1.79, f32 **4.24** / 0.87 m/s; all other M5
   criteria pass in all four. From 10-daily snapshots, single 1000-day windows reach 5.3 m/s
   in f64 (seed 2, days 200–1200) and 5.5 m/s in f32 (seed 2), so 1000-day means are not
   enough to resolve 3 m/s in either precision. Over 2000 days three runs fall to 1.1–1.6 m/s
   (snapshots), but f32 seed 1 stays asymmetric in both halves (3.7, 5.2): a persistent
   poleward broadening of its SH jet (max difference at σ 0.275, 52°). Reading: mostly
   sampling variability of a slow annular-mode-like fluctuation, with one f32 run in a long
   asymmetric excursion; a precision-induced bias cannot be excluded from four runs.
   Options: (a) accept f32 as recorded (f64 stays the default for acceptance, f32 only for
   speed); (b) define the M5 asymmetry criterion on ≥ 2000-day means or an ensemble mean of
   two seeds (a criterion change — owner's call); (c) four more seeds per precision (~3 h on
   the laptop) to test for a systematic f32 difference. Recommendation: (a) now, (c) only if
   f32 becomes a production mode. Figures: `docs/figures/P0/hs_t42l20_2200d_*.png`.
4. **Earth-gate thresholds** (2026-11-12): the numbers in SPEC.md §9.6 P5/P6 were set from
   judgement of what intermediate-complexity models achieve, before any Earth run exists.
   They are binding for sessions; the owner may want to revise them after seeing the first
   `earth_amip` scorecard (recorded without thresholds at the end of P2).
5. **Courant trigger of SPEC.md §9.8** (2026-11-15): "max Courant number above 0.8 in the log →
   reduce Δt". The logged number is SPEC §4.5's u Δt / (a cos φ Δλ), which near the poles
   grows like 1/cos φ; the accepted HS T42 run (Δt 20 min, stable for 2200 days) logs up to
   3.78. For a spectral model the explicit limit is set by the total wavenumber, not the
   grid spacing at the pole. Proposed replacement: monitor |v|_max Δt √(M(M+1)) / a (or the
   same with the grid spacing at the equator) and set the trigger on that. Until decided,
   the trigger is not applied to Part I cases.
6. **APE Control: surface easterlies poleward of ~49°** (2026-11-16; P1 acceptance 5).
   - **What fails:** the gate requires lowest-level westerlies at every latitude from 35° to
     60°. At T42 the westerlies span 24°–48° (peak 7.8 m/s at 35°) and turn easterly
     from 49° (−4.2 m/s at 60°). At T21 the easterlies already begin at 41°. All other rows
     pass.
   - **Evidence:**
     - The whole eddy-driven circulation sits 10–15° equatorward of APE comprehensive
       models: a merged jet at 32°, storm-track variance near 40°, midlatitude rain at 33°.
     - The same pattern was present before P1: in the prototype gray aquaplanet and in TJ16
       moist HS (old and re-recorded M7), both with their own surface and BL schemes. The
       new PBL is therefore not the likely cause.
     - Going from T21 to T42 moves the turn to easterlies 8° poleward.
   - **Likely candidates:**
     - gray radiation with prescribed latitudinal optical depth (τ 6/1.5) and no clouds,
       which is replaced in P3–P4;
     - resolution: T63 is untested, about 2.5× the T42 cost;
     - the APE Control SST gradient is concentrated at 20–60° and falls to 0 °C at 60°.
   - **Options:**
     - (a) keep the row and expect it to be retested after P3/P4 radiation and clouds,
       continuing with P2 in the meantime. This needs the owner's go-ahead, because P2
       would start with a P1 row open.
     - (b) run T63 to test the resolution trend (~4 h).
     - (c) the owner revises the row for gray-radiation configurations (a criterion change,
       the owner's call).
   - **Recommendation:** (b) as data, then (a).
   - **Owner decision (2026-11-16): "Go ahead with b, then a".** T63 run started
     (`cases/bench/neale_hoskins2000/ape_control_t63.toml`, Δt 10 min). P2 may proceed with this
     row open; it is re-tested after P3/P4 and stays in the ratchet.
   - **Result of (b), with the transport change (2026-11-16):** years 2–4, NH; SH within 0.5°:

     | run | transport | westerlies | min u, 35–60° | u at 45° | Hadley edge | fixer |
     |---|---|---|---|---|---|---|
     | T21 | semi-Lagrangian | 19°–36° | −7.06 m/s at 47° | −6.05 | 16.8° | 0.17 %/day |
     | T42 | spectral | 27°–49° | −4.17 m/s at 60° | +3.45 | 23.6° | 2.29 %/day |
     | T42 | semi-Lagrangian | 27°–49° | −3.69 m/s at 60° | +4.17 | 24.3° | 0.12 %/day |
     | T63 (Δt 10 min) | spectral | 27°–51° | −2.55 m/s at 59° | +4.53 | 25.4° | 2.42 %/day |

     - Resolution moves the poleward edge of the surface westerlies 36° → 49° → 51° from
       T21 to T63 and weakens the 60° easterlies. The change from T42 to T63 is small, so
       the trend converges short of 60°: resolution alone does not close the row.
     - At T63 the jet (41 m/s at 34.5°), the storm-track eddy-variance maximum (~40°) and
       the Ferrel cell (~35°) are still 5–10° equatorward of comprehensive APE models.
     - The transport change does not move the circulation; it removes the §9.8 trigger
       (T42: 2.29 → 0.12 %/day).
     - Every other row passes in all four runs. Scorecards:
       `docs/scorecards/P1_aquaplanet_{t21_sl,t42_sl,t63}_2026-11-16.json`. Figures:
       `docs/figures/P1/ape_{t42_sl,t63}_{zonal_mean,moist}.png`.
     - Conclusion: per the owner's decision, the row stays open for the P3/P4 retest; P2
       proceeds.
7. **P2 bistability: one climate, with ice to ~45°** (2026-11-16; P2 acceptance 6).
   - **What fails:** the T21 aquaplanet (slab 20 m, sea ice, seasons, 1361 W/m², 12 years)
     reaches the same climate from both starts (`cases/bench/p2_bistability/`):
     - global T_s 279.3 K and global sea-ice cover 0.29 in both;
     - annual-mean sea ice reaches 42°N (concentration 0.2) and 47°S (0.55), full cover
       poleward of ~52°.
     - So the warm start does not stay ice-free equatorward of 50°, and the ice start does
       not stay ice-covered (it deglaciates in ~4 years).
     - T_s trend over the last 5 years is −0.13 / −0.07 K/yr (warm / ice start).
   - **Figures:** `docs/figures/P2/bistability_1361.png`, `bistability_1361_ts.png`.
   - **Likely candidates** (none tested yet):
     - the uniform TOA reflectance 0.23 (cloud stand-in) applies over sea ice too, on top of
       the ice albedo, so high latitudes absorb too little. Real clouds over ice add little
       to the albedo;
     - no ocean heat transport: the q-flux is zero in this case;
     - gray radiation with the Earth-shaped τ(φ) and no water-vapour feedback; P3 replaces
       it.
   - **Options:**
     - (a) keep the criterion and retest after P3/P4 (radiation, clouds), as for question 6;
     - (b) diagnostic runs, not adopted as a setting: the q-flux on, and the reflectance
       only over open ocean and land. This would be a physics change of the cloud
       stand-in, not tuning;
     - (c) the owner revises the criterion for the gray stand-in physics.
   - **Recommendation:** (b) as data, then (a).
   - **Instellation sweep** (acceptance 6 *record*): from the warm state (year 12) at 98, 96,
     94 and 92 % (luminosity scaled), 6 years each, climatology over years 15–18
     (`out/p2/sweep/`, binary f59dd7e). Ice edge = most equatorward latitude with
     annual-mean concentration ≥ 0.15:

     | Instellation | Global T_s | Global ice cover | Ice edge N / S | T_s trend, last 5 yr |
     |---|---|---|---|---|
     | 100 % (bistab_warm) | 279.3 K | 0.29 | 41.5° / 41.5° | −0.19 K/yr |
     | 98 % | 277.5 K | 0.30 | 41.5° / 41.5° | −0.01 K/yr |
     | 96 % | 275.4 K | 0.33 | 41.5° / 41.5° | −0.24 K/yr |
     | 94 % | 272.8 K | 0.37 | 36.0° / 36.0° | −0.44 K/yr |
     | 92 % | 270.3 K | 0.40 | 36.0° / 36.0° | −0.70 K/yr |

     - No run reaches a snowball within 6 years, and the tropics stay ice-free in all of
       them. The 94 % and 92 % runs are still cooling fast, so the freeze point is not
       found. It lies at or above 92 % if those runs keep going, and is undetermined
       otherwise.
     - Recording the transition would need longer runs (≥ 15 years) at 94–90 %. These are
       not started: question 7 is open and its physics may change.
