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

### Convection (P1)

| Parameter | Key | Default | Status | Source |
|---|---|---|---|---|
| Dry adjustment | `[convection] scheme = "dry"` or with `"sbm"` | on | fixed | SPEC.md §9.4.3 |
| SBM τ, RH_ref | `[convection] tau`, `rh_ref` | 2 h, 0.7 | knob (P5) | Frierson (2007) |
