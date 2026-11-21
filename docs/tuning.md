# Tuning register

SPEC.md §9.2 P-4. Every tunable parameter is listed here with its default, allowed range,
source, and the date and reason of every change.

Rules (from P-4):

- Tune **only on Earth targets** (P4, P5, P6); values are then frozen.
- **Never tune on an exoplanet benchmark.** A failing exoplanet gate is fixed by physics or
  recorded as an open question.
- A knob may not be a function of latitude, longitude or planet name. (The gray scheme's
  τ₀(φ) violates this; it is a prototype prescription kept for idealized comparators; since
  P3 it is the option `latitude_dependence`, on only in Earth-like idealized cases, SPEC.md
  §9.1.1 item 4.)

Status columns: **fixed** = taken from a paper and not to be tuned; **knob** = tunable within
the range; **proto** = prototype parameter that Phase 2 replaces (listed so that world runs
until then are reproducible; do not tune).

## Change log

| Date | Parameter | Old → new | Reason, evidence |
|---|---|---|---|
| 2026-11-15 | — | — | Register created in P0 from the prototype code (commit 0f7d33a). No values changed. |
| 2026-11-16 | P1 surface exchange and boundary layer | new | Defaults from Louis (1979), Frierson et al. (2006) and SPEC.md §9.4.2–9.4.3; not tuned (tuning is P4–P6). World cases: TJ16 exchange and Rayleigh drag removed. |
| 2026-11-16 | `tau_sw` in the APE benchmark cases | 0.22 → 0 | Not tuning: the benchmark configuration follows Frierson et al. (2006, no SW absorption). With 0.22 along the slant path the gray scheme heats the polar upper troposphere by several K/day at low sun (P1 SCM diagnosis). World cases keep 0.22 until P3 replaces the radiation. |
| 2026-11-16 | `[dynamics] tracer_transport` in the moist world and APE cases | spectral → semi_lagrangian | Not tuning: SPEC.md §9.8 remedy after the tracer-fixer trigger fired (2.3%/day of the vapour mass at T42). |
| 2026-11-16 | Surface (P2) | prototype land 2e5 J m⁻² K⁻¹ / albedo 0.3 and the 0.6 ice switch → soil, snow, sea ice, band albedos | Not tuning: SPEC.md §9.4.5–9.4.6 schemes at their published or SPEC starting values. |
| 2026-11-16 | `[radiation] reflectance` in the world cases | (new) 0 → 0.23 | Not fitted: observed fraction of the incoming shortwave reflected by clouds and atmosphere, 79/341.3 W m⁻² (Trenberth, Fasullo & Kiehl 2009, Fig. 1); a stand-in for the P4 clouds once the P2 surface albedos replaced the prototype's 0.3. Default 0 (APE, idealized cases unchanged). |
| 2026-11-17 | `[surface.ocean] model = "prescribed"` (Earth AMIP case) | (new) | Not tuning: AMIP-style boundary condition for P2 item 7 and P5; observed monthly SST and ice concentration (OISST v2, 1991–2020) with floe thickness 2 m / 1 m from the CICE prescribed-ice mode. |
| 2026-11-17 | Semi-gray radiation (P3), all knobs | (new) | Fitted, not tuned on climate: `tools/calibrate_radiation.py` fits the semi-gray knobs to RRTMG clear-sky fluxes (climlab 0.9.2) for 630 longwave and 2 × 144 shortwave columns (docs/radiation/calibration.md). Starting values Geen et al. (2016) via the Isca documentation. Earth configuration adds the fitted ozone column. |
| 2026-11-17 | Gray `latitude_dependence` | (new) implicit on → off by default | SPEC.md §9.6 P3 / defect 4: the Earth-shaped τ₀(φ) is now an option; every Earth-like gray case sets it on (results unchanged, regress), the tidally locked cases leave it off (results change on purpose). |
| 2026-11-18 | `[radiation] reflectance` with `scheme = "semigray"` (exploratory Earth AMIP T21 run `out/p3/earth_amip_t21_semigray_r165.toml` only; no case file changed) | 0.23 → 0.165 | Earth target: global planetary albedo. With 0.23 the semigray run had albedo 0.354 (clear-sky gas scattering and absorption add to the stand-in), against 0.30 observed (Trenberth, Fasullo & Kiehl 2009). 0.165 solves r + (1 − r)·0.161 = 0.30; measured 0.298. Exploratory, to separate the radiation change from the albedo error; the cloud stand-in goes away in P4. |
| 2026-11-18 | `[surface.ocean] albedo` in `cases/bench/p3_aquaplanet/aquaplanet_t21.toml` | (new) 0.38 → 0.25 | Case setting for the SPEC.md P3 acceptance 4 cloud stand-in, not fitted to an output of the test: with clear-sky radiation the stand-in must carry the net cloud effect (shortwave and longwave), so the value puts the 285 K warm start in TOA balance. 60-day runs: 0.22 / 0.26 / 0.30 → planetary albedo 0.225 / 0.252 / 0.279, net TOA +5.3 / −2.3 / −11.0 W m⁻². 0.38 (from one offline global-mean column) froze the planet over: 236 K after 10 years. |
| 2026-11-18 | Diagnostic clouds (P4), all knobs | (new) | Not tuned yet: SPEC.md §9.4.7 starting values (table "Clouds" below); optics constants from Slingo (1989) and Ebert & Curry (1992) via the NCAR CAM source. |
| 2026-11-21 | Freeze-dry low-cloud reduction (new physics, `freeze_dry_q`, `freeze_dry_min`, `freeze_dry_sigma`) | (new) off → on, 3e-3 kg/kg, 0.15, σ 0.75 | Not a fitted value: the form and constants of Vavrus & Waliser (2008) as in NCAR CAM `cloud_fraction.F90` (cam_cesm2_2_rel); the paper could not be fetched (HTTP 403); σ 0.75 for CAM's `premib` is unverified (σ 0.70 gave identical results at L26). Reason: P4 completion plan step 4, the RH_c knobs alone could not remove the polar cloud excess (below). T21 Earth AMIP 1 + 1 years: polar cover 0.86 → 0.81 with the old RH_c. |
| 2026-11-21 | Clouds `rh_crit_surface`, `rh_crit_upper`, `water_ratio` (code defaults) | 0.90, 0.80, 0.02 → 0.95, 0.90, 0.03 | Earth targets only (Earth AMIP T21, 1 + 1 years, climatology year 2; observed planetary albedo ≈ 0.30, CRE ≈ −47 / +26 W m⁻² (CERES EBAF, quoted from memory, not checked; the P4 ranges are −30 to −60 / +15 to +35), the zonal shape of SPEC.md P4 criterion 3). Explored within the ranges: defaults, s 0.95, s 0.95 + u 0.85, n 1, n 8 (no freeze-dry) all left the North Pole at 0.83–0.89 and no northern storm-track maximum; with freeze-dry: s 0.95 + u 0.85 / 0.90 / 0.95, s 0.90 + u 0.90, s 0.95 + u 0.90 + β 0.03. Chosen: s 0.95, u 0.90, β 0.03: albedo 0.301, OLR 237.5, net +0.2, SW CRE −52.4, LW CRE +23.6, cover 0.534; northern storm-track maximum at 64° (local), southern maximum at the 69° band edge (shape row still fails at T21). β restores the albedo lost to the higher RH_c. Not chosen with any reference to the 2×CO₂ warming. Runs: `out/p4/rhc/`, cases `cases/bench/p4_earth/rhc/`. |
| 2026-11-21 | SBM shallow convection (new physics, `[convection] shallow`) | (none: columns with CAPE > 0 and P_q ≤ 0 left alone) → "shallower" | Not a fitted value: closes the SPEC.md §9.4.4 gap ("the paper's shallow-convection treatment"). Frierson (2007, J. Atmos. Sci. 64, 1959–1976, §2c, eqs. 7–9; read from the author's copy, atmos.uw.edu/~dargan/papers/f07b.pdf): the paper's control. Reason: P4 open question 13; in the slab tropics 66 % (400 ppm) / 81 % (800 ppm) of column-steps had CAPE > 0 but were skipped, active convection 31 % → 16 %, CAPE 1012 → 2444 J/kg. Default for `[convection] scheme = "sbm"`; `shallow = "none"` restores the P1 behaviour; the idealized forcing bundles keep "none". Invalidates the P4 cloud tuning (to be redone). |

## Dynamical core (Part I; frozen regression base)

| Parameter | Config key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Hyperdiffusion order p (∇²ᵖ) | `[diffusion] order` | 4 | 2–4 | fixed | SPEC.md §4.5 |
| Hyperdiffusion time τ_h at ℓ = M | `[diffusion] tau` | 2.4 h (T42) | — | fixed per resolution | SPEC.md §4.5; HS94 practice |
| RAW filter ν | `[time_filter] nu` | 0.1 | — | fixed | Williams (2009) |
| RAW filter α | `[time_filter] alpha` | 0.53 | — | fixed | Williams (2009) |
| Reference temperature T̄ | (code) | 300 K | ≥ warmest typical T | fixed | SPEC.md §5.2 |
| Dry-mass fixer | `mass_fixer` | true | — | fixed | prototype (M4) |
| Orography spectral filter, damping at ℓ = M | `[surface] orography_filter` | 0.1 | — | proto | prototype (M9) |

## Idealized bundles (frozen regression physics; never tuned)

| Parameter | Key in `[forcing]` | Default | Source |
|---|---|---|---|
| HS ΔT_y, Δθ_z, T_surface, T_min | `delta_ty`, `delta_theta_z`, `t_surface`, `t_min` | 60 K, 10 K, 315 K, 200 K | Held & Suarez (1994) |
| HS k_f, k_a, k_s, σ_b | `tau_friction`, `tau_free`, `tau_surface`, `sigma_b` | 1 d, 40 d, 4 d, 0.7 | Held & Suarez (1994) |
| Tidally locked T_night, ΔT_d | `t_night`, `delta_td` | 250 K, 70 K | SPEC.md §8 M5 (own definition) |
| TJ16 T_surface, ΔT_y | `t_surface`, `delta_ty` | 294 K, 65 K | Thatcher & Jablonowski (2016) eqs. 18–20 |
| TJ16 C_H = C_E | `c_h`, `c_e` | 0.0044 | TJ16 eqs. 7–12 |
| TJ16 SST ΔT, T_min, Δφ | `sst_delta`, `sst_min`, `sst_width` | 29 K, 271 K, 26° | TJ16 eq. 6 |
| TJ16 p_pbl, p_strato | `p_pbl`, `p_strato` | 850 hPa, 100 hPa | TJ16 eqs. 15–17 |
| Water L_v, R_v, e₀, T₀ | `[moisture]` overrides | 2.5e6 J kg⁻¹, 461.5 J kg⁻¹ K⁻¹, 610.78 Pa, 273.16 K | TJ16 eqs. 1–5 |

## Prototype world physics (`[forcing] type = "gray_aquaplanet"`; replaced in P1–P4)

| Parameter | Key | Default | Status | Source |
|---|---|---|---|---|
| Gray LW optical depth, equator / pole | `tau_eq`, `tau_pole` | 6, 1.5 | proto | Frierson, Held & Zurita-Gotor (2006) |
| Gray LW linear fraction f_l | `f_linear` | 0.1 | proto | Frierson et al. (2006) |
| LW optical depth per kg m⁻² vapour | `tau_q` | 0 | proto | Byrne & O'Gorman (2013) form, off |
| SW absorption optical depth τ_sw (∝ σ²) | `tau_sw` | 0.22 | proto | O'Gorman & Schneider (2008) |
| SW reflectance at the top (cloud stand-in until P4) | `reflectance` | 0 (worlds: 0.23) | interim | Trenberth et al. (2009): 79 of 341.3 W m⁻² |
| Surface albedo (gray) | `albedo` | 0.3 | proto | Frierson et al. (2006) |
| SBM relaxation time τ | `tau_convection` | 2 h | knob (P1/P5) | Frierson (2007) |
| SBM reference RH | `rh_convection` | 0.7 | knob (P1/P5) | Frierson (2007) |
| Clouds RH_c, SW reflect, LW τ | `cloud_rh_crit`, `cloud_sw_reflect`, `cloud_lw_tau` | 0.8, 0.25, 3 | proto (experimental) | none (untuned); replaced in P4 |

## Surface (`[surface]`, P2; SPEC.md §9.4.5–9.4.6)

The prototype's single-layer land (2e5 J m⁻² K⁻¹, albedo 0.3) and the ice–albedo
temperature switch (0.6 below 271.15 / 273.15 K) were removed in P2. `[surface.land]
heat_capacity` still selects one layer of that heat capacity (P1 single-column stability
tests only).

| Parameter | Key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Mixed-layer depth | `[surface.ocean] mixed_layer_depth` | 20 m | 10–50 | knob (P6: 50 m) | common slab choice |
| Prescribed ocean heat transport Q₀ | `[surface.ocean] heat_transport` | 0 (worlds: 50 W m⁻²) | 0–60 | knob | Merlis et al. (2013) form, φ₀ = 16° |
| Ocean albedo (both bands) | `[surface.ocean] albedo` | 0.06 | — | knob | SPEC.md §9.4.6 starting value |
| Bare-land albedo vis / NIR | `[surface.land] albedo` (broadband) | 0.15 / 0.30 | — | knob | SPEC.md §9.4.6 starting values (no paper) |
| Soil volumetric heat capacity, conductivity | `soil_heat_capacity`, `soil_conductivity` | 2.0e6 J m⁻³ K⁻¹, 1.0 W m⁻¹ K⁻¹ | — | knob | SPEC.md §9.4.5 |
| Soil layers | (code) | 5, d₁ = 0.25 √(2κ/ω_day) ∈ [0.01, 0.2] m, ×3 per layer | — | fixed | SPEC.md §9.4.5 |
| Bucket capacity | `[surface.land] bucket_capacity` | 150 kg m⁻² | — | knob | Manabe (1969) |
| β = min(1, W/(0.75 W_max)) | (code) | — | — | fixed | Manabe (1969) |
| Initial bucket fill | `initial_bucket` | 0.75 | — | proto | — |
| Snow cover scale S_c (land) | `snow_cover_scale` | 15 kg m⁻² | — | knob | SPEC.md §9.4.5 |
| Snow cap | `snow_cap` | 5000 kg m⁻² | — | fixed | SPEC.md §9.4.5 |
| Dry snow albedo vis / NIR | (code) | 0.96 / 0.68 | — | knob | CAM6 Scientific Guide §8.3 |
| Melting snow: loss per K above −1 °C | (code) | 0.10 / 0.15 | — | knob | CAM6 §8.3 |
| Bare sea ice albedo vis / NIR; melt loss per K | (code) | 0.73 / 0.33; 0.075 / 0.075 | — | knob | CAM6 §8.3 |
| Snow cover on ice h_s/(h_s + 0.02 m) | (code) | S/(S + 6.6 kg m⁻²) | — | fixed | CAM6 §8.3 eq. 7, ρ_s = 330 |
| Band weights | (code) | Planck fraction below 0.7 µm at T_eff (Sun 0.49) | — | fixed | — |
| Sea-water freezing point | `[surface.sea_ice] freezing_point` | 271.35 K | — | fixed | SPEC.md §9.4.5; CAM6 −1.8 °C |
| Ice, snow conductivity | (code) | 2.034, 0.31 W m⁻¹ K⁻¹ | — | fixed | CAM6 §8.2 table |
| Ice, snow density | (code) | 917, 330 kg m⁻³ | — | fixed | CAM6 §8.2 table |
| Latent heat of fusion, T_freeze | (condensable) | 3.34e5 J kg⁻¹, 273.15 K | — | fixed | CAM6 §8.2 table |
| Sea-ice concentration scale h₀ | `[surface.sea_ice] h0` | 0.5 m | 0.2–1 | knob | SPEC.md §9.4.5 |
| Prescribed sea-ice floe thickness, NH / SH (AMIP ocean) | `[surface.ocean] ice_thickness` | 2 m / 1 m | 0.5–3 m | knob (P5) | CICE prescribed-ice mode (CICE documentation for CESM 1.0, Hunke & Lipscomb); the CAM6 Scientific Guide §8 gives 2 m / 0.5 m |
| Sea-ice thickness cap | `max_thickness` | 50 m | — | fixed | SPEC.md §9.4.5 |
| Ice fraction ramp of condensate | (condensable `ice_ramp`) | 20 K below T_freeze | 10–40 | knob | SPEC.md §9.4.4 |

## Orbit defaults (become derived quantities in config schema v2, P0)

| Parameter | Key | Default | Source |
|---|---|---|---|
| Year | `[orbit] year` | 365.25 d | Earth |
| Obliquity, perihelion longitude | `obliquity`, `perihelion_longitude` | 23.44°, 282.9° | Earth |
| Instellation | `instellation` | 1361 W m⁻² | Kopp & Lean (2011) |

## Phase 2 knobs

Added here with their defaults and sources as each scheme is implemented (SPEC.md §9.4,
marked **[knob]** there).

### Surface exchange and boundary layer (P1, `[boundary_layer] scheme = "kprofile"`)

| Parameter | Key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Gustiness w_g | `gustiness` | 1 m s⁻¹ | 0–3 | knob | SPEC.md §9.4.2 |
| Charnock constant α | `charnock` | 0.018 | 0.011–0.035 | knob | SPEC.md §9.4.2 (common NWP value) |
| Minimum ocean roughness | `z0_ocean_min` | 1.5e-5 m | — | knob | SPEC.md §9.4.2 |
| Land roughness (no map) | `z0_land` | 0.1 m | 0.01–1 | knob | SPEC.md §9.4.2 |
| Sea-ice and snow roughness | `z0_ice` | 1e-3 m | — | knob | SPEC.md §9.4.2 |
| z₀h / z₀m over land and ice | `z0h_ratio` | 0.1 | 0.01–1 | knob | SPEC.md §9.4.2 |
| Gust factors (unstable, stable) | `gust_unstable`, `gust_stable` | 3, 1 | — | knob (P8 calibrates) | SPEC.md §9.4.2 placeholder |
| Critical Richardson number Ri_c | `ri_crit` | 1 | 0.25–1 | knob | Frierson et al. (2006) Table 1 |
| Surface-layer fraction f_b | `surface_layer_fraction` | 0.1 | — | fixed | Frierson et al. (2006) Table 1 |
| Background diffusivity above h | `background_k` | 0 m² s⁻¹ | 0–1 | knob | SPEC.md §9.4.3; 0 as in Frierson et al. (2006) |
| Frictional heating | `frictional_heating` | true | — | fixed | SPEC.md §9.4.3 |
| von Kármán κ | (code) | 0.4 | — | fixed | SPEC.md §9.4.2 |
| Louis b, b′, C*_m, C*_h | (code) | 9.4, 4.7, 7.4, 5.3 | — | fixed | Louis (1979) eqs. 14–15, 20 |
| Businger γ_m, γ_h, β, R (2 m/10 m diagnostics) | (code) | 15, 9, 4.7, 0.74 | — | fixed | Businger et al. (1971) via Louis (1979) eqs. 6–9 |

### Dynamics (P1, `[dynamics]`)

| Parameter | Key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Hyperdiffusion energy fixer | `energy_fixer` | false (worlds: true) | — | fixed | SPEC.md §9.4.3 |
| Sponge on/off | `sponge.enabled` | false | — | decision rule §9.8 | SPEC.md §9.4.3 |
| Sponge bottom σ | `sponge.sigma_top` | 0.02 | — | knob | SPEC.md §9.4.3 |
| Sponge time at the top level | `sponge.tau` | 1 day | — | knob | SPEC.md §9.4.3 |
| Tracer transport | `tracer_transport` | `"spectral"` (moist worlds and APE: `"semi_lagrangian"`) | — | decision rule §9.8 (fired in P1) | SPEC.md §9.8 |
| SL midpoint iterations | (code) | 3 | — | fixed | Ritchie (1987) |
| SL interpolation | (code) | cubic Lagrange 4×4, clipped to the 2×2 range | — | fixed | Bermejo & Staniforth (1992) |
| SL vertical Courant number per sub-step | (code) | 0.5 | — | fixed | numerical (MUSCL, van Leer limiter) |
| Time filter of grid-point tracers | (code) | Robert–Asselin, ν as `[time_filter]`, α = 1 | — | fixed | positivity (all weights ≥ 0) |

### Convection (P1)

| Parameter | Key | Default | Status | Source |
|---|---|---|---|---|
| Dry adjustment | `[convection] scheme = "dry"` or with `"sbm"` | on | fixed | SPEC.md §9.4.3 |
| SBM τ, RH_ref | `[convection] tau`, `rh_ref` | 2 h, 0.7 | knob (P5) | Frierson (2007) |
| SBM shallow convection | `[convection] shallow` | "shallower" | fixed (published) | Frierson (2007) §2c, eqs. 7–9 (the paper's control) |

### Radiation (P3, `[radiation] scheme = "semigray"`; SPEC.md §9.4.1)

Fitted values from `tools/calibrate_radiation.py` (2026-11-17); residuals in
`docs/radiation/calibration.md`. Refit only with that tool (against RRTMG), never on a
climate target.

| Parameter | Key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Visible / near-IR edge | `vis_edge` | 0.75 µm | — | fixed | SPEC.md §9.4.1 |
| Share of the near-IR flux in `nir_weak` | `nir_weak_share` | 0.718 | 0.05–0.95 | fitted | RRTMG fit |
| Rayleigh column τ (vis, nir_weak, nir_strong) | `rayleigh` | 0.187, 0, 0.02 | near-IR ≤ 0.02 | fitted | RRTMG fit (near-IR bounded by the physical value, τ_R ≈ 0.008 at 1 µm) |
| Well-mixed near-IR absorption (weak, strong) | `dry_abs` | 0, 0.406 | — | fitted | RRTMG fit (O₂, CO₂, CH₄; not in SPEC.md) |
| Vapour mass absorption (weak, strong) | `k_wv` | 0.00246, 0.238 m² kg⁻¹ | — | fitted | RRTMG fit |
| Vapour/dry SW pressure exponent | `wv_n` | 0.372 | 0–2 | fitted | RRTMG fit |
| Ozone column τ, centre, width | `ozone_tau`, `ozone_p`, `ozone_width` | 0 (off), 1000 Pa, 1 | — | fitted (Earth: 0.02425, 6430 Pa) | RRTMG fit with climlab's ozone profile |
| LW rest band, weak part a, b, c, d | `lw_rest` | 0.533, 24.6, 518, 0 | — | fitted | RRTMG fit; Geen et al. (2016) form |
| LW rest band, strong part a, b, c, d | `lw_rest_strong` | 7.84, 30.9, 1.93e4, 3.84 | — | fitted | RRTMG fit (pseudo-band split, not in SPEC.md) |
| Strong share of the rest band | `lw_strong_share` | 0.427 | 0–0.95 | fitted | RRTMG fit |
| LW window a_w, b_w, c_w, d_w | `lw_window` | 0, 346, 2047, 0.0659 | — | fitted | RRTMG fit; Geen et al. (2016) form |
| Window edges | (code) | 8–14 µm | — | fixed | Geen et al. (2016) |
| LW line pressure exponent n, p_ref | `lw_n`, `lw_p_ref` | 0.453, 1e5 Pa | 0–2 | fitted | RRTMG fit (SPEC.md §9.4.1 knob n) |
| CO₂ pressure exponent, curvature κ | `lw_n_co2`, `lw_co2_curvature` | 0, 0.224 | — | fitted | RRTMG fit |
| Diffusivity factor | (code) | 1.66 | — | fixed | SPEC.md §9.4.1 |
| Two-stream method | (code) | δ-scaled PIFM | — | fixed | Joseph et al. (1976); Zdunkowski et al. (1980); RRTMG_SW default |
| Smallest cos zenith of the beam | (code) | 0.05 | — | fixed | as the gray scheme |
| Top reflectance (cloud stand-in) / forced planetary albedo | `reflectance`, `planetary_albedo` | 0 / none | — | case setting | P3 acceptance 3; replaced by P4 clouds |

### Clouds (P4, `[clouds] scheme = "diagnostic"`; SPEC.md §9.4.7)

Tunable in P4 on Earth targets only (the Earth-like aquaplanet and Earth AMIP). Optics
constants are published values, not tuned.

| Parameter | Key | Default | Range | Status | Source |
|---|---|---|---|---|---|
| Critical RH near the surface / aloft | `rh_crit_surface`, `rh_crit_upper` | 0.95, 0.9 (P4 frozen) | 0.75–0.95 (SPEC) | knob (P4) | SPEC.md §9.4.7; profile shape after ECHAM (Lohmann & Roeckner 1996, not checked) |
| RH_c profile exponent n (RH_c = u + (s − u) e^{1 − σ^−n}) | `rh_crit_exponent` | 4 | 1–8 | knob (P4) | as above |
| Convective cover b, cap (c = min(cap, b ln(1 + P/1 mm day⁻¹))) | `conv_coef`, `conv_max` | 0.125, 0.8 | 0.05–0.25, 0.3–0.8 | knob (P4) | form after Slingo (1987) |
| In-cloud water β (fraction of q_sat), cap | `water_ratio`, `water_max` | 0.03 (P4 frozen), 5e-4 kg/kg | 0.005–0.05, 2e-4–1e-3 | knob (P4) | SPEC.md §9.4.7 |
| No clouds above σ | `sigma_min` | 0.1 | — | fixed | stratosphere |
| Freeze-dry: q scale, minimum factor, lowest-cloud σ bound | `freeze_dry_q`, `freeze_dry_min`, `freeze_dry_sigma` | 3e-3 kg/kg, 0.15, 0.75 | — | fixed (published) | Vavrus & Waliser (2008) via NCAR CAM `cloud_fraction.F90`; σ bound unverified |
| Effective radius liquid / ice | `r_liquid`, `r_ice` | 10, 30 µm | fit ranges 4.2–16, 13–130 µm | knob (P4) | SPEC.md §9.4.7 |
| Liquid LW absorption (vertical) | `k_lw_liquid` | 0.090361 m² g⁻¹ | — | knob (P4) | NCAR CAM (×1.66 = 0.150; SPEC.md ≈ 0.14) |
| Ice LW absorption | (code) | 0.005 + 1/r_e m² g⁻¹ | — | fixed | NCAR CAM / Ebert & Curry (1992) |
| SW optics liquid / ice | (code) | Slingo (1989) / Ebert & Curry (1992), 4 bands; near-IR bands combined by the star's flux | — | fixed | coefficients via the NCAR CAM source (papers paywalled) |
