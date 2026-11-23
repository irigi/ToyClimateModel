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
| 2026-11-21 | Clouds `rh_crit_surface`, `rh_crit_upper`, `water_ratio` (code defaults) | 0.90, 0.80, 0.02 → 0.95, 0.90, 0.03 | Earth targets only (Earth AMIP T21, 1 + 1 years, climatology year 2; observed planetary albedo 0.291, SW / LW CRE −45.2 / +25.8 W m⁻², cloud area 0.67 (CERES EBAF Ed4.1 TOA, 2001–2020 climatology, Loeb et al. 2018, fetched 2026-11-22 via the Zenodo extract doi:10.5281/zenodo.10426438 by `tools/fetch_reference.py`, global means on the T42 grid; these replace the values first quoted here from memory, −47 / +26, which were close; the P4 ranges are −30 to −60 / +15 to +35), the zonal shape of SPEC.md P4 criterion 3). Explored within the ranges: defaults, s 0.95, s 0.95 + u 0.85, n 1, n 8 (no freeze-dry) all left the North Pole at 0.83–0.89 and no northern storm-track maximum; with freeze-dry: s 0.95 + u 0.85 / 0.90 / 0.95, s 0.90 + u 0.90, s 0.95 + u 0.90 + β 0.03. Chosen: s 0.95, u 0.90, β 0.03: albedo 0.301, OLR 237.5, net +0.2, SW CRE −52.4, LW CRE +23.6, cover 0.534; northern storm-track maximum at 64° (local), southern maximum at the 69° band edge (shape row still fails at T21). β restores the albedo lost to the higher RH_c. Not chosen with any reference to the 2×CO₂ warming. Runs: `out/p4/rhc/`, cases `cases/bench/p4_earth/rhc/`. |
| 2026-11-21 | SBM shallow convection (new physics, `[convection] shallow`) | (none: columns with CAPE > 0 and P_q ≤ 0 left alone) → "shallower" | Not a fitted value: closes the SPEC.md §9.4.4 gap ("the paper's shallow-convection treatment"). Frierson (2007, J. Atmos. Sci. 64, 1959–1976, §2c, eqs. 7–9; read from the author's copy, atmos.uw.edu/~dargan/papers/f07b.pdf): the paper's control. Reason: P4 open question 13; in the slab tropics 66 % (400 ppm) / 81 % (800 ppm) of column-steps had CAPE > 0 but were skipped, active convection 31 % → 16 %, CAPE 1012 → 2444 J/kg. Default for `[convection] scheme = "sbm"`; `shallow = "none"` restores the P1 behaviour; the idealized forcing bundles keep "none". Invalidates the P4 cloud tuning (to be redone). |
| 2026-11-22 | SBM shallow convection `[convection] shallow` (code default) | "shallower" → "none"; new option "qref" | Owner decision on P4 question 13: off by default, code and key kept, SPEC.md §9.4.4 gap stays open. Evidence: APE T21 year 2, fixed SST, gray, no clouds: Hadley 15.6 / 4.7 / 5.5 × 10¹⁰ kg s⁻¹ and equatorial precipitation 9.6 / 6.3 / 6.7 mm/d for none / shallower / qref (Frierson 2007 Table 1, T42 gray slab, reports only −26 % Hadley for none → shallower; our rebuild of that slab setup reproduces it, −27 %, so the collapse comes with prescribed SST). "qref" (eqs. 10–15, the reference-humidity adjustment) added as an option and tested; it does not keep the circulation either. `scripts/regress` with "none" reproduces the pre-shallow log (commit 662f27f) line for line apart from timings; `diagnostics.csv` gains three columns only. The P4 cloud tuning of 2026-11-21 (made with shallow off) stands. Not chosen with any reference to the 2×CO₂ warming. |
| 2026-11-22 | Convective cloud geometry `[clouds] convective` (new option, knobs `core_share`, `deep_sigma`, `shallow_sigma`, `anvil_water`, `conv_water_path`) | (new) "column" default, "anvil" option | Not tuned yet: P4 question 13 option (b) after the diagnosis held (tropical net CRE −43.1 against CERES −14.4 W m⁻²; the convective part carries 85 % of the tropical 2×CO₂ net CRE change; convective in-cloud water path median 1.26 kg m⁻², visible τ ≈ 100–200). Form and constants from Slingo (1987, QJRMS 113, 899–927, §3c, Table 1, Table 3; PDF supplied by the owner). "column" stays the default: byte-identical (10-day `ape_control_t21`, P4 `aquaplanet_t21`). |
| 2026-11-22 | Exploratory, T21 Earth AMIP 1 + 1 years, `convective = "anvil"` batch 1 (no default changed) | — | Earth targets only (CERES EBAF Ed4.1 bands, P4 criteria 1–3); runs `out/p4/anvil/`, cases `cases/bench/p4_earth/anvil/`. Year 2, albedo / cover / global SW, LW CRE / 20°S–20°N net CRE (CERES −14.6): column (frozen) 0.301 / 0.534 / −52.4, +23.6 / −51.6; anvil defaults 0.296 / 0.526 / −50.5, +24.6 / −45.2; + RH_c aloft 0.85: 0.309 / 0.554 / −55.1, +26.1 / −50.2; + 0.80: 0.318 / 0.573 / −58.3, +27.7 / −53.3; `conv_water_path` 0.3: 0.292 / 0.537 / −49.2, +24.7 / −42.0. All pass P4 rows 1–2 and the cover range; the shape row fails in every one (as in "column"). The anvil geometry removes 6–10 of the ≈ 37 W m⁻² tropical net CRE excess; widening RH_c aloft adds tropical reflection. Not chosen with any reference to the 2×CO₂ warming. |
| 2026-11-22 | Exploratory, T21 Earth AMIP 1 + 1 years, `convective = "anvil"` batch 2 (no default changed) | — | All on `conv_water_path` 0.3. Year 2, albedo / cover / global SW, LW CRE / 20°S–20°N SW, LW, net CRE (CERES −43.9, +29.3, −14.6): `anvil_water` 3e-5 (b_wm3) 0.291 / 0.538 / −48.9, +24.5 / −68.3, +27.1, −41.2; same + RH_c aloft 0.85 (b_wm3u85) 0.299 / 0.558 / −51.7, +26.0 / −72.3, +28.0, −44.3; β 0.02 + RH_c aloft 0.85 (b_b02u85) 0.295 / 0.558 / −50.3, +23.1 / −72.4, +25.5, −46.9; `water_max` 2e-4 + RH_c aloft 0.80 (b_wm2u80) 0.303 / 0.577 / −52.9, +27.8 / −70.4, +29.4, −41.0. Figure `docs/figures/P4/cre_zonal_t21_anvil_batch2.png`. The tropical SW CRE stays at −68 to −72 whatever the water knobs: the tropical cloud is optically saturated, so its reflection follows its fraction, not its water. Shape row fails in all. |
| 2026-11-22 | Exploratory, T21 Earth AMIP 1 + 1 years, `convective = "anvil"` batch 3, β = 0.01 (Slingo 1987 Table 3 control value; no default changed) | — | All on `conv_water_path` 0.3. Same columns: β 0.01 (c_b01) 0.269 / 0.540 / −41.4, +17.8 / −62.4, +21.3, −41.2 (albedo row fails); + RH_c aloft 0.85 (c_b01u85) 0.274 / 0.555 / −43.1, +18.7 / −64.0, +21.9, −42.2; + 0.80 (c_b01u80) 0.284 / 0.581 / −46.4, +20.1 / −68.5, +23.3, −45.2; + RH_c surface 0.90 (c_b01u80s90) 0.288 / 0.589 / −48.1, +20.1 / −71.2, +23.4, −47.8. Figure `docs/figures/P4/cre_zonal_t21_anvil_batch3.png`. β 0.01 dims the extratropics (40–65°S SW CRE −36 to −43 against CERES −66; global LW CRE ≈ +18–20 against +25.8) without fixing the tropics, so it is not taken. Shape row fails in all. |
| 2026-11-22 | Choice for the step 4 slab pair (exploratory; defaults unchanged) | b_wm2u80: `convective` "anvil", `conv_water_path` 0.3, `water_max` 2e-4, `rh_crit_upper` 0.80 | Chosen on Earth targets only, before any slab run: tropical net CRE −41.0 (tied best with b_wm3 −41.2 and c_b01 −41.2, but those have cover 0.538 / 0.540 < 0.55), cover 0.577, global LW CRE +27.8 (closest to CERES +25.8), 40–65°S SW CRE −57.4 (closest to CERES −65.6), P4 rows 1–2 pass, shape row fails. The tropical net CRE excess is still ≈ 26 W m⁻²; no variant tried removes it. Not a default: the T42 re-tune is the owner's call. |
| 2026-11-23 | Low cloud after Slingo (1987) (new physics, `[clouds] low_cloud`, knobs `low_sigma`, `low_rh`, `low_rh_crit`, `low_omega`, `inv_pressure`, `inv_slope`, `inv_offset`) | (new) "rh" default, "slingo" option | Not tuned yet: owner decision on P4 question 13, option (iv). Reason: Earth target, not the warming — at T42 the boundary-layer stratiform cloud (σ ≥ 0.7) has a 20°S–20°N net CRE of −27.7 W m⁻², the whole excess against CERES EBAF Ed4.1 (−42.3 against −14.4); Slingo: the ω dependence "in the subtropics … suppresses the excessive cloudiness which would otherwise occur". Form and constants from Slingo (1987) §2c, eqs. (5)–(9). The column physics gains ω (σ-coordinate continuity, as in the dynamics). "rh" stays the default: byte-identical (10-day `aquaplanet_t21`, `ape_control_t21`). |
| 2026-11-23 | Low cloud "slingo_lts" (new option, knobs `lts_slope`, `lts_intercept`; `low_sigma`, `low_rh`, `low_rh_crit`, `low_omega`, `inv_pressure` shared with "slingo") | (new) published defaults 0.057, −0.5573 | Not tuned yet. Owner decision (vi)/(A): L26 does not resolve the inversion of Slingo eq. (8) (check of 2026-11-23); the bulk stability is resolved. Offline on the T42 Earth output (`tools/lts_check.py`): law alone 0.50–0.68 in the four main decks at peak season, 0.14–0.16 in the trades, 0.07 in the warm pool; estimated global cover 0.494 → 0.534. "rh" stays the default: byte-identical (10-day `aquaplanet_t21`, `ape_control_t21`, `earth_amip_t21`). |
| 2026-11-24 | Exploratory, T21 Earth AMIP 1 + 1 years, `low_cloud = "slingo_lts"` (owner step 4; no default changed) | — | Earth targets only; year 2; albedo / OLR / net TOA / global SW, LW CRE / cover / 20°S–20°N net CRE (CERES −14.6), cases `cases/bench/p4_lts/t21/`, runs `out/p4/lts/t21/`: (a) frozen 0.301 / 237.5 / +0.21 / −52.4, +23.6 / 0.534 / −51.6; (b) gate only (`low_rh` "sundqvist", `lts_slope` 0) 0.262 / 238.9 / +12.06 / −39.1, +22.2 / 0.457 / −29.0; (c) gate + law 0.305 / 237.5 / −0.81 / −53.6, +23.7 / 0.500 / −47.5; (d) as (c) with `rh_crit_surface` 0.90 / 0.85 / 0.80: −48.5 / −47.1 / −49.9 tropical, cover 0.497 / 0.497 / 0.500; as (c) with `water_max` 3e-4: 0.301 / 236.8 / +1.14 / −52.3, +24.2 / 0.501 / −45.2; `water_max` 2e-4: 0.292 / 237.2 / +3.70 / −49.3, +23.7 / 0.499 / −39.8; `lts_intercept` −0.65: 0.289 / 237.8 / +4.19 / −48.3, +23.3 / 0.485 / −39.9; `water_max` 2e-4 + `rh_crit_upper` 0.85: 0.303 / 235.2 / +1.85 / −53.2, +25.9 / 0.529 / −42.2. No variant reaches the tropical net CRE within 15 W m⁻² of CERES with criteria 1–2 and cover ≥ 0.5 (only (b), −29.0, which fails albedo, net TOA and cover): owner stop condition, nothing adopted. Shape row fails at T21 in all (as with the frozen knobs). Not chosen with any reference to the 2×CO₂ warming. |

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
| SBM shallow convection | `[convection] shallow` | "none" (options "shallower", "qref") | fixed (published options; off by owner decision 2026-11-22, SPEC.md §9.4.4 gap open) | Frierson (2007) §2c: "shallower" eqs. 7–9 (the paper's control), "qref" eqs. 10–15 |

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
| Convective cloud geometry | `convective` | "column" | "column", "anvil" | option (P4 q13 (b)) | "anvil" after Slingo (1987) §3c |
| Deep-convection core share of C_c ("anvil") | `core_share` | 0.25 | 0.1–0.5 | knob (P4) | Slingo (1987) §3c: "only 25% … the full depth" |
| Deep convection if the top is above σ ("anvil") | `deep_sigma` | 0.4 | — | fixed (published) | Slingo (1987) eq. (2): "above 400 mb" |
| σ at and below which the shallow remainder sits ("anvil") | `shallow_sigma` | 0.8 | 0.7–0.9 | knob (P4) | my reading of Slingo's "low-level"; his low/middle threshold 2 km |
| Anvil cover C_H = 2 (a + b ln P − 0.3) for a + b ln P > 0.4 | (code) | a 0.2454, b 0.1248 (P mm/day) | — | fixed (published) | Slingo (1987) eqs. (1)–(2); a, b fitted to his Table 1 |
| Anvil in-cloud water content ("anvil") | `anvil_water` | 3.5e-5 kg m⁻³ | 1e-5–1e-4 | knob (P4) | Slingo (1987) Table 3, anvil cirrus 0.035 g m⁻³ |
| Cap of the convective in-cloud water path ("anvil") | `conv_water_path` | 1.0 kg m⁻² | 0.05–2 | knob (P4) | not from a paper; initial value near the 400 ppm median of "column" (1.26 kg m⁻²) |
| Low-cloud treatment | `low_cloud` | "rh" | "rh", "slingo", "slingo_lts" | option (P4 q13 (iv)) | "slingo": Slingo (1987, QJRMS 113, 899–927) §2c eqs. (5)–(9), PDF supplied by the owner; equations checked on the page images; "slingo_lts": Slingo eqs. (5)–(7) + the stability stratus below (owner decision (vi)/(A), 2026-11-23) |
| σ at and below which eqs. (6)–(7) apply ("slingo") | `low_sigma` | 0.8 | 0.75–0.85 | knob (P4) | Slingo (1987) Fig. 1, low/middle boundary at about 800 mb (read from the figure) |
| RH law of the low cloud C_L' ("slingo") | `low_rh` | "slingo" | "slingo", "sundqvist" | option | "slingo": eq. (6) with RH_e = RH (1 − C_c), eq. (5); "sundqvist": this scheme's fraction with RH_c(σ) |
| Threshold RH of eq. (6) ("slingo", `low_rh = "slingo"`) | `low_rh_crit` | 0.8 | 0.7–0.9 | knob (P4) | Slingo (1987) eq. (6): 0.8, "somewhat arbitrary and may be model dependent" (p. 904) |
| ω of full RH low cloud, eq. (7) ("slingo") | `low_omega` | 0.1 Pa s⁻¹ | — | fixed (published) | Slingo (1987) eq. (7): zero for ω ≥ 0, linear to ω = −0.1 Pa s⁻¹ |
| Inversion search below ("slingo") | `inv_pressure` | 75000 Pa | — | fixed (published) | Slingo (1987) eq. (8): "the most stable layer below 750 mb" |
| Inversion cloud C' = −a Δθ/Δp − b, Δθ/Δp in K hPa⁻¹ ("slingo") | `inv_slope`, `inv_offset` | 6.67 hPa K⁻¹, 0.667 | 3.3–10, 0.33–1.0 | knob (P4) | Slingo (1987) eq. (8); "a different vertical resolution may need changes, particularly to … Eq. (8)" (p. 923) |
| Stratus from lower-tropospheric stability, strat = min(1, max(0, f_ocn (a·LTS + b))), LTS = θ(σ≈0.7) − θ_s, open ocean only ("slingo_lts") | `lts_slope`, `lts_intercept` | 0.057 K⁻¹, −0.5573 | 0.040–0.075 K⁻¹, −0.75 to −0.35 (my ranges: ±30 % of the slope, ±35 % of the intercept, about the scatter of published LTS regressions; not from a source); `lts_slope = 0` = gate only (diagnostic) | knob (P4); defaults published | Klein & Hartmann (1993, J. Climate 6, 1587–1606) as coded in CAM (`cloud_fraction.F90`, tag cam_cesm2_2_rel, fetched 2026-11-23) and quoted on the CESM forum; the 1993 paper and the CAM3 note NOT read. CAM's −0.125 K/hPa trigger omitted (owner decision (A), 2026-11-23): L26 does not resolve layer inversions |
| RH at the inversion base: none below 0.6, linear to full at 0.8 ("slingo") | (code) | 0.6, 0.8 | — | fixed (published) | Slingo (1987) eq. (9) |
