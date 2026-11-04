# toyclim — specification

A laptop-scale, qualitatively plausible general circulation model for worldbuilding,
including exoplanets (tidally locked, slow and fast rotators). Rust core, Godot viewer.

`toyclim` is a working name; rename freely.

This document is the contract for implementation sessions. It defines architecture,
conventions, the numerical method, and milestones with acceptance tests. Milestones M0–M6
(a dry spectral GCM with a viewer) are in scope now. M7+ are sketched only so that early
design decisions do not block them; **do not start M7+ without the owner's go-ahead**.

---

## 1. Goals and non-goals

**Goals**

- Spectral transform dynamical core on the sphere (shallow water, then σ-coordinate
  primitive equations), T21–T85, with clean, tested operators.
- Planet parameters (radius, rotation rate including zero/negative, gravity, gas constants,
  surface pressure) are data, not code.
- Deterministic: same config and same build give bitwise-identical output regardless of
  thread count.
- A headless runner for long spin-ups and experiments, and a Godot viewer that runs the
  model live in-process.
- Outputs readable from Python (numpy) for analysis and comparison with reference models
  (ExoPlaSim, SpeedyWeather, Isca).

**Non-goals (for now)**

- Moist physics, radiation, clouds, ocean, land (M7+).
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
interface must leave room for humidity, surface fields and per-column diagnostics (M7+)
without changing call sites; using a struct of slices or a builder is fine.

### 5.5 Insolation geometry (needed in M5, extended in M8)

A `Star`/`Orbit` description provides the substellar point (λ_ss(t), φ_ss(t)).

- Tidally locked: λ_ss is constant (default π), φ_ss = 0.
- Otherwise: λ_ss advances at the synodic rate; obliquity and eccentricity come in M8.

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

## 8. Milestones and acceptance tests

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

## 9. Future milestones (do not start without the owner's go-ahead)

Short notes so that M0–M6 choices stay compatible:

- **M7 Moisture:**
  - specific humidity q (spectral advection with a negative-value fixer and global mass
    fixer; semi-Lagrangian grid-point advection may replace it later);
  - large-scale condensation;
  - simplified Betts–Miller convection (Frierson 2007) or a SPEEDY-like mass flux scheme
    (SpeedyWeather.jl, MIT, is a porting reference);
  - surface evaporation over an aquaplanet with fixed SST.
- **M8 Radiation and insolation:**
  - orbit with obliquity, eccentricity and rotation (incl. tidal locking);
  - stellar spectrum (blackbody T_eff) split into bands;
  - gray → two-stream multi-band with water-vapour and CO₂ absorbers, with an IR window;
  - hook for precomputed correlated-k tables.
- **M9 Surface:**
  - slab ocean (mixed-layer depth, diffusive and Ekman heat transport, Codron 2012);
  - thermodynamic sea ice;
  - land bucket (soil heat capacity, moisture, snow, albedo);
  - orography from a heightmap with spectral filtering (avoid Gibbs ringing).
- **M10 Clouds:** diagnostic RH-based large-scale clouds, convective clouds, cloud
  radiative effects.
- **M11 Downscaling and biomes:**
  - lapse-rate temperature correction to a fine heightmap;
  - Smith & Barstad (2004) linear orographic precipitation via FFT, driven by sampled GCM
    winds and moisture;
  - Köppen and Holdridge classification;
  - viewer layers.
- **M12 Validation suite:** Earth-like and tidally locked aquaplanets against owner-supplied
  ExoPlaSim runs in `reference/exoplasim/`, and the THAI TRAPPIST-1e protocols.

---

## 10. Session protocol (for Claude sessions)

1. Read `SPEC.md` and `docs/progress.md` first. Continue at the lowest milestone whose
   acceptance criteria are not all met.
2. Work on a branch `m<n>-<short-name>`; commit at least after each passing test group;
   open a PR when the milestone is accepted.
3. **Never weaken an acceptance criterion to make it pass.** If a criterion looks wrong
   after checking the code against the equations:
   - write the evidence (numbers, plots, reasoning) and a proposed replacement in
     `docs/progress.md` under "Open questions for the owner";
   - mark the item as blocked;
   - continue with the next independent item.
4. **Build and run discipline:**
   - Build long runs with `--release` and run them in the background; never paste large
     outputs into the conversation.
   - Write summaries (numbers, PNGs) instead.
   - Keep test runtimes reasonable: anything over ~30 s is `#[ignore]` and listed in the
     milestone entry.
5. Keep `toyclim-core` free of Godot and I/O side effects outside `io/`. Public APIs get doc
   comments stating units.
6. **`docs/progress.md` entry format per session:**
   - date
   - milestone
   - what was done
   - measured numbers (errors, drifts, timings, with T and Δt)
   - figures produced
   - open questions
   - next step
7. Do not start milestones M7+; propose them in progress.md if M6 is done.

---

## 11. References

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
- ECMWF IFS Documentation, Part III: Dynamics and numerical procedures (spectral transform
  and semi-implicit details).
- Durran, *Numerical Methods for Fluid Dynamics* (Springer), chapter on spectral methods
  on the sphere.
- Williams (2009), *A proposed modification to the Robert–Asselin time filter*, Mon. Wea.
  Rev. 137, 2538–2546.
- Held & Suarez (1994), *A proposal for the intercomparison of the dynamical cores of
  atmospheric general circulation models*, BAMS 75, 1825–1830.
- Jablonowski & Williamson (2006), *A baroclinic instability test case for atmospheric
  model dynamical cores*, QJRMS 132, 2943–2975.
- Merlis & Schneider (2010), *Atmospheric dynamics of Earth-like tidally locked
  aquaplanets*, JAMES 2, 13.
- Heng, Menou & Phillipps (2011), *Atmospheric circulation of tidally locked exoplanets:
  a suite of benchmark tests for dynamical solvers*, MNRAS 413, 2380–2402.
- Frierson (2007), *The dynamics of idealized convection schemes and their effect on the
  zonally averaged tropical circulation*, JAS 64, 1959–1976.
- Codron (2012), *Ekman heat transport for slab oceans*, Clim. Dyn. 38, 379–389.
- Smith & Barstad (2004), *A linear theory of orographic precipitation*, JAS 61,
  1377–1391.
- Reference implementations: SpeedyWeather.jl (MIT), PUMA/PlaSim and ExoPlaSim, Isca.
