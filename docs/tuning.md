# Tuning register

SPEC.md §9.2 P-4. Every tunable parameter is listed here with its default, allowed range,
source, and the date and reason of every change.

Rules (from P-4):

- Tune **only on Earth targets** (P4, P5, P6); values are then frozen.
- **Never tune on an exoplanet benchmark.** A failing exoplanet gate is fixed by physics or
  recorded as an open question.
- A knob may not be a function of latitude, longitude or planet name. (The gray scheme's
  τ₀(φ) violates this; it is a prototype prescription kept for idealized comparators and
  becomes an Earth-only option in P3, SPEC.md §9.1.1 item 4.)

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
