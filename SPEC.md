# toyclim — specification

A laptop-scale, qualitatively plausible general circulation model for worldbuilding,
including exoplanets (tidally locked, slow and fast rotators). Rust core, Godot viewer.

`toyclim` is a working name; rename freely.

This document is the contract for implementation sessions. It defines architecture,
conventions, the numerical method, and milestones with acceptance tests.

**How this document is organised (revised 2026-11-12)**

- **Part I, §1–§8: the dynamical core and viewer, milestones M0–M6.** Accepted. This is
  the regression base; its criteria must keep passing.
- **Part II, §9: Phase 2, milestones P0–P11.** Everything that turns the core into a
  world simulator: surface, boundary layer, snow and ice, radiation, clouds, ocean,
  validation gates, downscaling and biomes. **This is where current work is.** It replaces
  the earlier sketch of "M7–M12"; the code written under those labels is a prototype
  (§9.1).
- §10: session protocol. §11: references.

A session starts by reading this file, then `docs/progress.md` (status table), and
continues at the lowest unmet milestone: §9.6.

---

## 1. Goals and non-goals

**Goals**

- A plausible climate for a designed planet, on a laptop, overnight: winds, ocean
  currents, the diurnal and seasonal cycles, rain and snow, sea ice, biomes inferred from
  the climate, and local weather statistics at chosen places.
- Planets other than Earth: any rotation rate, tidally locked and resonant rotators,
  eccentric and oblique orbits, stars from M dwarfs to F stars, surface pressures from 0.1
  to 10 bar (the supported envelope is in §9.2).
- **Consistency, not realism:** in every supported regime the model must agree with
  published results of stronger, tuned models within stated tolerances (§9.2).
- Spectral transform dynamical core on the sphere (shallow water, then σ-coordinate
  primitive equations), T21–T85, with clean, tested operators.
- Planet, star, orbit and atmosphere parameters are data, not code.
- Deterministic: same config and same build give bitwise-identical output regardless of
  thread count.
- A headless runner for long spin-ups and experiments, and a Godot viewer that runs the
  model live in-process.
- Outputs readable from Python (numpy) for analysis and comparison with reference models
  (ExoPlaSim, SpeedyWeather, Isca).

**Non-goals**

- Reproducing Earth's weather or climate in detail; forecasting.
- A deep ocean circulation, cloud microphysics, atmospheric chemistry, vegetation dynamics.
- Atmospheres outside the envelope of §9.2 (Mars-like, Venus-like, non-water condensables)
  until P11.
- Nonhydrostatic dynamics, regional models, data assimilation.
- GPU compute. The design must not preclude it, but it is not a goal.

---

## 2. Repository layout

```
Cargo.toml                 workspace
crates/
  toyclim-core/            library: numerics, models, physics, I/O. No Godot dependency.
  toyclim-headless/        binary `toyclim`: runs cases, writes output, benchmarks
  toyclim-godot/           cdylib (gdext): thin wrapper exposing the model to Godot
godot/                     Godot project (viewer)
cases/                     TOML configs for test cases and experiments
tools/                     Python analysis and plotting (requirements.txt: numpy, matplotlib)
docs/
  progress.md              session log (see §9)
  figures/M<n>/            small PNGs produced by acceptance runs (committed)
reference/                 reference outputs supplied by the owner (git-ignored if large)
out/                       run output (git-ignored)
SPEC.md, CLAUDE.md, README.md
run                        script: build the extension and start the viewer
```

### toyclim-core module plan

This is the Part I plan. The target layout for Phase 2 (physics, surface, star/orbit) is
in §9.3.1 and wins where they differ.

```
planet.rs            Planet constants and derived quantities
config.rs            serde structs for TOML configs
spectral/
  gauss.rs           Gaussian latitudes and weights
  legendre.rs        normalized associated Legendre functions, ε, H tables
  packing.rs         triangular spectral storage and indexing
  fft.rs             real FFT wrapper per latitude row
  transform.rs       synthesis/analysis (scalar, div/curl of vectors, U/V from ψ/χ)
  ops.rs             Laplacian, inverse Laplacian, gradients, hyperdiffusion factors
grid.rs              Gaussian grid description, area weights, regridding to regular lat-lon
time/
  leapfrog.rs        leapfrog + RAW filter state handling
shallow_water/       state, tendencies, semi-implicit solver, test cases
primitive/           vertical coordinate, state, tendencies, semi-implicit solver, test cases
physics/
  column.rs          ColumnPhysics trait (see §5.6)
  held_suarez.rs     Held–Suarez forcing
  tidally_locked.rs  day/night Newtonian forcing
diagnostics.rs       global integrals, conservation checks, CFL
io/
  npy.rs             minimal .npy writer/reader (write it; the format is trivial)
  meta.rs            JSON metadata sidecars
  restart.rs         restart files (spectral state as .npy + meta.json)
```

### Dependencies

Allowed without discussion: `rustfft` or `realfft`, `num-complex`, `rayon`, `serde`,
`serde_json`, `toml`, `anyhow`, `thiserror`, `clap`, `log` + `env_logger`; dev: `approx`,
`criterion`; `godot` (gdext) in `toyclim-godot` only. Anything else: note the reason in
`docs/progress.md`. No `unsafe` outside the gdext crate.

---

## 3. Conventions

- **Units:** SI throughout. Time in seconds internally; configs may use `"30min"`, `"1day"`
  strings parsed by `config.rs`.
- **Coordinates:** longitude λ ∈ [0, 2π) eastward, latitude φ, μ = sin φ. Grid index 0 is
  the northernmost latitude. Arrays are row-major `[lat][lon]` for 2D and
  `[lev][lat][lon]` for 3D. Level 0 is the model top.
- **Precision:** a type alias `Real` (f64 by default; cargo feature `f32` switches it).
  Acceptance tests run in f64 unless stated otherwise.
- **Velocity components:** the code works with U = u cos φ and V = v cos φ, which are
  smooth at the poles.
- **Planet defaults (Earth, Williamson et al. 1992 values):** a = 6.37122e6 m,
  Ω = 7.292e-5 s⁻¹, g = 9.80616 m s⁻², R_d = 287.04 J kg⁻¹ K⁻¹, c_p = 1004.64 J kg⁻¹ K⁻¹,
  κ = R_d/c_p, p₀ = 1e5 Pa.
- **Determinism:** never reduce in parallel with a non-fixed order.
  - Parallelize the Legendre transform over m.
  - Parallelize FFTs and grid-point physics over latitude rows or columns.
  - Global sums are computed serially or in fixed-order chunks.
  - A test (M6) checks bitwise identity between 1 and N threads.

---

## 4. Spectral transform method

### 4.1 Truncation and grid

Triangular truncation T_M. Spectral coefficients f_ℓ^m for 0 ≤ m ≤ ℓ ≤ M (only m ≥ 0
stored; for real fields f_ℓ^{−m} = conj(f_ℓ^m)).

Store them m-major, contiguous in ℓ, with a helper `idx(l, m)`. Allow an extra degree
L = M + 1 where a routine needs it internally.

The Gaussian grid must be alias-free for quadratic terms: N_lat ≥ (3M+1)/2 and
N_lon ≥ 3M+1, rounded up to FFT-friendly sizes.

| T | N_lat × N_lon |
|---|---|
| 21 | 32 × 64 |
| 31 | 48 × 96 |
| 42 | 64 × 128 |
| 63 | 96 × 192 |
| 85 | 128 × 256 |
| 127 | 192 × 384 |
| 170 | 256 × 512 |

