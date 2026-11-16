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
| Surface albedo (gray) | `albedo` | 0.3 | proto | Frierson et al. (2006) |
| SBM relaxation time τ | `tau_convection` | 2 h | knob (P1/P5) | Frierson (2007) |
| SBM reference RH | `rh_convection` | 0.7 | knob (P1/P5) | Frierson (2007) |
| Surface exchange C_H = C_E | `c_h`, `c_e` | 0.0044 | proto | TJ16; replaced by bulk formulas (P1) |
| Boundary-layer mixing p_pbl, p_strato | `p_pbl`, `p_strato` | 850, 100 hPa | proto | TJ16; replaced by K-profile (P1) |
| Rayleigh drag k_f, σ_b | `tau_friction`, `sigma_b` | 1 d, 0.7 | proto | HS94; removed from worlds in P1 |
| Clouds RH_c, SW reflect, LW τ | `cloud_rh_crit`, `cloud_sw_reflect`, `cloud_lw_tau` | 0.8, 0.25, 3 | proto (experimental) | none (untuned); replaced in P4 |

## Prototype surface (`[surface]`; replaced in P2)

| Parameter | Key | Default | Status | Source |
|---|---|---|---|---|
| Mixed-layer depth | `mixed_layer_depth` | 20 m | knob (P6: 50 m) | common slab choice |
| Land heat capacity | `land_heat_capacity` | 2e5 J m⁻² K⁻¹ | proto | ≈ 5 cm water; replaced by soil layers (P2) |
| Land albedo | `land_albedo` | 0.3 | proto | none; replaced by §9.4.6 (P2) |
| Ocean albedo | `ocean_albedo` | radiation `albedo` | proto | — |
| Bucket capacity | `bucket_capacity` | 150 kg m⁻² | knob | Manabe (1969) |
| Initial bucket fill | `initial_bucket` | 0.75 | proto | — |
| Ice/snow albedo switch | `ice_albedo` | 0.6 | proto | none; removed in P2 (§9.4.6) |
| Ocean freezing / land snow threshold | `ocean_freeze`, `land_freeze` | 271.15 K, 273.15 K | proto | — |
| Prescribed ocean heat transport Q₀ | `ocean_heat_transport` | 0 (worlds: 50 W m⁻²) | proto | Merlis et al. (2013) form, φ₀ = 16° |

## Orbit defaults (become derived quantities in config schema v2, P0)

| Parameter | Key | Default | Source |
|---|---|---|---|
| Year | `[orbit] year` | 365.25 d | Earth |
| Obliquity, perihelion longitude | `obliquity`, `perihelion_longitude` | 23.44°, 282.9° | Earth |
| Instellation | `instellation` | 1361 W m⁻² | Kopp & Lean (2011) |

## Phase 2 knobs

Added here with their defaults and sources as each scheme is implemented (SPEC.md §9.4,
marked **[knob]** there).