Gaussian latitudes are the roots μ_j of P_{N_lat}(μ), found by Newton iteration from
asymptotic initial guesses. The weights are w_j = 2 / ((1 − μ_j²) [P'_{N_lat}(μ_j)]²),
so Σ w_j = 2.

### 4.2 Legendre functions

Normalization: ∫_{−1}^{1} P̄_ℓ^m P̄_{ℓ'}^m dμ = δ_{ℓℓ'}, with **no** Condon–Shortley phase.

- P̄_0^0 = 1/√2
- P̄_m^m = √((2m+1)/(2m)) · √(1−μ²) · P̄_{m−1}^{m−1}
- ε_ℓ^m = √((ℓ² − m²)/(4ℓ² − 1))
- P̄_ℓ^m = (μ P̄_{ℓ−1}^m − ε_{ℓ−1}^m P̄_{ℓ−2}^m) / ε_ℓ^m   (gives P̄_{m+1}^m = √(2m+3) μ P̄_m^m)
- H_ℓ^m ≡ (1 − μ²) dP̄_ℓ^m/dμ = −ℓ ε_{ℓ+1}^m P̄_{ℓ+1}^m + (ℓ+1) ε_ℓ^m P̄_{ℓ−1}^m

Precompute P̄ and H on the Gaussian latitudes for ℓ ≤ M (P̄ up to M+1 internally for H).
Exploit the symmetry P̄_ℓ^m(−μ) = (−1)^{ℓ+m} P̄_ℓ^m(μ) to halve the cost (optional,
recommended).

### 4.3 Transforms

- **Fourier:** F_m(μ_j) = (1/N_lon) Σ_k f(λ_k, μ_j) e^{−imλ_k}, and
  f = Σ_{m=−M}^{M} F_m e^{imλ}. Mind the scaling of the FFT library.
- **Synthesis:** F_m(μ_j) = Σ_{ℓ=m}^{M} f_ℓ^m P̄_ℓ^m(μ_j), then inverse FFT.
- **Analysis:** f_ℓ^m = Σ_j w_j F_m(μ_j) P̄_ℓ^m(μ_j).

### 4.4 Operators

- **Laplacian:** ∇²Y_ℓ^m = −ℓ(ℓ+1)/a² · Y_ℓ^m. The inverse sets ℓ = 0 to zero.

- **Winds from ψ, χ (synthesis via H; no M+1 row in storage):**
  - U = (1/a)[ ∂χ/∂λ − (1−μ²) ∂ψ/∂μ ]
  - V = (1/a)[ ∂ψ/∂λ + (1−μ²) ∂χ/∂μ ]

  On the grid, U = (1/a)[ Σ i m χ_ℓ^m P̄_ℓ^m e^{imλ} − Σ ψ_ℓ^m H_ℓ^m e^{imλ} ], and
  similarly for V.

- **Divergence and curl of a cos φ-weighted vector (A, B), analysis with integration by
  parts:**
  - D = (1/(a(1−μ²))) [ ∂A/∂λ + (1−μ²) ∂B/∂μ ]:
    D_ℓ^m = (1/a) Σ_j (w_j/(1−μ_j²)) [ i m A_m(μ_j) P̄_ℓ^m(μ_j) − B_m(μ_j) H_ℓ^m(μ_j) ]
  - C = (1/(a(1−μ²))) [ ∂B/∂λ − (1−μ²) ∂A/∂μ ]:
    C_ℓ^m = (1/a) Σ_j (w_j/(1−μ_j²)) [ i m B_m(μ_j) P̄_ℓ^m(μ_j) + A_m(μ_j) H_ℓ^m(μ_j) ]

  So ∇·(X v) = D(XU, XV) and k·∇×(X v) = C(XU, XV).

- **cos φ-weighted gradient of a scalar X:** (1/a) ∂X/∂λ and (1/a)(1−μ²) ∂X/∂μ, both
  synthesized via i m P̄ and H respectively.

- **Advection:** v·∇X = [U · (1/a)∂X/∂λ + V · (1/a)(1−μ²)∂X/∂μ] / (1−μ²).

- **Kinetic energy:** E = (U² + V²) / (2(1−μ²)).

- **Global integral:** ∫X dA = 2π a² Σ_j w_j · mean_k X(λ_k, μ_j).

### 4.5 Time stepping

- **Scheme:** leapfrog with semi-implicit treatment of gravity-wave terms (§5) and the
  Robert–Asselin–Williams (RAW) filter. After computing x^{n+1}:
  - d = (ν/2)(x^{n−1} − 2x^n + x^{n+1})
  - x^n += α d
  - x^{n+1} −= (1−α) d
  - Defaults ν = 0.1, α = 0.53 (Williams 2009).
- **First step:** forward step of length Δt (x⁻ = x⁰, half-length semi-implicit step).
- **Implicit hyperdiffusion** applied after each step to ζ, δ and the thermodynamic variable
  (never to ℓ = 0):
  x_ℓ ← x_ℓ / (1 + 2Δt · τ_h⁻¹ · [ℓ(ℓ+1) / (M(M+1))]^p).
  Defaults p = 4 (∇⁸), τ_h = 2.4 h at T42; configurable.
- **CFL monitor:** log the max Courant number U Δt / (a cos φ Δλ) per output step, and
  abort cleanly (with a restart dump) on NaN/Inf.

---

## 5. Models

### 5.1 Shallow water

State: ζ, δ, φ' (spectral), where φ = g h is the layer-depth geopotential and
φ' = φ − φ̄. h_s is the bottom topography.

Continuous equations:

- ∂ζ/∂t = −∇·[(ζ+f) v]
- ∂δ/∂t = k·∇×[(ζ+f) v] − ∇²(φ + g h_s + E)
- ∂φ/∂t = −∇·(φ v)

Nonlinear (explicit) parts, with η = ζ + f:

- N_ζ = −D(ηU, ηV)
- N_δ = C(ηU, ηV) − ∇²(g h_s + E)
- N_φ = −D(φ'U, φ'V)

Implicit linear parts: −∇²φ' in the δ equation, −φ̄δ in the φ equation.

Semi-implicit solve per ℓ, with Λ_ℓ = ℓ(ℓ+1)/a²:

1. x (1 + Δt² Λ_ℓ φ̄) = δ⁻ + Δt [ N_δ + Λ_ℓ (φ'⁻ + Δt N_φ) ]
2. δ⁺ = 2x − δ⁻
3. φ'⁺ = φ'⁻ + 2Δt (N_φ − φ̄ x)
4. ζ⁺ = ζ⁻ + 2Δt N_ζ

Diagnostics:

- mass ∫φ dA / g (must be conserved to round-off: it is the ℓ = 0 coefficient)
- total energy ∫ [φ (u²+v²)/2 + φ²/2 + φ g h_s] / g dA
- potential enstrophy ∫ g (ζ+f)² / (2φ) dA
- axial angular momentum

### 5.2 Primitive equations, σ coordinates

State (spectral): ζ_k, δ_k, T'_k = T_k − T̄ (T̄ isothermal reference, default 300 K,
configurable per planet; it must be at least as warm as typical temperatures for
stability), and ln p_s.

Grid: Lorenz staggering.

- L full levels σ_k, half levels σ_{k+1/2}, with σ_{1/2} = 0 and σ_{L+1/2} = 1.
- Δσ_k = σ_{k+1/2} − σ_{k−1/2}.
- Discretization: Simmons & Burridge (1981), which in pure σ simplifies to:
  - α_1 = ln 2; α_k = 1 − (σ_{k−1/2}/Δσ_k) ln(σ_{k+1/2}/σ_{k−1/2}) for k > 1
  - D_k = δ_k + v_k·∇ln p_s
  - **Hydrostatic:** Φ_k = Φ_s + Σ_{j>k} R T_j ln(σ_{j+1/2}/σ_{j−1/2}) + α_k R T_k
  - **Surface pressure:** ∂ln p_s/∂t = −Σ_k Δσ_k D_k
  - **Vertical velocity:** σ̇_{k+1/2} = σ_{k+1/2} Σ_{j=1}^{L} Δσ_j D_j − Σ_{j=1}^{k} Δσ_j D_j
  - **Vertical advection:** (σ̇ ∂X/∂σ)_k =
    [σ̇_{k+1/2}(X_{k+1} − X_k) + σ̇_{k−1/2}(X_k − X_{k−1})] / (2Δσ_k)
  - (ω/p)_k = v_k·∇ln p_s − (1/Δσ_k) [ ln(σ_{k+1/2}/σ_{k−1/2}) Σ_{j<k} Δσ_j D_j + α_k Δσ_k D_k ]
  - **Pressure gradient force:** R T_k ∇ln p_s (exact in pure σ with this discretization)

Momentum, written with Q_k = (ζ_k + f) k × v_k + (σ̇ ∂v/∂σ)_k + R T'_k ∇ln p_s:

- ∂ζ_k/∂t = −k·∇×Q_k + F_ζ
- ∂δ_k/∂t = −∇·Q_k − ∇²(Φ_s + Φ'_k + R T̄ ln p_s + E_k) + F_δ, where Φ'_k = (G T')_k

In terms of §4.4: with η = ζ + f, ηk×v has cos φ-weighted components (−ηV, ηU).

Thermodynamics:

- ∂T_k/∂t = −v_k·∇T'_k − (σ̇ ∂T'/∂σ)_k + κ T_k (ω/p)_k + Q_T/c_p

### 5.3 Semi-implicit scheme for the primitive equations

Linearize about T̄ and a uniform reference ln p_s. Define:

- G (L×L): G_{kj} = R ln(σ_{j+1/2}/σ_{j−1/2}) for j > k; G_{kk} = R α_k; 0 for j < k
- τ (L×L): τ_{kj} = κ T̄ ln(σ_{k+1/2}/σ_{k−1/2}) Δσ_j / Δσ_k for j < k; τ_{kk} = κ T̄ α_k; 0 for j > k
  (so the linear T tendency, κ T̄ (ω/p)_lin, is −τδ)
- w (L): w_j = Δσ_j
- **1** (L): a vector of ones

Linear parts:

- δ_t ⊃ Λ_ℓ (G T' + R T̄ **1** ln p_s)
- T'_t ⊃ −τ δ
- (ln p_s)_t ⊃ −wᵀ δ

N_δ, N_T, N_p are the full tendencies minus these linear parts, evaluated at time n.

For each ℓ, with S_ℓ = I + Δt² Λ_ℓ (G τ + R T̄ **1** wᵀ):

1. S_ℓ x = δ⁻ + Δt [ N_δ + Λ_ℓ ( G (T'⁻ + Δt N_T) + R T̄ **1** (ln p_s⁻ + Δt N_p) ) ]
2. δ⁺ = 2x − δ⁻
3. T'⁺ = T'⁻ + 2Δt (N_T − τ x)
4. ln p_s⁺ = ln p_s⁻ + 2Δt (N_p − wᵀ x)

Precompute the S_ℓ⁻¹ (or LU factors) for all ℓ whenever Δt changes. Verify the signs
against a linear gravity-wave test: small perturbations at rest on a non-rotating planet
must oscillate at the analytic frequencies of the vertical normal modes of the discrete
system (eigenvalues of Gτ + R T̄ **1** wᵀ) and must not grow.

### 5.4 Column physics interface (designed now, used from M5 on)

(Part I interface. Phase 2 replaces it with the sequential driver of §9.3.3.)

Grid-point physics works on independent columns:

```rust
pub struct ColumnIn<'a> { lat: Real, lon: Real, sigma: &'a [Real], ps: Real,
                          u: &'a [Real], v: &'a [Real], t: &'a [Real], /* q later */ time: Real }
pub struct ColumnOut<'a> { du: &'a mut [Real], dv: &'a mut [Real], dt: &'a mut [Real] }
pub trait ColumnPhysics: Send + Sync {
    fn tendencies(&self, planet: &Planet, col: &ColumnIn, out: &mut ColumnOut);
}
```

The model calls a `Vec<Box<dyn ColumnPhysics>>` in a fixed order, in parallel over columns,
and adds the tendencies to the grid-point tendencies before analysis. Vorticity and
divergence tendencies come from (du, dv) via C and D on (du cos φ, dv cos φ). The
interface must leave room for humidity, surface fields and per-column diagnostics (Phase 2)
without changing call sites; using a struct of slices or a builder is fine.

### 5.5 Insolation geometry (needed in M5; star and orbit front end in §9.3.2)

A `Star`/`Orbit` description provides the substellar point (λ_ss(t), φ_ss(t)).

- Tidally locked: λ_ss is constant (default π), φ_ss = 0.
- Otherwise: λ_ss advances at the synodic rate; obliquity and eccentricity are implemented in `orbit.rs` (prototype, kept).

Ω may be zero or negative (retrograde).

---

## 6. Output, configs and the headless runner

- **CLI:**
  - `toyclim run --config cases/<name>.toml --out out/<run>`
  - `toyclim bench --config ...`
  - `toyclim list-cases`
- **Config (TOML):** model type, truncation, levels, Δt, run length, output interval and
  fields, planet block, forcing block, diffusion. Example:

  ```toml
  model = "shallow_water"
  truncation = 42
  dt = "30min"
  length = "5day"
  output_every = "6h"
  output_fields = ["h", "vor", "u", "v"]
  initial_condition = { case = "williamson2", alpha = 0.0 }
  [planet]
  preset = "earth"         # or explicit radius/rotation_rate/gravity/...
  [diffusion]
  order = 4
  tau = "2.4h"
  ```

- **Output:**
  - `out/<run>/meta.json` (grid latitudes and longitudes, σ levels, planet, config echo,
    git hash)
  - `out/<run>/fields/<field>_<step:06>.npy` (float32 grid fields, Gaussian grid)
  - `out/<run>/diagnostics.csv` (time, mass, energy, enstrophy, AM, max Courant, ...)
- **Stations** (added 2026-11-04 with the owner's approval, see `docs/design-decisions.md`):
  `[[stations]]` entries (`name`, `lat`, `lon` in degrees), `station_every` (default: every
  step) and `station_fields` (default: all fields). Fields are interpolated bilinearly from
  the Gaussian grid and written to `out/<run>/stations/<field>.npy` (float32,
  `[time][station]` or `[time][station][lev]`), `time.npy` (float64 s) and `stations.json`.
  A run continued from a restart into the same directory extends the series bitwise
  identically to an uninterrupted run. `tools/station_stats.py` plots series and
  distributions.
- **Restart:** spectral state (both leapfrog levels) as .npy plus meta.json; loading a
  restart and continuing must be bitwise identical to an uninterrupted run.
- **Python:**
  - `tools/plot_field.py` (map, polar view, zonal mean)
  - `tools/zonal_mean.py` (time-mean zonal-mean cross sections for PE runs)
  - `tools/compare.py` (error norms against an analytic or reference field)

---

## 7. Godot viewer (toyclim-godot + godot/)

- Current stable Godot 4.x (standard single-precision build is fine) and the matching
  `godot` crate (gdext). Record the exact versions in `docs/progress.md`.
- **Threading:** a GDExtension class `ClimateModel` (Node) owns the model on a Rust worker
  thread.
  - Commands: load config, run/pause, steps per second or speed multiplier, single step,
    save restart.
  - The worker publishes snapshots through a double buffer (`Mutex<Arc<Snapshot>>` swap is
    fine); the main thread never blocks on the model.
- **Data handoff:** for display, Rust regrids requested fields from the Gaussian grid to a
  regular equirectangular grid (linear in latitude, 2× N_lon by N_lon) and returns a
  `PackedFloat32Array` or directly an `Image` (FORMAT_RF). GDScript only uploads it to an
  `ImageTexture`.
- **Rendering:**
  - Sphere mesh with a shader sampling the field texture; colormap as a 1D texture; min/max
    as uniforms (auto or fixed).
  - Toggle to a flat equirectangular map.
  - Orbit camera.
  - Coastline/orography overlay is a stretch goal.
- **UI:** field selector, level selector (PE), colormap range, run/pause, speed, step, model
  time and key diagnostics (mass, energy, max Courant). Keep the UI in GDScript, small.
- **Later (not M3):** wind particles or streamlines, vertical cross-section panel,
  multiple planets side by side.

---

## 8. Milestones and acceptance tests (Part I: M0–M6, accepted)

M0–M6 are accepted (numbers in `docs/progress.md`); one item is open (f32 Held–Suarez
asymmetry, handled in P0). These criteria are the regression base for Phase 2.

Every milestone ends with:

- all tests green (`cargo test`, plus `cargo test --release -- --ignored` for long tests
  listed in the milestone);
- `cargo clippy -- -D warnings` and `cargo fmt --check` clean;
- figures written to `docs/figures/M<n>/`;
- an entry in `docs/progress.md` with the measured numbers.

Numbers are acceptance criteria unless marked *record*, which means measure, write down and
compare qualitatively with the cited figure.

Before implementing a published test case, fetch the paper (or a reliable copy) and
transcribe its constants into `cases/<name>.toml` with the citation. The formulas below
are given for convenience; **the paper wins if they disagree**. Note any discrepancy in
`docs/progress.md`.

### M0 — Skeleton

- Workspace with the three crates, `cases/`, `tools/`, `docs/progress.md`, `.gitignore`,
  README with build/run instructions.
- Config parsing with duration strings; .npy writer and reader with a round-trip test;
  meta.json writer.
- `toyclim run` on a dummy case writes one analytic field; `tools/plot_field.py` renders
  it to PNG.
- A GitHub Actions workflow running fmt, clippy and tests (non-ignored).

**Accept:** CI config present; all of the above runs locally in the session.

### M1 — Spectral transforms

Implement §4.1–4.4 with unit tests:

1. **Gaussian quadrature:** Σ w_j = 2 to 1e-14; integrates μ^n exactly (1e-13) for
   n ≤ 2N_lat − 1.
2. **Orthonormality:** Σ_j w_j P̄_ℓ^m P̄_{ℓ'}^m = δ to 1e-13 for T170.
3. **Round trip:** analysis(synthesis(random band-limited f)) for T21, T42, T85, T170,
   max relative error < 1e-12.
4. **Laplacian:** ∇² of synthesized Y_ℓ^m matches −ℓ(ℓ+1)/a² (via U/V and div of the
   gradient) to 1e-10.
5. **Wind round trip:** random ψ, χ → U, V (grid) → C(U,V), D(U,V) gives ∇²ψ, ∇²χ, relative
   error < 1e-10.
6. **Solid-body rotation:** ψ = −a u₀ sin φ gives u = u₀ cos φ, v = 0 on the grid to 1e-12
   relative.
7. **Benchmarks (criterion):** synthesis and analysis per field at T42 and T85. *Record.*

**Accept:** tests 1–6 pass; benchmark numbers recorded.

### M2 — Shallow-water model

Implement §5.1 and the test cases below as configs, using Earth constants from §3.

**Williamson et al. (1992) test 2** (steady geostrophic flow), α ∈ {0, π/2 − 0.05}:

- u₀ = 2πa / 12 days, g h₀ = 2.94e4 m² s⁻²
- ψ = −a u₀ (sin φ cos α − cos λ cos φ sin α)
- g h = g h₀ − (aΩu₀ + u₀²/2)(−cos λ cos φ sin α + sin φ cos α)²
- Run 5 days at T42, Δt = 30 min (also T21 and T85 for a convergence table).

**Accept:** normalized l2(h) < 1e-5 (f64) at day 5 for both α; mass relative drift < 1e-12.
Error norms l1, l2, l∞ are as defined in Williamson et al.

**Test 5** (zonal flow over an isolated mountain):

- u₀ = 20 m/s, α = 0, h₀ = 5960 m
- Mountain h_s = 2000 m · (1 − r/R), R = π/9, centre (λ_c, φ_c) = (3π/2, π/6),
  r = min(R, √((λ−λ_c)² + (φ−φ_c)²))
- g(h + h_s) = g h₀ − (aΩu₀ + u₀²/2) sin²φ
- Run 15 days at T42 and T85.

**Accept:** stable for 15 days; mass drift < 1e-12; total-energy change < 1% at T85.
*Record* plots of h at days 5, 10 and 15 and compare with published figures.

**Test 6** (Rossby–Haurwitz wave, R = 4):

- ω = K = 7.848e-6 s⁻¹, h₀ = 8000 m
- ψ = −a²ω sin φ + a²K cos^R φ sin φ cos Rλ
- g h = g h₀ + a²A(φ) + a²B(φ) cos Rλ + a²C(φ) cos 2Rλ, with
  - A = (ω/2)(2Ω+ω) cos²φ + (K²/4) cos^{2R}φ [(R+1) cos²φ + (2R² − R − 2) − 2R² cos^{−2}φ]
  - B = [2(Ω+ω)K / ((R+1)(R+2))] cos^R φ [(R² + 2R + 2) − (R+1)² cos²φ]
  - C = (K²/4) cos^{2R}φ [(R+1) cos²φ − (R+2)]
- Cross-check: solving the nonlinear balance equation spectrally,
  ∇²(φ + E) = C(ηU, ηV) with δ = 0, must reproduce this h up to a constant to 1e-8
  relative.
- Run 14 days at T42.

**Accept:**

- the balance cross-check passes;
- the wave-4 pattern is retained at day 14;
- energy and potential-enstrophy drift < 1%;
- the eastward phase speed is within 15% of the nondivergent value
  ν = [R(3+R)ω − 2Ω] / [(1+R)(2+R)] ≈ 12.2°/day. *Record* the measured speed.
- The R = 4 wave is known to be unstable on long runs (Thuburn & Li 2000), so do not
  test beyond 14 days.

**Galewsky et al. (2004) barotropic instability:**

- u = (u_max/e_n) exp[1/((φ−φ₀)(φ−φ₁))] for φ₀ < φ < φ₁, else 0
- u_max = 80 m/s, φ₀ = π/7, φ₁ = π/2 − φ₀, e_n = exp(−4/(φ₁−φ₀)²)
- h from gradient balance g h(φ) = g h₀ − ∫ a u (f + u tan φ / a) dφ (numerical quadrature),
  with h₀ set so the global mean depth is 10 km.
- Perturbation h' = 120 m · cos φ · exp(−(λ/α)²) · exp(−((φ₂−φ)/β)²), with
  φ₂ = π/4, α = 1/3, β = 1/15, λ ∈ (−π, π].
- T85 (and T127 if affordable), 6 days.

**Accept:**

- unperturbed run: max |Δζ| / max|ζ| < 1% at day 6;
- perturbed run: the jet rolls up into vortices by day 6.

*Record* the relative vorticity map at day 6 and compare with Galewsky et al. Fig. 4.

**Performance:** T42 shallow water single-threaded ≥ 10 model days per wall second.
*Record* the actual number.

### M3 — Minimal Godot viewer

Implement §7 for the shallow-water model: load a case, run live, show h/ζ/u/v on the
sphere and the flat map, with run/pause/speed/step.

**Accept (cloud, no display):**

- `cargo build -p toyclim-godot --release` succeeds;
- `godot --headless --path godot --quit-after 600` exits 0 and the log shows the model
  advancing (download the official Linux Godot build if needed);
- **optional:** a screenshot via `xvfb-run` with the `opengl3` driver and software
  rendering, saved to `docs/figures/M3/`.

The owner checks visuals locally. Keep a `run` script that builds the extension and
starts the viewer.

### M4 — Dry primitive-equation core

Implement §5.2–5.3 with σ levels from config.

1. **Resting atmosphere:** isothermal, uniform p_s, no forcing, Ω arbitrary. Stays at
   rest (max |u| < 1e-10 m/s after 10 days).
2. **Linear gravity waves:** non-rotating, small T' perturbation. Oscillation frequencies
   match the discrete normal-mode prediction to 1%, no growth over 30 days.
3. **Conservation (adiabatic, inviscid except hyperdiffusion):** mass drift < 1e-10
   relative over 30 days. *Record* the drift in total energy ∫(c_p T + Φ_s + E) dm and
   angular momentum.
4. **Jablonowski & Williamson (2006) baroclinic wave**, T85, 26 levels evaluated at
   σ = η (p_s uniform 1000 hPa initially):
   - unperturbed: zonal-mean u changes by < 0.5 m/s anywhere after 10 days;
   - perturbed: *record* p_s and 850 hPa T at days 7, 9 and 11, and the minimum p_s at
     day 9; compare with the paper's figures and reported spread.

**Accept:** tests 1–3 pass; test 4 recorded with plots and the unperturbed criterion met.

### M5 — Held–Suarez and tidally locked forcing

**Held & Suarez (1994)** as `ColumnPhysics`:

- T_eq = max{200 K, [315 K − ΔT_y sin²φ − Δθ_z ln(p/p₀) cos²φ] (p/p₀)^κ},
  with ΔT_y = 60 K and Δθ_z = 10 K
- k_T = k_a + (k_s − k_a) max(0, (σ−σ_b)/(1−σ_b)) cos⁴φ, with k_a = 1/40 d⁻¹ and
  k_s = 1/4 d⁻¹
- Rayleigh friction k_v = k_f max(0, (σ−σ_b)/(1−σ_b)), with k_f = 1 d⁻¹ and σ_b = 0.7
- Flat surface, p_s = 1000 hPa initially, small random T perturbation.
- T42 L20 (equally spaced σ), 1200 days; discard 200; time-mean zonal-mean statistics.

**Accept:**

- one westerly jet per hemisphere, maximum 26–34 m/s, located at 38°–50° latitude and
  σ 0.15–0.35;
- easterlies at the lowest level in the tropics and westerlies in mid-latitudes;
- hemispheric asymmetry of zonal-mean u < 3 m/s everywhere;
- no trend in global kinetic energy over the last 1000 days (< 5%).

*Record* the zonal-mean u, T and eddy variance cross sections and compare with HS94
Figs. 1–2. If the owner places SpeedyWeather or Isca Held–Suarez output in `reference/`,
compare against it as well.

**Tidally locked variant** (our definition, in the spirit of Merlis & Schneider 2010 and
Heng, Menou & Phillipps 2011):

- T_eq = max{200 K, [T_n + ΔT_d max(0, cos φ cos(λ − λ_ss)) − Δθ_z ln(p/p₀)] (p/p₀)^κ}
- Defaults T_n = 250 K, ΔT_d = 70 K, Δθ_z = 10 K; same damping and friction as HS
  (cos⁴φ factor replaced by max(0, cos φ cos(λ − λ_ss))⁴ on the day side and 0 on the
  night side).
- Runs at Ω/Ω⊕ ∈ {1, 0.1, 0.02}, T42 L20, 800 days.

*Record*, and expect qualitatively:

- at Ω⊕: equatorial superrotation at upper levels (zonal-mean u at the equator > 0 above
  σ = 0.5) and a Matsuno–Gill-like stationary pattern;
- at slow rotation: a day-to-night overturning circulation and much weaker horizontal
  temperature contrasts in the free troposphere than near the surface (weak temperature
  gradient regime).

**Rotation sweep for HS:** Ω/Ω⊕ ∈ {0.25, 1, 2} at T42 (T85 for 2), 800 days each.
*Record* the jet latitudes, jet count and Hadley-cell extent (from the mean meridional
streamfunction). Expect wider Hadley cells and poleward jets at slow rotation, and
equatorward or multiple jets at fast rotation.

### M6 — Performance and robustness

- Parallelize per §3. Determinism test: 10-day PE run with `RAYON_NUM_THREADS=1` and with
  N threads gives byte-identical output (integration test, `--ignored`).
- Restart test: stop and continue gives byte-identical output.
- `f32` feature builds, and HS still meets the M5 jet criteria in f32. *Record* the
  speedup.
- **Budget:** HS T42 L20 one model year in ≤ 10 min on 8 threads of the owner's laptop
  (i7-11850H). Measure in the cloud and *record*, noting the core count (`nproc`).
- The viewer runs the PE model with level selection.

---

## 9. Phase 2 — from prototype to validated world simulator

Written 2026-11-12 after an architecture and plan audit. This section **replaces** the old
sketch of milestones M7–M12. It is the contract for all work after M6.

### 9.1 Status and order of work

M0–M6 (Part I, §1–§8) are accepted and are now the **regression base**: their tests and
acceptance numbers must keep passing.

The code written on 2026-11-04/05 under the labels "M7–M11 first steps" is a **prototype**.
It runs and conserves energy and water, but it has no acceptance criteria, and the world
runs of 2026-11-12 show defects (see §9.1.1). It is raw material for Phase 2, not accepted
work. What happens to each piece:

| Prototype piece | Fate |
|---|---|
| Tracers, negative fixer, `Condensable` | keep; extend in P2 (fusion), monitor fixer (§9.8) |
| TJ16 moist Held–Suarez bundle | keep unchanged as an idealized regression case |
| Simplified Betts–Miller convection | keep; re-validated in P1/P5 |
| Gray radiation (Frierson) | keep as `scheme = "gray"` for idealized comparators only |
| Orbit, calendar | keep; wrapped by the star/orbit front end in P0 |
| Slab ocean, bucket, ice-albedo switch inside `primitive/mod.rs` | replaced by `surface/` (P0 move, P2 rewrite) |
| Rayleigh friction as the momentum sink in world runs | replaced by surface stress + boundary layer (P1) |
| Diagnostic RH clouds (experimental) | replaced in P4 |
| Prescribed zonal q-flux | kept as an option; superseded by P6 |
| Python tools (Köppen, downscaling, point rain, export) | keep; calibrated and extended in P8 |

Phase 2 milestones are **P0–P11** (§9.6), done in order. P0–P9 are authorised by the owner
(2026-11-12). **P10 and P11 need the owner's go-ahead.** Branches are named
`p<n>-<short-name>`; figures go to `docs/figures/P<n>/`.

#### 9.1.1 Known defects of the prototype (evidence: `docs/figures/M9/testworld_t42_*.png`)

These are the first things Phase 2 must make go away. Each is tied to a metric in §9.5.

1. **Grid-point storms over tropical land.** Time-mean precipitation has single-cell maxima
   near 40 mm/day; the Köppen map there is a cell-scale mosaic of Af and BWh.
   Hypotheses to test, in this order: (a) dry bucket (β → 0) switching convection off
   around a cell that stays wet; (b) explicit surface temperature step with a small land
   heat capacity; (c) convection and large-scale condensation both acting on the same
   input state; (d) the convection scheme's parameters over land.
2. **Wrong continental climate.** 40–60°N is mostly Dsa (hot summer, dry summer); Earth-like
   continents there should be mostly Dfb/Dfc.
3. **Too much ice.** Open ocean is EF poleward of about 55°S: the albedo is a hard switch to
   0.6 at 271 K on a 20 m slab with no sea-ice thermodynamics.
4. **Optical depth is a function of latitude** (`τ₀(φ)` in the gray scheme). This is an
   Earth-shaped prescription and is meaningless on a tidally locked planet.
5. `tools/tidally_locked.py` fails on `tidally_locked_gray` (no `t500` climatology map).

### 9.2 Principles

**P-1. What "right" means.** toyclim is not expected to reproduce reality. It is expected
to be **consistent with published results of stronger, tuned models** in every regime it
claims to support. Every milestone names its comparators.

**P-2. The spread rule.** When a benchmark has results from several models for a scalar X
with range [X_min, X_max] and width w = X_max − X_min, toyclim passes if
X ∈ [X_min − w/2, X_max + w/2]. When only one model is published:

- if its physics is the same as ours (e.g. gray radiation + SBM in FMS): within ±15% of the
  value, or ±3 K for temperatures, or ±3° for latitudes;
- otherwise: within ±30%, ±6 K, ±6°.

For trends across a parameter sweep the criterion is the **sign and monotonicity** of the
trend. For maps it is a pattern correlation with the threshold given in the milestone.
Milestones may state tighter or looser numbers explicitly; those win over this rule.

**P-3. Papers win.** Formulas and constants in this section were written from memory for
convenience. Before implementing a scheme or a benchmark, fetch the paper and transcribe
its constants into the case file or the scheme's doc comment with the citation
(CLAUDE.md rule). If a paper does not contain what this document attributes to it, say so
in `docs/progress.md` and use the correct source; do not invent a number.

**P-4. Tuning governance.**

- Tunable parameters are listed in `docs/tuning.md` (created in P0) with: name, default,
  allowed range, source, and the date and reason of every change.
- Tuning is done **only on Earth targets** (P4, P5, P6). Values are then frozen.
- **Never tune on an exoplanet benchmark.** If an exoplanet gate fails, the fix is physics
  or a recorded open question, not a knob.
- A knob may not be made a function of latitude, longitude or planet name.

**P-5. Supported envelope.** Inside it, toyclim must pass its gates. Outside it, the model
must refuse to run or warn loudly (config validation), not silently produce numbers.

| Quantity | Supported range |
|---|---|
| Mean surface pressure | 0.1–10 bar |
| Background gas | N₂/O₂/Ar-like mix with CO₂ up to 1% by volume |
| Condensable | water, dilute: q ≤ 0.1 kg/kg (abort with a message beyond that) |
| Instellation | 0.3–1.5 S⊕ (no runaway greenhouse, no CO₂ condensation) |
| Stellar T_eff | 2500–7000 K, blackbody spectrum |
| Rotation | any period ≥ 6 h, either sign, including synchronous and p:q resonances |
| Radius, gravity | 0.5–2 R⊕, 3–25 m s⁻² |
| Obliquity, eccentricity | 0–90°, 0–0.4 |
| Resolution | T21 (preview), T42 (production), T85 (dry or short runs only) |

Mars-like and Venus-like atmospheres, non-water condensables, non-dilute moisture,
several stars and moons are P11 (interfaces stay open, nothing is tested).

**P-6. Look at what you produce.** Every figure a session creates must be opened and
described in one sentence in `docs/progress.md`, including anything implausible. A figure
that was not looked at does not count as a recorded result.

**P-7. Ratchet.** A criterion that passed at an earlier milestone must still pass. Each
milestone ends by re-running the regression suite (§9.5.4).

### 9.3 Target architecture

#### 9.3.1 Module layout of `toyclim-core`

```
spectral/ grid.rs time/            unchanged (Part I)
shallow_water/                     unchanged
primitive/
  mod.rs          the model struct, step(), Model impl          (≤ 500 lines)
  state.rs        PeState, flat [lev][point] grid storage
  tendencies.rs   explicit dynamics tendencies (column_row)
  implicit.rs     semi-implicit solve
  vertical.rs     σ levels, G, τ matrices
  tracers.rs      tracer advection, fixers
  levels.rs       diagnostics on pressure levels, psl
  cases.rs        initial conditions
system/
  star.rs         Star: T_eff, luminosity, mass; band fractions
  orbit.rs        Kepler orbit, substellar point(s), spin state
  calendar.rs     named months, solar day
  derive.rs       derived instellation, year, rotation; consistency checks
atmosphere.rs     gas mix → R, c_p, molar mass; surface pressure
physics/
  driver.rs       sequential column driver, workspaces, diagnostics registry
  column.rs       ColumnState, ColumnCtx, SurfaceExchange
  radiation/      mod.rs (two-stream solver, Opacity trait), gray.rs, semigray.rs,
                  ktable.rs (P11), cloud_optics.rs
  surface_flux.rs exchange coefficients, 2 m / 10 m diagnostics
  boundary_layer.rs
  convection.rs   dry adjustment, simplified Betts–Miller
  condensation.rs large-scale condensation, precipitation fall, phase
  clouds.rs       diagnostic cloud fraction and water path
  damping.rs      sponge, frictional heating
  idealized/      held_suarez.rs, tj16.rs (frozen regression physics)
surface/
  mod.rs          Surface: tiles, coupler interface, restart
  geography.rs    heightmap → land fraction, orography, roughness
  ocean_slab.rs   mixed layer, q-flux, diffusion
  ocean_wind.rs   Ekman + reduced-gravity layer (P6)
  sea_ice.rs
  land.rs         soil heat layers, bucket
  snow.rs
  albedo.rs       per band, per tile
diagnostics.rs climatology.rs stations.rs runtime.rs io/ config.rs
```

Rules: `primitive/` knows nothing about what the surface or the physics is; it gets total
tendencies from the driver. `surface/` knows nothing about spectral space. No file over
about 600 lines; split by responsibility, not by size alone.

#### 9.3.2 Config schema v2

Top-level run keys (`model`, `truncation`, `dt`, `length`, `output_*`, `[[stations]]`,
`[climatology]`, `[diffusion]`, `[time_filter]`, `[planet]`) keep their meaning.
`[forcing] type = "held_suarez" | "tidally_locked" | "moist_held_suarez"` keeps working
unchanged (idealized bundles). World physics is configured by **typed blocks**, each a
serde struct with `deny_unknown_fields`:

```toml
schema = 2

[star]
t_eff = 5772.0            # K
luminosity = 1.0          # L_sun
mass = 1.0                # M_sun

[orbit]
semi_major_axis = 1.0     # AU → instellation and year are derived
eccentricity = 0.0167
obliquity = 23.44         # degrees
perihelion_longitude = 282.9
start_solar_longitude = 280.0
spin = "free"             # "free" | "synchronous" | "resonant"
# resonance = [3, 2]      # for spin = "resonant": 3 rotations per 2 orbits
diurnal = true
months = ["Jan", "Feb"]   # optional calendar
# explicit overrides (checked against the derived values, see below):
# instellation = 1361.0
# year = "365.25day"

[planet]
radius = 6.37122e6
gravity = 9.80616
rotation_period = "23.9345h"   # sidereal; used only when spin = "free"; negative = retrograde

[atmosphere]
surface_pressure = 1.0e5       # global-mean dry surface pressure [Pa]
composition = { N2 = 0.7808, O2 = 0.2095, Ar = 0.0093, CO2 = 400e-6 }   # volume fractions
# R_d and c_p are computed from the mix; explicit gas_constant / cp override them.

[moisture]
condensable = "water"

[radiation]
scheme = "semigray"            # "semigray" | "gray" | "none"
every = 1                      # in dynamics steps

[boundary_layer]
scheme = "kprofile"

[convection]
scheme = "sbm"                 # "sbm" | "dry" | "none"

[condensation]
reevaporation = false

[clouds]
scheme = "diagnostic"          # "diagnostic" | "none"

[surface]
geography = "worlds/testworld.npy"   # or "aquaplanet"
[surface.ocean]
model = "slab"                 # "fixed_sst" | "slab" | "wind_driven"
mixed_layer_depth = 50.0
[surface.sea_ice]
enabled = true
[surface.land]
bucket_capacity = 150.0

[dynamics.sponge]
enabled = false
```

Derivations in `system/derive.rs` (unit-tested):

- instellation S = 1361 W m⁻² · L / a² (L in L☉, a in AU; the constant is a named
  parameter);
- orbital period P = 365.256 d · √(a³ / M) (a in AU, M in M☉);
- `spin = "synchronous"`: Ω = 2π / P, the substellar longitude is fixed for e = 0 and
  librates for e > 0;
- `spin = "resonant"` with [p, q]: Ω = (p/q) · 2π / P;
- shortwave band fractions from the Planck function at T_eff (§9.4.1).

If an explicit override and the derived value differ by more than 1%, config loading fails
unless `allow_inconsistent = true` is set in that block. Schema-1 world cases
(`gray_aquaplanet`, `testworld*`, `tidally_locked_gray*`) are migrated in P0 with identical
results; `[forcing] type = "gray_aquaplanet"` remains as an alias that expands to blocks.

Named level sets: `levels = "L26"` (the JW06 set, production), `"L20"`, `"L10"` (equal σ,
idealized only). World runs use L26. Default time steps: 15 min at T42, 30 min at T21.

Case files are reorganised in P0: `cases/core/` (Part I tests), `cases/idealized/`
(Held–Suarez family, TJ16), `cases/bench/<paper>/` (published comparators),
`cases/worlds/` (Earth, test worlds, the owner's worlds). `toyclim list-cases` walks the
tree. Update every reference in tests, scripts and tools in the same commit.

#### 9.3.3 Physics driver contract

- Physics is evaluated on the n−1 leapfrog level over Δt_p = 2Δt (Δt on the first step),
  as now.
- **Sequential (time-split) update.** The driver copies the column into a provisional
  `ColumnState` (u, v, T, tracers, p_s). Each scheme reads the provisional state, as
  modified by the schemes before it, and updates it in place over Δt_p. The tendency handed
  to the dynamics is (final − initial) / Δt_p. Order:
  1. radiation (heating rates; surface fluxes handed to step 2);
  2. surface fluxes + boundary layer + surface energy balance (one coupled implicit solve,
     §9.4.3);
  3. dry convective adjustment;
  4. moist convection;
  5. large-scale condensation and precipitation fall (melting, optional re-evaporation);
  6. cloud diagnosis (stored for the next radiation call);
  7. sponge and frictional-heating bookkeeping.
- The idealized bundles (`held_suarez`, `tidally_locked`, `tj16`) run through the same
  driver. Check in TJ16 / Reed & Jablonowski (2012) whether their simple physics is
  specified as sequential; if it is, switch the bundle to the paper's order and re-record
  the M7 numbers.
- Scheme interface (shape, not final names):

  ```rust
  pub trait ColumnScheme: Send + Sync {
      fn name(&self) -> &'static str;
      fn diagnostics(&self) -> &'static [&'static str];
      fn apply(&self, ctx: &ColumnCtx, state: &mut ColumnState,
               sfc: &mut SurfaceExchange, diag: &mut [Real], work: &mut Workspace);
  }
  ```

  `ColumnCtx` is the read-only context (lat, lon, σ, Φ_s, time, Δt_p, planet, atmosphere,
  beams). `SurfaceExchange` carries what atmosphere and surface trade for this column
  (per-band downward shortwave, downward longwave, rain, snow, and back: fluxes, stress).
- **No heap allocation per column.** Each rayon worker owns a `Workspace`; diagnostic
  offsets are computed once at model build.
- **Diagnostics registry.** Schemes declare their diagnostic names once; the driver owns
  the storage. Consumers (bucket, climatology, stations) look fields up by a typed id, not
  by string prefix.
- **Conservation is a property of the driver, tested there:** for every scheme, column
  energy and water changes must equal the boundary fluxes it reports (§9.5.1).
- **Carried physics state** (anything a scheme remembers between steps, e.g. cloud fields,
  friction velocity, stored radiative fluxes) is part of the restart. The bitwise restart
  and thread-count tests are extended whenever such state is added.

#### 9.3.4 Sign and naming conventions

- Radiative fluxes are positive downward at the surface and in "net" quantities; `olr` and
  reflected shortwave are positive upward. Turbulent fluxes (`shf`, `lhf`, `evap`) are
  positive from surface to atmosphere. Precipitation and evaporation are in
  kg m⁻² s⁻¹.
- Output field names. Existing names stay. New fields use these:

| Name | Meaning |
|---|---|
| `tas`, `huss`, `uas`, `vas`, `sfcwind` | 2 m temperature and humidity, 10 m wind (true 2 m / 10 m from P1; the prototype's lowest-level versions are renamed `t_low`, `u_low`, `v_low`, `wind_low`) |
| `gust` | 10 m gust estimate |
| `ts` | surface (skin) temperature, tile-area mean |
| `psl` | sea-level pressure |
| `precip`, `precip_ls`, `precip_conv`, `prsn` | total, large-scale, convective precipitation; snowfall part |
| `evap`, `shf`, `lhf`, `tauu`, `tauv` | surface fluxes and stress |
| `rsdt`, `rsut`, `olr`, `asr`, `rsds`, `rlds`, `rsus`, `rlus` | radiative fluxes at top and surface |
| `clt`, `cl` | total cloud cover, cloud fraction per level |
| `snw`, `snc` | snow water equivalent, snow cover fraction |
| `sic`, `sit` | sea-ice concentration and thickness |
| `mrso`, `mrro` | soil water (bucket), runoff |
| `tos`, `uo`, `vo`, `zos` | ocean mixed-layer temperature, currents, upper-layer thickness anomaly |
| `land`, `orog` | land fraction, model orography |

### 9.4 Physics specifications

Each subsection gives the required behaviour, a default formulation, and the knobs. Knobs
are marked **[knob]** and go to `docs/tuning.md`.

#### 9.4.1 Radiation (P3)

**Structure.** One generic multi-band two-stream solver and an `Opacity` provider trait:

```rust
pub trait Opacity {
    fn sw_bands(&self) -> usize;  fn lw_bands(&self) -> usize;
    /// Layer optical properties per band for one column (gas only).
    fn layer_optics(&self, col: &ColumnState, ctx: &ColumnCtx, out: &mut LayerOptics);
    /// Fraction of the stellar flux in each shortwave band; Planck fraction per longwave band.
    fn sw_weights(&self, star: &Star, out: &mut [f64]);
    fn lw_planck_fraction(&self, t: f64, out: &mut [f64]);
}
```

Providers: `Gray` (existing Frierson scheme; optical depth as a function of σ, with the
latitude dependence optional and off for non-Earth cases), `SemiGray` (below), `KTable`
(P11: bands × g-points from precomputed tables fit the same interface).

**Shortwave solver.** Per band and per beam (a beam is one star: flux at the top of the
atmosphere and cos zenith; the interface takes a list, one star is tested): δ-Eddington
layer reflectances and transmittances for direct and diffuse light, combined with the
adding method; surface albedo per band from `surface/albedo.rs`. Output: heating rates,
per-band direct + diffuse flux at the surface, reflected flux at the top.

**Longwave solver.** Absorption and emission without scattering, diffusivity factor 1.66,
exact integration per layer (as in the existing gray code), surface emissivity 1. Each band
carries the Planck fraction of σT⁴ for its interval.

**Semi-gray opacities (default for worlds).**

- Shortwave bands: `vis` (λ < 0.75 µm), `nir_weak`, `nir_strong` (λ > 0.75 µm split into
  two pseudo-bands so water absorption saturates realistically; the split weights are
  **[knob]**, fitted offline).
  - Band fractions: fraction of a blackbody at T_eff below and above 0.75 µm, by numerical
    integration of the Planck function. Tests: about 0.54 below 0.75 µm at 5772 K and about
    0.11 at 3000 K (check by an independent quadrature).
  - Rayleigh scattering: τ_R = τ_R0,band · (p_s / 1e5 Pa) · (9.81 / g), with τ_R0 for an
    Earth-air column **[knob]** (transcribe a band-mean value; near-IR is much smaller than
    visible).
  - Water vapour (near-IR only): dτ = k_band · q · (p / p_ref)^n dp / g **[knobs]**.
  - Optional ozone-like visible absorber as a prescribed column (off by default).
- Longwave bands: `window` (8–14 µm) and `rest`, after the intermediate scheme of
  Geen et al. (2016) as implemented in Isca (transcribe the published form and constants):
  - rest: dτ/dσ = a + b·q + c·ln(CO₂ / CO₂_ref);
  - window: dτ/dσ = a_w + b_w·q + c_w·q²;
  - window Planck fraction as a function of temperature.
  - Extension for the envelope: the dry terms scale with column mass and pressure
    broadening, a → a · (p_s / 1e5) · (9.81 / g) · (p / p_ref)^n **[knob n]**.
- Calibration is offline (`tools/calibrate_radiation.py`): fit the knobs to reference
  clear-sky fluxes for a set of profiles. Preferred reference: RRTMG through `climlab` if it
  installs on the laptop; otherwise published benchmark values. The fit, its inputs and its
  residuals are committed under `docs/radiation/`.

**Clouds in the solver.** Two-column approximation: total cloud cover C from
maximum-random overlap; one clear and one cloudy solve per band, fluxes combined as
(1 − C)·F_clear + C·F_cloudy. In the cloudy column each layer has the in-cloud optical
depth scaled by c_k / C.

**Calling interval.** `every = 1` (each physics step) is the default. With `every > 1`, the
stored heating rates and surface fluxes are reused, shortwave is rescaled by the current
top-of-atmosphere flux, and both atmosphere and surface use the same stored numbers so the
budget still closes. The surface solve's linearised emission change 4σT_s³ΔT_s goes to
space and is added to `olr`.

**Unit and offline acceptance (independent of the chosen formulas):**

1. Energy: top-of-atmosphere net = atmospheric heating + surface absorption to 1e-6 W m⁻²
   for random columns, with and without clouds.
2. A non-absorbing, non-scattering atmosphere returns the surface albedo as planetary
   albedo; a conservative-scattering layer over a black surface conserves flux.
3. Isothermal column: OLR = σT⁴.
4. Clear-sky OLR for a moist-adiabatic profile with 75% relative humidity and 400 ppm CO₂:
   260–275 W m⁻² at T_s = 288 K; slope dOLR/dT_s = 1.7–2.5 W m⁻² K⁻¹ between 265 and
   300 K (compare Koll & Cronin 2018).
5. CO₂ doubling 400 → 800 ppm on the same 288 K profile reduces OLR by 2.5–5 W m⁻².
6. Clear-sky shortwave, Sun, global-mean conditions (cos zenith 0.5, 25 kg m⁻² water,
   ocean albedo): atmospheric absorption 17–24% of the incoming flux, planetary albedo
   0.12–0.19.
7. Same column under a 3000 K star: atmospheric absorption fraction higher and planetary
   albedo lower than under the Sun.

#### 9.4.2 Surface exchange (P1)

- Bulk formulas at the lowest level (height z_L from the hydrostatic relation):
  τ = ρ C_D |V| v, H = ρ c_p C_H |V| (θ_s − θ_L), E = β ρ C_E |V| (q_sat(T_s) − q_L).
- |V|² = u² + v² + w_g², gustiness w_g = 1 m s⁻¹ **[knob]**.
- Neutral coefficient C_n = κ² / [ln(z_L/z_0m) ln(z_L/z_0h)], κ = 0.4.
- Stability dependence on the bulk Richardson number, Louis (1979) functions (transcribe):
  enhanced exchange for Ri < 0, long-tailed reduction for Ri > 0 (never exactly zero, so
  the night-time surface does not decouple completely).
- Roughness: ocean Charnock, z_0m = max(1.5e-5 m, 0.018 u*² / g) using the stored u* of the
  previous step; sea ice and snow 1e-3 m; land 0.1 m default or from an optional map;
  z_0h = z_0m / 10 over land and ice.
- 2 m temperature and humidity and 10 m wind by the same similarity functions between the
  surface and the lowest level. `gust` = 10 m wind + 3·u* in unstable conditions,
  + 1·u* otherwise **[knobs]** (a placeholder until P8 calibrates it).
- The TJ16 constant coefficient 0.0044 stays inside the `tj16` bundle only.

#### 9.4.3 Boundary layer and the coupled surface solve (P1)

- K-profile scheme after Troen & Mahrt (1986) as used by Frierson et al. (2006): boundary
  layer height h from a bulk Richardson criterion; K(z) ∝ κ u* z (1 − z/h)² with the
  surface-layer stability functions; the same K for heat, moisture and momentum. Above h, a
  small background diffusivity **[knob]**.
- Diffused quantities: dry static energy (c_p T + g z), q, other tracers, u, v.
- One implicit (backward Euler) tridiagonal solve per quantity per column.
- **Coupling to the surface** (conservative and unconditionally stable):
  1. Downward elimination of the tridiagonal system gives, for the lowest level, linear
     relations s_L⁺ = A_s + B_s·H and q_L⁺ = A_q + B_q·E in the yet unknown surface fluxes.
  2. Each surface tile solves its own energy balance implicitly for T_s⁺:
     C_s (T_s⁺ − T_s)/Δt_p = SW_net + LW_down − σT_s⁺⁴ − H − L·E − G, with H and E written
     with s_L⁺, q_L⁺ from step 1, and σT⁴ and q_sat linearised about T_s. G is the
     conductive flux into the soil, ice or ocean below (implicit too).
  3. The driver area-averages the tile fluxes (H, E, τ, upward longwave) and
     back-substitutes them into the atmospheric solve.
  The flux removed from the tiles equals the flux given to the atmosphere to round-off.
  (With several tiles, step 2 treats each tile as if it covered the cell; this is the
  standard approximation and stays conservative because step 3 uses the mean.)
- Momentum: implicit surface stress, no Rayleigh friction in world runs.
- **Frictional heating:** kinetic energy removed by vertical diffusion, surface drag and
  the sponge is returned as heat in the same layer. Kinetic energy removed by
  hyperdiffusion is diagnosed globally each step and returned as a uniform heating
  (energy fixer, off for Part I cases).
- **Sponge** (`[dynamics.sponge]`, off by default): linear damping of u, v toward zero in
  the top levels (σ < 0.02, time scale 1 day at the top level) **[knobs]**. Turn on for a
  case only when the run shows wave reflection or instability at the top, and record it.
- **Dry convective adjustment:** energy-conserving adjustment to the dry adiabat wherever
  the provisional profile is unstable; the only convection when moisture is off.

#### 9.4.4 Convection and condensation (P1–P2)

- Moist convection: simplified Betts–Miller (Frierson 2007), τ = 2 h, RH_ref = 0.7
  **[knobs]**, with the paper's shallow-convection treatment chosen and recorded (the
  prototype has none).
- Large-scale condensation: as now (TJ16 eq. 2 form), acting on the state left by
  convection.
- **Phase.** `Condensable` gains `latent_heat_fusion` and `t_freeze`. Condensate formed in a
  layer is ice with fraction f_ice(T): 0 at T ≥ t_freeze, 1 at T ≤ t_freeze − 20 K, linear
  between **[knob]**; latent heating uses L_v + f_ice·L_f. One saturation curve (over
  liquid) is used at all temperatures; document this.
- **Fall.** Rain and snow fluxes are carried downward layer by layer. Snow melts in layers
  warmer than t_freeze, cooling them by L_f, limited so the layer does not cool below
  t_freeze. Rain does not refreeze. Optional re-evaporation of rain into sub-saturated
  layers (`reevaporation`, off by default; the Earth gate decides) **[knob]**.
- The surface receives rain and snow separately (`precip`, `prsn`).
- Abort with a clear message if q > 0.1 anywhere for more than 100 consecutive steps
  (outside the dilute envelope).

#### 9.4.5 Land, snow and sea ice (P2)

**Tiles.** Each cell has fractions of open ocean, sea ice and land (land fraction from the
heightmap, no 50% threshold). Each tile has its own T_s, albedo, roughness and wetness.

**Land.**

- Soil heat: 5 layers, implicit diffusion, zero flux at the bottom. Top layer thickness
  d₁ = 0.25·√(2κ_s / ω_day) clamped to [0.01, 0.2] m, where ω_day is the solar-day
  frequency (use 0.03 m when there is no diurnal cycle); each layer three times thicker
  than the one above. Volumetric heat capacity 2.0e6 J m⁻³ K⁻¹, conductivity
  1.0 W m⁻¹ K⁻¹ **[knobs]**.
- Water: bucket (Manabe 1969), capacity 150 kg m⁻² **[knob]**, β = min(1, W / (0.75 W_max)),
  runoff when full (`mrro`, leaves the system; the ocean is an infinite reservoir).
- Albedo and roughness from optional maps, else defaults (§9.4.6).
- No prognostic vegetation.

**Snow** (on land and on sea ice).

- Snow water equivalent S accumulates snowfall; melts with latent heat L_f when the tile's
  energy balance would take T_s above t_freeze (T_s is held at t_freeze while snow
  remains); sublimates through E with L_v + L_f.
- Cover fraction f_snow = S / (S + S_c), S_c = 15 kg m⁻² **[knob]**.
- S is capped at 5000 kg m⁻²; the excess is added to runoff. Cells at the cap in every
  month of the climatology are reported as `glacier` (a diagnostic mask for ice sheets).

**Sea ice.** Zero-layer thermodynamic model (Semtner 1976):

- forms when the mixed layer would cool below the freezing point (271.35 K for sea water,
  configurable); the energy deficit grows ice, L_f ρ_i dh/dt;
- surface temperature from the tile energy balance with conductive flux
  k_i (T_freeze − T_s) / h_eff through ice and snow; surface melt when T_s reaches 273.15 K;
  bottom melt from mixed-layer heat;
- concentration from mean thickness, sic = min(1, h / h₀), h₀ = 0.5 m **[knob]**;
- thickness capped at 50 m (the excess energy is taken from the mixed layer and reported in
  the diagnostic `ice_cap_flux`; on permanent night sides the cap is physical bookkeeping,
  not a tuning knob);
- no ice drift.

#### 9.4.6 Albedo (P2, per band from P3)

Per tile and per shortwave band (visible / near-IR). Defaults to transcribe from a cited
source (e.g. the values used by Shields et al. 2013 or a CESM/CCSM description); starting
values **[knobs]**: ocean 0.06 / 0.06; bare land 0.15 / 0.30; sea ice 0.70 / 0.45; snow
0.90 / 0.60 at T_s ≤ 263 K, falling linearly to 0.75 / 0.45 at 273 K. Snow-covered tiles
blend with f_snow. Until P3, a single broadband value is the Sun-weighted mean of the two.
There is **no temperature switch** for ice any more; albedo follows the ice and snow state.

#### 9.4.7 Clouds (P4)

- Stratiform fraction per level from relative humidity (Sundqvist-type):
  c = 1 − √((1 − RH) / (1 − RH_c)) for RH > RH_c, with RH_c(σ) between 0.75 and 0.95
  **[knobs]**.
- Convective cloud from the convective precipitation rate, between the cloud base and top
  of the convection scheme **[knob]**.
- In-cloud water path per layer diagnosed (no prognostic cloud water): proportional to
  q_sat(T) of the layer with a cap **[knobs]**; ice fraction as f_ice(T).
- Optics: shortwave optical depth 3·LWP / (2 ρ_w r_e), r_e = 10 µm liquid, 30 µm ice,
  single-scattering albedo and asymmetry per band (transcribe, e.g. Slingo 1989);
  longwave absorption optical depth k_lw·LWP, k_lw ≈ 0.14 m² g⁻¹ **[knobs]**.
- Tuned on Earth only (P-4).

#### 9.4.8 Ocean (P6)

Three options, chosen by `[surface.ocean] model`:

1. `fixed_sst`: prescribed field or analytic profile (idealized cases, Earth AMIP gate).
2. `slab`: mixed layer of fixed depth with optional horizontal diffusion and optional
   prescribed q-flux.
3. `wind_driven`: slab plus dynamically computed transport. Two parts.

**Ekman transport** after Codron (2012) (transcribe): surface Ekman mass transport from the
wind stress with a frictional regularisation at the equator,
M_E = (ε τ − f k×τ) / (ε² + f²) form; return flow in a layer below at temperature T_d;
upwelling from the divergence. Gives equatorial upwelling and poleward Ekman heat
transport.

**Reduced-gravity upper layer** (gyres, boundary currents, circumpolar flow). A linear
1.5-layer shallow-water model on the ocean cells of the Gaussian grid (Arakawa C grid,
no-normal-flow at coasts), forced by the stress, damped by linear friction:

- ∂u/∂t − f v = −g′ ∂η/∂x + τ_x / (ρ H) − r u
- ∂v/∂t + f u = −g′ ∂η/∂y + τ_y / (ρ H) − r v
- ∂η/∂t + H ∇·(u, v) = 0

Defaults: H = 400 m, g′ = 0.03 m s⁻², r chosen so the Stommel boundary-layer width r/β at
30° latitude is 1.5 grid cells **[knobs]**. Semi-implicit Coriolis and gravity terms; its
own time step, sub-cycled or super-cycled against the atmosphere deterministically. It
needs no island rule, works at any rotation rate (including synchronous) and gives
circumpolar currents where the geography allows them.

**Heat.** The mixed-layer temperature (`tos`, depth 50 m) is advected by the Ekman flow
plus the layer flow (upwind, conservative), diffused horizontally, and exchanges heat with
a thermocline temperature T_d (the rest of H) through upwelling and a slow vertical
mixing; T_d is advected by the layer flow and the Ekman return flow. Below H the ocean is
inert. Total ocean heat content changes only by the surface flux (tested).

Outputs: `uo`, `vo` (mixed-layer currents), `zos`, a barotropic-like transport
streamfunction diagnostic, `tos`.

Not represented: deep overturning (no AMOC), eddies, salinity, tides.

#### 9.4.9 Sub-grid statistics and downscaling (P8)

- Lapse-rate downscaling uses the **model's own** climatological lower-troposphere lapse
  rate per cell and month, not a fixed 6.5 K/km.
- Orographic precipitation: Smith & Barstad (2004) with the model's winds, moist stability
  and time scales; afterwards the fine field is rescaled so its mean over each model cell
  equals the model's value (the correction redistributes, it does not create water).
- Point statistics: a stochastic generator mapping cell-mean hourly rain to point rain,
  with parameters that are functions of **large-scale predictors only** (convective
  fraction, CAPE, precipitable water, cell-mean rate, wind), never of location, so the same
  mapping applies on any planet. Calibrated on the Earth run against station data.
- Biomes: Köppen–Geiger (Peel et al. 2007 rules, generalised to any calendar by dividing
  the year into 12 equal parts of the orbit; planets without seasons classify from annual
  values and say so in the legend), Holdridge life zones, and a simple
  temperature–precipitation (Whittaker) biome.
- Rivers: route the runoff climatology downhill on the fine heightmap (D8 flow directions,
  depression filling), giving discharge maps and lakes in closed basins.

### 9.5 Validation infrastructure

#### 9.5.1 Single-column model (built in P1)

`toyclim scm --config <case>` runs the physics driver and the surface on one column with
prescribed large-scale conditions (no dynamics). Uses:

- conservation tests per scheme and for the whole chain: energy and water change of
  column + surface equals the boundary fluxes to 1e-6 W m⁻² and 1e-9 kg m⁻² s⁻¹;
- stability tests (tiny heat capacity, strong wind, large Δt);
- radiative–convective equilibrium cases for radiation and clouds.

Every new scheme arrives with SCM tests before it is used in 3-D.

#### 9.5.2 Scorecards

`tools/scorecard.py <run> --suite <name>` computes the metrics of a gate, prints a
pass/fail table and writes `docs/scorecards/<milestone>_<suite>_<date>.json`. Suites:
`aquaplanet`, `earth_amip`, `earth_coupled`, `tidally_locked`, `sweep` (trend checks).
Metrics every world suite includes:

- global means: T_s, precipitation, evaporation, precipitable water, planetary albedo, OLR,
  net top-of-atmosphere and surface fluxes;
- **energy closure:** |d(total energy)/dt − (net TOA − net surface)| in the atmosphere, and
  the same for the whole system; **water closure**;
- **grid-point storm index G:** the largest ratio, over all cells with flat surroundings
  (orography within ±300 m of its neighbours), of a cell's annual-mean precipitation to
  the mean of its 8 neighbours. Pass: G ≤ 2.0;
- tracer fixer rate (§9.8);
- equilibrium: a run is **equilibrated** when the 5-year trend of global-mean T_s is below
  0.02 K/yr and the 5-year mean net top-of-atmosphere flux is within ±1 W m⁻² of the
  energy-closure residual. Climatologies for gates come from equilibrated runs only
  (fixed-SST runs: after 1 year).

#### 9.5.3 Reference data

- `tools/fetch_reference.py` downloads public data with fixed URLs into `reference/`
  (git-ignored), and writes regridded T42/T21 arrays plus a `SOURCES.md` with URL, date,
  citation and licence to `reference/derived/` (committed if under 5 MB in total and the
  licence allows). The script sends nothing but the request itself.
- Earth: topography (ETOPO), observed Köppen–Geiger map (Beck et al. 2018), long-term
  monthly means of precipitation (GPCP), 2 m temperature, sea-level pressure, zonal wind
  (NCEP/NCAR reanalysis or ERA5 climatology), sea-surface temperature and sea ice
  (NOAA OISST or HadISST), snow cover. Use whichever sources are reachable without an
  account and record them.
- Owner-supplied model output (ExoPlaSim, Isca, SpeedyWeather) goes to `reference/<model>/`;
  when present, the matching gate also reports a comparison against it.

#### 9.5.4 Regression suite

`scripts/check` (fast: fmt, clippy, tests) and `scripts/regress` (slow, run at the end of
every milestone, in the background):

- all `--ignored` tests of Part I;
- Held–Suarez T42 L20 1200 d → M5 criteria;
- T21 versions of every Phase 2 gate passed so far → their scorecards;
- `scripts/regress --against <git-ref>`: builds the reference commit into a separate
  target directory, runs a fixed list of short cases with both builds on this machine and
  compares outputs byte for byte. Used for refactors that must not change results (P0).

### 9.6 Milestones

Every milestone ends as in §8 (tests, clippy, fmt, figures, progress entry) plus:
scorecards written, `docs/tuning.md` updated, regression suite passed (P-7), every figure
looked at (P-6). Numbers are acceptance criteria unless marked *record*.

Standard runs: "T21 smoke" = 3 years; "T42 gate" = equilibrated + 10-year mean for
interactive oceans, 1 + 5 years for fixed SST.

#### P0 — Groundwork (no change of results)

1. Close what the audit left open in Part I:
   - f32 Held–Suarez asymmetry (open question 3): rerun 2200 days with two perturbation
     seeds in f32 and in f64, report all four; the owner decides.
   - Fix `tools/tidally_locked.py` / `tidally_locked_gray` (missing maps).
   - Laptop checklist item 2 (constants of Williamson 6 and Galewsky against the papers).
2. Create `scripts/regress` (§9.5.4) and `docs/tuning.md` (all current parameters of the
   prototype physics, with their sources).
3. Restructure to §9.3.1: split `primitive/mod.rs`; move the surface into `surface/`; move
   orbit and calendar under `system/`; flat `[lev][point]` storage; workspaces instead of
   per-column and per-step allocations (including `diagnostic_names()` in the column loop
   and the clone of the previous state); diagnostics registry.
4. Config schema v2 (§9.3.2) with the star/orbit derivations and `[atmosphere]`
   composition; migrate world cases; reorganise `cases/`.
5. Performance: profile Held–Suarez T42 L20 and the test world; *record* ms/step before
   and after. Target (not a criterion): at least 1.5× faster on 8 threads.

**Accept:**

- `scripts/regress --against <pre-P0 commit>`: byte-identical output for Held–Suarez T21
  (30 d), TJ16 T21 with and without SBM (30 d), gray aquaplanet T21 (60 d), test world T21
  (60 d), tidally locked gray T21 (60 d), in f64, with 1 and with 8 threads.
- Derivation tests: Earth values give S = 1361 W m⁻² and a year of 365.256 d to 1e-4
  relative; a TRAPPIST-1e-like input (L = 5.5e-4 L☉, M = 0.09 M☉, a = 0.0293 AU;
  transcribe current values) gives a 6.1-day year and about 900 W m⁻² within 5%; an
  inconsistent override is rejected.
- No file in `toyclim-core/src` over 700 lines.

#### P1 — Surface exchange, boundary layer, sequential physics, single-column model

Implement §9.3.3 (sequential driver), §9.4.2, §9.4.3, §9.5.1, the scorecard tool with the
`aquaplanet` suite, and `psl`. World configurations stop using Rayleigh friction.

**Accept:**

1. SCM conservation for every scheme and the full chain (§9.5.1 tolerances).
2. SCM stability: land tile with heat capacity 1e3 J m⁻² K⁻¹, 30 m s⁻¹ wind, Δt_p = 1 h:
   T_s shows no 2Δt oscillation (successive differences do not alternate in sign with
   growing amplitude) over 10 days.
3. SCM diurnal cycle over dry land (prescribed clear-sky insolation, equinox, 20° latitude):
   2 m diurnal range 8–25 K; nocturnal boundary layer stable; over a 50 m ocean mixed
   layer the range is below 1 K.
4. Neutral 10 m drag coefficient over ocean at 10 m s⁻¹: 1.0e-3 to 1.6e-3.
5. Aquaplanet with fixed SST (use the "Control" profile of the Aqua-Planet Experiment,
   Neale & Hoskins 2000; transcribe), gray radiation, T42 L26, 1 + 3 years:
   - lowest-level zonal-mean wind: easterlies in the tropics, westerlies at 35–60°, peak
     surface westerlies 5–12 m s⁻¹;
   - global-mean precipitation = evaporation within 1%;
   - energy closure of the atmosphere within 0.5 W m⁻²;
   - G ≤ 2.0;
   - *record* the zonal-mean precipitation, the Hadley cell strength and edge, jets.
6. Held–Suarez and TJ16 regression unchanged (their bundles do not use the new schemes;
   if the TJ16 order was changed per §9.3.3, re-record M7 and say so).
7. Viewer still runs world cases (headless check).

#### P2 — Land, snow, sea ice, precipitation phase

Implement §9.4.4 (phase, fall), §9.4.5, §9.4.6 (broadband for now). Diagnose prototype
defect 1 (§9.1.1) with the listed hypotheses and record which one it was.

**Accept:**

1. SCM: snowpack and sea-ice energy and water conservation including latent heat of
   fusion (§9.5.1 tolerances); a snow-covered tile cannot exceed the freezing point while
   snow remains.
2. Global water budget of a 3-year T21 land world: d/dt (vapour + soil water + snow) =
   E − P − runoff to within 1e-3 mm/day.
3. System energy closure within 0.5 W m⁻² (T21 world, 3 years).
4. Bitwise restart and thread independence with all new surface state.
5. Test world T42, gray radiation, slab 50 m, equilibrated: G ≤ 2.0; no EF classification
   over ice-free ocean; sea ice only where the annual-mean `tos` is at the freezing point.
6. Bistability, T21 aquaplanet with slab and seasons: started ice-free it stays ice-free
   equatorward of 50° at 1361 W m⁻²; started fully ice-covered it stays ice-covered at
   1361 W m⁻². *Record* the instellation at which the warm branch freezes over (steps of 2%).
7. *Record* an Earth scorecard (`earth_amip` suite as far as it can be computed) for later
   comparison; no thresholds yet.

#### P3 — Radiation

Implement §9.4.1 (solver, `Gray` and `SemiGray` providers, calibration tool) and per-band
albedo. The gray scheme's latitude-dependent optical depth becomes an option that is on
only in Earth-like idealized cases.

**Accept:**

1. Offline tests 1–7 of §9.4.1.
2. Calibration report committed (`docs/radiation/`), with residuals: clear-sky OLR and
   surface downward longwave within 8 W m⁻², shortwave atmospheric absorption within
   10% (relative), over the fitted profile set (T_s 250–310 K, CO₂ 100–10000 ppm,
   p_s 0.3–3 bar where the reference allows).
3. SCM radiative–convective equilibrium, Sun, global-mean insolation, planetary albedo
   forced to 0.3 by a reflective layer, 400 ppm: T_s = 283–293 K; doubling CO₂ warms the
   equilibrium by 1.5–4.5 K.
4. T21 aquaplanet, slab, 400 ppm, surface albedo raised to stand in for clouds: stable for
   10 years, energy closure within 0.5 W m⁻², G ≤ 2.0.
5. Budget: `every = 1` radiation costs less than 40% of a step at T42 L26 (*record*).

#### P4 — Clouds and Earth-like energy balance

Implement §9.4.7. Tune the cloud knobs (and only those listed in `docs/tuning.md` as
tunable) on an Earth-like aquaplanet and on the Earth AMIP configuration of P5.

**Accept** (Earth AMIP configuration, T42 L26, 1 + 5 years):

1. Planetary albedo 0.27–0.33; OLR 228–250 W m⁻²; net TOA flux within ±8 W m⁻²
   (fixed SST, so not constrained to zero).
2. Cloud radiative effect: shortwave −30 to −60 W m⁻², longwave +15 to +35 W m⁻².
3. Total cloud cover 0.5–0.75; maxima in the ITCZ and the storm tracks, minima in the
   subtropics (zonal mean).
4. Slab aquaplanet with clouds, equilibrated, 400 ppm: global-mean T_s 283–293 K;
   2×CO₂ equilibrium warming 1.5–6 K (*record* the value).
5. Energy closure within 0.5 W m⁻².

#### P5 — Earth gate (atmosphere and land, prescribed ocean)

Earth topography and land fraction, observed monthly SST and sea-ice climatology
(`fixed_sst`), Earth orbit, 400 ppm, T42 L26, 1 + 5 years. `earth_amip` scorecard against
the reference data of §9.5.3. Thresholds were set on 2026-11-12 from what
intermediate-complexity models typically achieve; they are acceptance criteria.

**Accept:**

| Metric | Threshold |
|---|---|
| Global-mean precipitation | 2.4–3.4 mm/day |
| Zonal-mean annual precipitation vs GPCP | correlation ≥ 0.85; tropical peak 5–9 mm/day between 10°S and 12°N; minima < 2.5 mm/day in both subtropics |
| Annual precipitation map vs GPCP (T42) | pattern correlation ≥ 0.60 |
| Annual-mean 2 m temperature over land | area-weighted RMSE ≤ 4.5 K (after lapse-rate correction to the reference orography) |
| Seasonal cycle of land 2 m temperature (July − January) | pattern correlation ≥ 0.90; amplitude ratio 0.75–1.3 |
| Sea-level pressure, DJF and JJA, 60°S–60°N | pattern correlation ≥ 0.60 each |
| Jets at 200 hPa | winter jets 30–55 m s⁻¹ at 25–45° in each hemisphere |
| Monsoon | JJA − DJF precipitation over 10–25°N, 70–90°E ≥ 3 mm/day |
| Köppen main group (A, B, C, D, E) vs Beck et al. | ≥ 60% of land area agrees |
| Named regions, majority class | B: Sahara (18–30°N, 10°W–30°E), Arabia (18–28°N, 42–55°E), central Australia (20–30°S, 120–140°E); A: Amazon (10°S–0°, 70–55°W), Congo (5°S–5°N, 15–28°E), Borneo/Sumatra (5°S–5°N, 100–118°E); D: central Siberia (55–65°N, 90–120°E); E: Greenland interior, Antarctica |
| Diurnal 2 m range | Sahara box 10–22 K; open tropical ocean < 1.5 K |
| NH snow cover extent | February 35–58 million km²; August < 10 million km² |
| G (grid-point storm index) | ≤ 2.0 |
| Energy and water closure | 0.5 W m⁻², 1e-3 mm/day |
| Cost | ≤ 30 min per model year at T42 L26 on 8 threads (*record* T21 too; target 20 min) |

*Record:* the full 30-class Köppen agreement, the maps, the zonal-mean cross sections, the
station distributions for ten cities in different climates.

If a threshold fails after the knobs in `docs/tuning.md` have been explored within their
ranges, do not widen the threshold: record the evidence under "Open questions", check the
conditional items of §9.8, and continue with P6 only if at most two rows fail.

#### P6 — Ocean

Implement §9.4.8 in two steps.

**P6a — Ekman transport and diffusion.** Reproduce the aquaplanet experiments of
Codron (2012) (transcribe set-up and numbers).

**Accept:** poleward ocean heat transport peak within the spread rule (single different
model: ±30%) and at the right latitude (±6°); equatorial SST minimum from upwelling
present; heat conservation of the ocean to 1e-3 W m⁻².

**P6b — Reduced-gravity layer and coupled Earth.**

**Accept:**

1. Analytic tests: Stommel gyre in a rectangular basin on a β-plane-like sector
   (streamfunction within 10% of the analytic solution away from the boundary layer);
   mass conservation to round-off; no flow through coasts.
2. Earth, wind-driven ocean, equilibrated, `earth_coupled` scorecard:
   - subtropical gyres anticyclonic and subpolar gyres cyclonic in every basin; western
     boundary currents on the western sides; eastward circumpolar flow in the Southern
     Ocean;
   - North Atlantic and North Pacific subtropical gyre transport 15–70 Sv;
   - at 20–30° latitude, eastern-boundary `tos` colder than western-boundary `tos` in the
     same basin by at least 2 K (annual mean) in at least 4 of the 5 subtropical basins;
   - equatorial Pacific cold tongue: east colder than west by at least 2 K;
   - annual-mean `tos` RMSE vs observations, 60°S–60°N ≤ 3.5 K; sea-ice extent within a
     factor 1.6 of observed in each hemisphere (annual mean);
   - Köppen main-group agreement ≥ 50% of land area; G ≤ 2.0; closures as before;
   - global-mean T_s 284–292 K, drift below the equilibrium criterion.
3. *Record:* the same run with `slab` only, to show what the ocean transport changes.

#### P7 — Exoplanet gates

No tuning (P-4). For each benchmark: fetch the paper(s), transcribe the configuration to
`cases/bench/<paper>/`, run, compare by the spread rule (P-2), write a scorecard and a
figure that mirrors the paper's key figure. Minimum set:

1. **Tidally locked gray aquaplanet**, Merlis & Schneider (2010), slow and Earth-like
   rotation, with `scheme = "gray"` and SBM (same physics → ±15% / ±3 K): day-side and
   night-side surface temperature extremes, substellar precipitation maximum, equatorial
   superrotation in the fast case, weak free-troposphere temperature gradients in the slow
   case.
2. **Parameter sweeps of an Earth-like aquaplanet**, Kaspi & Showman (2015): rotation
   (1/8 to 4 Ω⊕, T85 for the fastest if affordable), surface pressure, gravity,
   instellation. Trends: number of jets and equator-to-pole temperature difference
   increase with rotation rate; Hadley cell narrows; equator-to-pole difference decreases
   with atmospheric mass. Values at Earth parameters by the spread rule.
3. **THAI protocol for TRAPPIST-1e** (Fauchez et al. 2020; Turbet et al. 2022;
   Sergeev et al. 2022): cases Ben1 (dry, N₂ with 400 ppm CO₂) and Hab1 (moist), with the
   full physics and a 2600 K star. Global-mean, day-side and night-side surface
   temperatures, cloud cover and planetary albedo within the multi-model range widened per
   P-2. (Ben2/Hab2 are CO₂-dominated: outside the envelope, skip.)
4. **Circulation regimes of synchronous rotators**, Haqq-Misra et al. (2018) or
   Noda et al. (2017): three rotation periods spanning their slow, intermediate and fast
   regimes; the circulation pattern (day–night overturning vs. superrotation and
   stationary Rossby gyres vs. mid-latitude jets) matches the regime the paper assigns.
5. **Stellar type and ice**, Shields et al. (2013): the instellation at which an aquaplanet
   freezes over is lower for an M-dwarf spectrum than for a G star (sign required;
   magnitude by the spread rule).
6. **Substellar clouds**, Yang, Cowan & Abbot (2013): for a slowly rotating synchronous
   planet the day-side cloud cover and the planetary albedo increase with instellation.
7. **Obliquity**: aquaplanets at 0°, 23.5°, 54°, 90° (e.g. Ferreira et al. 2014;
   Linsenmeier et al. 2015): at 90° the annual-mean equator is colder than the poles and
   the seasonal cycle is extreme; meridional temperature gradient changes sign near 54°.
8. **Eccentricity and resonance**: e = 0.3 with free rotation; 3:2 resonance at e = 0.3.
   Unit check: orbit-mean flux = S·(1 − e²)^(−1/2). *Record* seasonal extremes and compare
   with one published study each (e.g. Bolmont et al. 2016; Boutle et al. 2017 for a
   Proxima-b-like 3:2 case).
9. **Pressure**: Earth-like world at 0.3, 1, 3 bar: surface warms and the equator-to-pole
   difference shrinks with pressure (compare Kaspi & Showman 2015, Komacek & Abbot 2019).

**Accept:** items 1–7 and 9 pass their criteria; item 8 recorded. A benchmark that fails
goes to "Open questions" with the evidence; more than two failures block P8.

#### P8 — Local statistics, downscaling, biomes, exports (the owner's headline outputs)

Implement §9.4.9 and finish the tools.

1. Stations: all fields of §9.3.4 including `gust`, `prsn`, `clt`; hourly.
2. Point-rain and gust generators calibrated on the P5/P6 Earth run against public hourly
   station data from at least 20 stations in different Köppen classes, half of them held
   out.
3. Downscaling per §9.4.9; Köppen, Holdridge, Whittaker; rivers and lakes; snow line and
   `glacier` mask; wind roses; ocean-current maps.
4. Export: 16-bit PNG and GeoTIFF layers with a JSON legend; one command
   (`tools/make_atlas.py <run>`) that produces the complete set for a world.

**Accept:**

- Held-out stations: wet-hour frequency within a factor 1.5; P(≥ 2 mm/h) and
  P(≥ 20 mm/h) within a factor 3; 95th percentile of 10 m wind within 30%; monthly-mean
  diurnal temperature range within 3 K, for at least 75% of the stations.
- Downscaled Earth precipitation in four mountain boxes (Alps, Cascades, southern Andes,
  Himalaya): the windward/lee contrast has the observed sign in all four; correlation with
  a fine reference climatology (e.g. WorldClim) is higher than for the bilinearly
  interpolated model field in at least three.
- Cell means preserved by the downscaling to 1e-6 relative.
- Earth rivers: at least 7 of the 10 largest river basins by discharge are among the
  model's 20 largest.
- `make_atlas.py` runs on the Earth run, the test world and a tidally locked world.

#### P9 — Viewer

Wind particles or streamlines, ocean currents, day/night shading, layer toggles (surface
temperature, precipitation, snow, sea ice, clouds, biomes), clicking a point to see its
monthly statistics, a planet set-up panel that writes a schema-2 TOML, playback of saved
output. The UI stays in GDScript; data handling in Rust.

**Accept:** headless check as in M3 for a full-physics world; screenshots in
`docs/figures/P9/`; the owner checks the visuals.

#### P10 — Procedural terrain (needs go-ahead; write its own spec first)

Plate-tectonics-like terrain generation producing the heightmap that `[surface] geography`
reads. Out of scope until the owner asks; the first step is a short spec appended here.

#### P11 — Extended envelope (needs go-ahead, item by item)

- correlated-k opacity tables (`KTable` provider; generation tool based on public
  opacity data) for thin, thick and CO₂-rich atmospheres;
- non-dilute moist thermodynamics and virtual temperature in the dynamics;
- CO₂ condensation; non-water condensables (methane);
- several stars (the beam list), moons (eclipses, tidal heating as a surface flux);
- thermodynamic refinements of sea ice; ice-sheet elevation feedback;
- biome feedback (albedo and roughness from the diagnosed biome, iterated);
- validation against owner-supplied ExoPlaSim runs.

### 9.7 Known structural limits (state them in the README and in atlas legends)

- No deep ocean circulation: regions warmed by overturning on Earth (north-east Atlantic)
  will be too cold.
- T42 cells are about 300 km: no tropical cyclones, fronts are smooth, mountains are low
  and broad. Local detail comes from statistical downscaling and is labelled as such.
- Clouds are diagnostic and tuned on Earth. On tidally locked planets published models
  disagree with each other on clouds; landing inside their spread is the best available.
- No ozone and no stratospheric chemistry: the stratosphere is too cold and not a target.
- No sub-grid orographic drag: winter jets tend to be too strong and too zonal.
- One saturation curve, no cloud microphysics, no aerosols, no carbon or vegetation
  dynamics, no ice-sheet flow.
- Spectral advection of moisture produces small negative values that a global fixer
  repairs (monitored, §9.8).

### 9.8 Conditional work items and decision rules

Do these **only** when their trigger fires; record the trigger evidence.

| Item | Trigger | Remedy |
|---|---|---|
| Grid-point tracer transport | fixer moves more than 1% of the global vapour mass per day, or precipitation ripples next to orography exceed 20% of the local mean in the P5 maps | horizontal semi-Lagrangian transport of tracers on the Gaussian grid (shape-preserving cubic interpolation, mass fixer), vertical in flux form |
| Temperature diffusion over mountains | spurious precipitation or temperature anomalies locked to steep orography | apply the hyperdiffusion to T minus a reference profile along σ (pressure-surface correction) |
| Orographic drag | P5 jets or sea-level pressure rows fail with too-strong, too-zonal flow | effective roughness from the sub-grid orography variance of the heightmap |
| Sponge on by default | top-level winds exceed 150 m s⁻¹ or runs blow up at the top in P7 | enable `[dynamics.sponge]` for that case family |
| Shorter time step | max Courant number above 0.8 in the log | reduce Δt for that case; do not raise diffusion |
| Rain re-evaporation | P5 deserts too wet with the other rows passing | `reevaporation = true`, tune its knob |
| Longer levels / higher top | tropopause not resolved for warm or thick atmospheres (P7) | a 32-level set with the same lower levels |

### 9.9 When to ask for a review

Work proceeds without external review unless one of these happens; then stop that line of
work, write the evidence in "Open questions for the owner", and continue elsewhere:

- a gate (P5, P6b, P7) fails in more than two rows after the allowed tuning;
- a fix seems to need a change in Part I (the dynamical core) beyond §9.8;
- a published benchmark contradicts a formulation prescribed in §9.4;
- the envelope (P-5) would have to change;
- cost exceeds twice the P5 budget.

---

## 10. Session protocol (for Claude sessions)

1. Read `SPEC.md` in full, then `docs/progress.md` (status table and the last three
   entries at least), `docs/design-decisions.md` and `docs/tuning.md`. Continue at the
   lowest milestone whose acceptance criteria are not all met: first Part I (§8), then
   P0–P9 in order (§9.6). Do not start P10 or P11 without the owner's go-ahead.
2. One branch per work segment, `p<n>-<short-name>` (Part I work: `m<n>-…`), from the
   latest merged state; commit after each passing test group; open a PR when the segment is
   done. `main` holds accepted work only.
3. **Never weaken an acceptance criterion to make it pass.** If a criterion looks wrong
   after checking the code against the equations and the paper:
   - write the evidence (numbers, plots, reasoning) and a proposed replacement in
     `docs/progress.md` under "Open questions for the owner";
   - mark the item as blocked;
   - continue with the next independent item.
4. **Papers first.** Fetch the paper before implementing a scheme or benchmark; transcribe
   constants with the citation; the paper wins over this document (P-3). If a paper cannot
   be reached, say so and mark the constants as unverified in the case file.
5. **Schemes arrive with tests.** A new physics scheme needs its conservation and
   limiting-case tests in the single-column model before it runs in 3-D (§9.5.1).
6. **Tuning** only as P-4 allows, logged in `docs/tuning.md`.
7. **Build and run discipline:**
   - long runs in `--release`, in the background, several lanes of 4 threads
     (`scripts/night_runs` pattern); never paste large outputs into the conversation;
   - develop at T21 (3-year smoke runs), accept at T42;
   - anything over about 30 s is `#[ignore]` and listed in the milestone entry;
   - a run that produced NaN, or a job that exited non-zero, is reported as such.
8. **Look at every figure you produce** (P-6) and write what it shows.
9. Keep `toyclim-core` free of Godot and of I/O outside `io/`; no `unsafe` outside
   `toyclim-godot`; public APIs get doc comments with units; determinism rules of §3 hold
   for all new code (surface, ocean, physics state included).
10. **`docs/progress.md` entry per session:** date, milestone, what was done, measured
    numbers (with T, levels, Δt, run length), scorecard file names, figures with one
    sentence each, tuning changes, open questions, next step. Update the status table.
11. When a milestone is accepted, run `scripts/regress` (P-7) and record its result.
12. Review triggers: §9.9.

---

## 11. References

Citations for Part II were written from memory. Verify each when it is first used (P-3);
correct this list when a citation turns out to be wrong.

**Part I**

- Williamson, Drake, Hack, Jakob & Swarztrauber (1992), *A standard test set for numerical
  approximations to the shallow water equations in spherical geometry*, J. Comput. Phys.
  102, 211–224.
- Galewsky, Scott & Polvani (2004), *An initial-value problem for testing numerical models
  of the global shallow-water equations*, Tellus A 56, 429–440.
- Thuburn & Li (2000), *Numerical simulations of Rossby–Haurwitz waves*, Tellus A 52,
  181–189.
- Hoskins & Simmons (1975), *A multi-layer spectral model and the semi-implicit method*,
  QJRMS 101, 637–655.
- Simmons & Burridge (1981), *An energy and angular-momentum conserving vertical
  finite-difference scheme and hybrid vertical coordinates*, Mon. Wea. Rev. 109, 758–766.
- ECMWF IFS Documentation, Part III: Dynamics and numerical procedures.
- Durran, *Numerical Methods for Fluid Dynamics* (Springer).
- Williams (2009), *A proposed modification to the Robert–Asselin time filter*, Mon. Wea.
  Rev. 137, 2538–2546.
- Held & Suarez (1994), *A proposal for the intercomparison of the dynamical cores of
  atmospheric general circulation models*, BAMS 75, 1825–1830.
- Jablonowski & Williamson (2006), *A baroclinic instability test case for atmospheric
  model dynamical cores*, QJRMS 132, 2943–2975.

**Idealized moist physics and radiation**

- Reed & Jablonowski (2012), *Idealized tropical cyclone simulations of intermediate
  complexity*, JAMES 4, M04001.
- Thatcher & Jablonowski (2016), *A moist aquaplanet variant of the Held–Suarez test*,
  Geosci. Model Dev. 9, 1263–1292.
- Frierson, Held & Zurita-Gotor (2006), *A gray-radiation aquaplanet moist GCM. Part I*,
  JAS 63, 2548–2566.
- Frierson (2007), *The dynamics of idealized convection schemes and their effect on the
  zonally averaged tropical circulation*, JAS 64, 1959–1976.
- O'Gorman & Schneider (2008), *The hydrological cycle over a wide range of climates
  simulated with an idealized GCM*, J. Climate 21, 3815–3832.
- Byrne & O'Gorman (2013), *Land–ocean warming contrast over a wide range of climates*,
  J. Climate 26, 4000–4016 (water-vapour-dependent gray optical depth).
- Geen, Czaja & Haigh (2016), *The effects of increasing humidity on heat transport by
  extratropical waves*, GRL 43, 8314–8321 (two-band longwave with a window).
- Vallis et al. (2018), *Isca, v1.0*, Geosci. Model Dev. 11, 843–859.
- Koll & Cronin (2018), *Earth's outgoing longwave radiation linear due to H₂O greenhouse
  effect*, PNAS 115, 10293–10298.
- Lacis & Hansen (1974), *A parameterization for the absorption of solar radiation in the
  Earth's atmosphere*, JAS 31, 118–133.
- Joseph, Wiscombe & Weinman (1976), *The delta-Eddington approximation for radiative flux
  transfer*, JAS 33, 2452–2459.
- Slingo (1989), *A GCM parameterization for the shortwave radiative properties of water
  clouds*, JAS 46, 1419–1427.
- Sundqvist, Berge & Kristjánsson (1989), *Condensation and cloud parameterization studies
  with a mesoscale numerical weather prediction model*, Mon. Wea. Rev. 117, 1641–1657.

**Surface, boundary layer, ocean**

- Louis (1979), *A parametric model of vertical eddy fluxes in the atmosphere*,
  Bound.-Layer Meteor. 17, 187–202.
- Troen & Mahrt (1986), *A simple model of the atmospheric boundary layer*, Bound.-Layer
  Meteor. 37, 129–148.
- Best, Beljaars, Polcher & Viterbo (2004), *A proposed structure for coupling tiled
  surfaces with the planetary boundary layer*, J. Hydrometeor. 5, 1271–1278.
- Manabe (1969), *Climate and the ocean circulation I*, Mon. Wea. Rev. 97, 739–774.
- Semtner (1976), *A model for the thermodynamic growth of sea ice in numerical
  investigations of climate*, J. Phys. Oceanogr. 6, 379–389.
- Codron (2012), *Ekman heat transport for slab oceans*, Clim. Dyn. 38, 379–389.
- Stommel (1948), *The westward intensification of wind-driven ocean currents*, Trans.
  AGU 29, 202–206.
- Neale & Hoskins (2000), *A standard test for AGCMs including their physical
  parametrizations I: the proposal*, Atmos. Sci. Lett. 1, 101–107.

**Exoplanet and parameter-sweep comparators**

- Merlis & Schneider (2010), *Atmospheric dynamics of Earth-like tidally locked
  aquaplanets*, JAMES 2, 13.
- Heng, Menou & Phillipps (2011), *Atmospheric circulation of tidally locked exoplanets: a
  suite of benchmark tests for dynamical solvers*, MNRAS 413, 2380–2402.
- Kaspi & Showman (2015), *Atmospheric dynamics of terrestrial exoplanets over a wide
  range of orbital and atmospheric parameters*, ApJ 804, 60.
- Komacek & Abbot (2019), *The atmospheric circulation and climate of terrestrial planets
  orbiting Sun-like and M dwarf stars over a broad range of planetary parameters*,
  ApJ 871, 245.
- Fauchez et al. (2020), *TRAPPIST-1 Habitable Atmosphere Intercomparison (THAI):
  motivations and protocol*, Geosci. Model Dev. 13, 707–716.
- Turbet et al. (2022) and Sergeev et al. (2022), *The TRAPPIST-1 Habitable Atmosphere
  Intercomparison (THAI)*, Parts I and II, Planet. Sci. J. 3, 211 and 212.
- Haqq-Misra, Wolf, Joshi, Zhang & Kopparapu (2018), *Demarcating circulation regimes of
  synchronously rotating terrestrial planets within the habitable zone*, ApJ 852, 67.
- Noda et al. (2017), *The circulation pattern and day–night heat transport in the
  atmosphere of a synchronously rotating aquaplanet*, Icarus 282, 1–18.
- Yang, Cowan & Abbot (2013), *Stabilizing cloud feedback dramatically expands the
  habitable zone of tidally locked planets*, ApJL 771, L45.
- Shields et al. (2013), *The effect of host star spectral energy distribution and
  ice-albedo feedback on the climate of extrasolar planets*, Astrobiology 13, 715–739.
- Joshi & Haberle (2012), *Suppression of the water ice and snow albedo feedback on
  planets orbiting red dwarf stars*, Astrobiology 12, 3–8.
- Ferreira, Marshall, O'Gorman & Seager (2014), *Climate at high obliquity*, Icarus 243,
  236–248.
- Linsenmeier, Pascale & Lucarini (2015), *Climate of Earth-like planets with high
  obliquity and eccentric orbits*, Planet. Space Sci. 105, 43–59.
- Bolmont et al. (2016), *Habitability of planets on eccentric orbits: limits of the mean
  flux approximation*, A&A 591, A106.
- Boutle et al. (2017), *Exploring the climate of Proxima B with the Met Office Unified
  Model*, A&A 601, A120.
- Paradise et al. (2022), *ExoPlaSim: extending the Planet Simulator for exoplanets*,
  MNRAS 511, 3272–3303.

**Downscaling and classification**

- Smith & Barstad (2004), *A linear theory of orographic precipitation*, JAS 61,
  1377–1391.
- Peel, Finlayson & McMahon (2007), *Updated world map of the Köppen–Geiger climate
  classification*, Hydrol. Earth Syst. Sci. 11, 1633–1644.
- Beck et al. (2018), *Present and future Köppen–Geiger climate classification maps at
  1-km resolution*, Sci. Data 5, 180214.

**Reference implementations.** SpeedyWeather.jl (MIT) may be used as a porting reference.
PlaSim, ExoPlaSim and Isca are GPL-licensed: read their papers and manuals, compare
against their output, but **do not copy or translate their source** into this MIT-licensed
repository.
