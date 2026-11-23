# Progress archive: Closed owner questions, with their rulings

Moved unchanged from `docs/progress.md` on 2026-11-23 (old line ranges 2593–2593, 2597–2617, 2622–2631, 2632–2702, 2769–2789, 2790–2826, 2827–2852, 2853–2949, 2950–2966); newest at the bottom. Summary in `docs/progress.md`.

1. ~~License~~ — resolved: MIT (LICENSE added).
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
   **Owner decision (2026-11-18): (a).** f32 accepted as recorded; f64 stays the default for
   acceptance. Resolved.
5. **Courant trigger of SPEC.md §9.8** (2026-11-15): "max Courant number above 0.8 in the log →
   reduce Δt". The logged number is SPEC §4.5's u Δt / (a cos φ Δλ), which near the poles
   grows like 1/cos φ; the accepted HS T42 run (Δt 20 min, stable for 2200 days) logs up to
   3.78. For a spectral model the explicit limit is set by the total wavenumber, not the
   grid spacing at the pole. Proposed replacement: monitor |v|_max Δt √(M(M+1)) / a (or the
   same with the grid spacing at the equator) and set the trigger on that. Until decided,
   the trigger is not applied to Part I cases.
   **Owner decision (2026-11-18): replace it.** SPEC.md §9.8 now uses |v|_max Δt √(M(M+1)) / a
   > 0.8; `tools/scorecard.py` reports it as `spectral_courant_max` from the diagnostics
   rows (P3 aquaplanet 0.58, P2 test world 0.50). Resolved.
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
   - **P3 retest (2026-11-18):** T42 semi-Lagrangian with the semi-gray radiation and the
     Earth ozone column, the top reflectance tuned to 0.165 (planetary albedo 0.256), years
     2–4 (`docs/scorecards/P1_aquaplanet_t42_semigray_2026-11-18.json`):
     - surface westerlies span 24°–46° (peak 8.5 m/s at 35°), easterlies from 49°, minimum
       −6.1 m/s at 60°. That is worse than gray (−3.69 m/s); the jet is at 32° (55 m/s)
       and the Hadley edge at 23.5°.
     - The other five rows pass (G 1.16, P = E to 0.03 %, energy closure 0.05 W m⁻²).
     - Radiation is therefore not the cause. Clear-sky semi-gray radiation sharpens the
       jet and keeps it equatorward. The row stays open for the P4 retest with clouds.
       Clouds are the last candidate named under "Likely candidates"; if they do not move
       the westerlies, the remaining option is (c).
   - **P4 note (2026-11-18):** the T21 slab aquaplanets with clouds have westerlies at every
     latitude 35–60° (minimum +4.1 m/s at 400 ppm). This is not the retest: the APE Control
     gate (T42, prescribed SST) has not been run with clouds. It is step 6 of the P4
     completion plan. The row stays open.
   - **Retest after P4 (2026-11-21):** APE Control T42 with the frozen P4 clouds
     (`cases/bench/p4_retest/ape_control_t42_clouds.toml`, 1 + 3 years) fails the same row.
     - Surface westerlies peak at 9.8 m/s at ±37.7° and end at 51°; easterlies poleward of
       that, −4.95 m/s at 62.8°.
     - The other five rows pass.
     - Clouds do not change it; still open.
   - **Owner decision (2026-11-23): closed as a known limitation** (question 14).
8. **P2 acceptance 5: G and seasonal sea ice in the test world** (2026-11-17).
   - **G = 2.18 against ≤ 2.0:** one dry ocean cell (0.8 mm/day against ~0.37 in its
     neighbours) at 18°N, off a desert coast. Every other cell is ≤ 1.76.
     - The criterion is a ratio, so it trips in near-dry regions without any storm.
     - Options:
       - (a) keep it, and retest after P3/P4 changes the subtropics;
       - (b) the owner adds a precipitation floor to G (e.g. only cells with neighbour
         mean > 1 mm/day).
   - **"Sea ice only where the annual-mean `tos` is at freezing":** seasonal ice breaks
     this wording, with 589 cells failing. Monthly, ice coexists with water above
     freezing only in the months it forms or melts (1.3 % of cell-months, ≤ 273.8 K).
     - Options:
       - (a) keep the wording;
       - (b) the owner restates it per month, or for perennial ice (annual ice ≥ 0.95,
         which passes).
   - I have not changed either criterion.
   - **Owner decision (2026-11-18): restate both.** SPEC.md §9.5 G now counts only cells
     whose neighbour mean is ≥ 1 mm/day; P2 acceptance 5 now reads "perennial sea ice
     (annual-mean concentration ≥ 0.95) only where the annual-mean `tos` is within 0.1 K of
     freezing". Re-evaluated on the same run: G 1.71 (4.2°N 149°E), perennial ice in 2804
     cells with `tos` ≤ 271.369 K (T_f 271.35 K). **P2 acceptance 5 met.** Resolved.
9. **P3 acceptance 1 and 2: spec ranges against the RRTMG reference** (2026-11-18).
   - **Test 4** (clear-sky OLR at 288 K, RH 75 %, 400 ppm: 260–275 W m⁻²) and **test 6**
     (Sun, global mean: planetary albedo 0.12–0.19):
     - RRTMG itself, on exactly these columns, gives 251.1 W m⁻² and 0.107. Both are
       outside the ranges, so a scheme that matched RRTMG perfectly would fail.
     - toyclim gives 246.2 W m⁻² (slope 1.72 ✓) and 0.116 (absorption 0.196 ✓).
     - The tests are in the code as ignored tests: `clear_sky_olr_spec_range`,
       `clear_sky_albedo_spec_range`.
     - Probable cause: SPEC.md §9.4.1 was written from memory (CLAUDE.md). The Koll &
       Cronin (2018) values it cites could not be fetched.
   - **Acceptance 2** ("within 8 W m⁻² and 10 %"):
     - met as rms at 0.3–1 bar: OLR 2.6–4.9, LW↓ 5.1–9.0 W m⁻², SW absorption 7–8 %;
     - not met as maxima: OLR 6.2–11.1, LW↓ 15.2–17.7 W m⁻², SW 30 %;
     - the worst columns sit at the edges of the set (310 K, 10000 ppm, 0.3 bar, very low
       sun);
     - at 2–3 bar the RRTMG reference is extrapolated beyond its tables (OLR max 23–37).
   - **Options:**
     - (a) keep the criteria; P3 stays open on 1 and 2, and more terms are added to the
       semi-gray scheme;
     - (b) the owner restates test 4 as "within 8 W m⁻² of RRTMG" and test 6 as "within
       0.015 of RRTMG" (both pass now), and acceptance 2 as rms over the set, with maxima
       recorded (passes at 0.3–1 bar);
     - (c) as (b), with the p_s range limited to where RRTMG is valid (≤ 1 bar fitted, 2–3
       bar recorded).
   - **Recommendation:** (c). The semi-gray scheme is a calibrated approximation. Its
     worst-case error at the edges of the envelope is the known price of three bands.
   - **Deviations from §9.4.1, for the owner's record:**
     - PIFM in place of δ-Eddington: RRTMG's default; it fits RRTMG at low sun;
     - pressure scaling on all terms, a CO₂ pressure exponent and curvature, the "rest"
       band split into weak and strong, and a dry near-IR absorber;
     - the gray scheme keeps its own solver;
     - `every > 1` is not implemented: the cost is 21 % of a step with `every = 1`.
   - I have not changed any criterion.
   - **Owner decision (2026-11-18): (b)/(c), compare to RRTMG.** SPEC.md §9.4.1 tests 4 and
     6 are now relative to RRTMG (8 W m⁻², 0.015), and P3 acceptance 2 is an rms over
     0.3–1 bar with maxima and 2–3 bar recorded. The two ignored spec-range tests are
     removed; the RRTMG-relative tests pass. **P3 acceptance 1 and 2 met.** Resolved.
10. **P3 acceptance 4: the aquaplanet is numerically stable but drifts to a snowball**
    (2026-11-18).
    - **Results:**
      - closure 0.13 W m⁻² and G 1.38 pass;
      - with the warm-start-balanced stand-in (ocean albedo 0.25), global T_s falls
        277 → 268 K between years 10 and 20, at −0.8 to −1.0 K/yr, with the ice edge
        moving from 41.5° to 30–36°;
      - with 0.38 the planet froze over in 10 years.
    - **Options:**
      - (a) "stable" means numerically stable (no blow-up, closures hold): acceptance 4
        passes;
      - (b) "stable" means a steady climate: acceptance 4 stays open, and is retested in
        P4 with real clouds replacing the albedo stand-in. Clouds over ice add little
        albedo but a longwave greenhouse effect, which the stand-in cannot represent;
      - (c) as (b), adding the test world's ocean heat transport (50 W m⁻²) to the
        aquaplanet case now.
    - **Recommendation:** (b). It is the same ice-edge sensitivity as question 7, and the
      stand-in is the weak part of the configuration.
    - I have not changed the criterion.
    - **Owner decision (2026-11-18): (b).** "Stable" means a steady climate; acceptance 4
      stays open and is retested in P4 with clouds.
    - **P4 retest (2026-11-18):** the T21 slab aquaplanet with clouds holds a steady climate for
      30 years: T_s 286.5 K, 20-year trend −0.004 K/yr, net TOA −0.17 W m⁻², closure
      0.10 W m⁻², G 1.21. **Proposal: the owner closes this question.** It rests on PR #8's
      clouds, so P3 alone does not show it. The formal §9.5.2 trend test is question 11.
    - **Owner decision (2026-11-23): closed.**
11. **P4 acceptance 4: 2×CO₂ warming 6.87 K, and the equilibrium trend test** (2026-11-18).
    - **Evidence:** the entry "P4 first full evaluation".
    - **Warming:** 6.87 K against the 1.5–6 K range. Sea ice is the largest amplifier, but the
      tropics also warm 5.9 K. The cause is not separated yet.
    - **Trend test:** both runs fail 0.02 K/yr over 5 years (−0.043 / −0.049 K/yr), probably
      because the test uses single-moment yearly values. Annual means may still miss the
      limit if interannual variability is large; that will be measured.
    - **Plan:** SPEC.md §9.6, P4 completion plan, steps 1–3. I have not changed either
      criterion and will not pick knob values to bring the warming into range.
    - **Owner decision needed only if** the warming is still above 6 K after step 3, or the
      trend test still fails with annual means. P5 may start in the first case; P6 waits for
      the ruling.
    - **Owner decision (2026-11-20, before the overnight session): "Proceed if cause found".**
      If the warming is still above 6 K after step 3 and no defect is found, P6 may start
      (when P5 allows it) provided the excess has a documented physical cause and no fault
      is found in the scheme. The row stays open and is listed as a known limitation; the
      criterion is unchanged. Recheck the warming once the P6 ocean exists.
    - **Steps 2–3 result (2026-11-21), old cloud knobs** (entry "P4 completion plan, steps 1–5"):
      - sea ice off (`cases/bench/p4_aquaplanet/noice_t21_{400,800}.toml`, 12 years,
        climatology years 9–12): 289.86 → 294.07 K, **+4.21 K**. With sea ice: +6.84 K
        (years 31–35). Sea ice adds about 2.6 K (ice cover 0.131 → 0.021).
      - Global clear-sky instantaneous 2×CO₂ forcing on 13 monthly states of the 400 ppm run
        (6656 columns, `tools/co2_forcing.py`): semi-gray **3.78**, RRTMG **3.45 W m⁻²**
        (+9 %); by band 0–30° 5.06 / 4.08 (+24 %), 30–60° 3.00 / 3.21, 60–90° 1.58 / 1.93.
        At the model's sensitivity the excess is worth roughly 0.6 K.
      - The sea-ice code matches SPEC.md §9.4.5 (Semtner zero-layer, CAM6 constants); no
        defect found. Documented cause: the ice–albedo feedback of a thermodynamic-only ice
        cover on a 20 m slab without ocean heat transport.
      - Not changed: the semi-gray CO₂ term. Its tropical forcing is 24 % too strong and the
        polar one 18 % too weak; the P3 calibration fits OLR to 8 W m⁻² rms and the forcing
        differences only with weight 1 W m⁻². A refit is a radiation change that would
        reopen P3 and the cloud tuning. **Owner:** refit the CO₂ term now (I would weight
        the forcing pairs more heavily and accept a slightly larger OLR rms), or leave it as
        a known limitation.
    - **Gate with the frozen knobs (2026-11-21):** warming **11.06 K** (800 ppm at years 51–55),
      implied equilibrium 11.7 K; total feedback −0.32 W m⁻² K⁻¹ (old knobs −0.50).
      - The extra warming is the tropical SW cloud term (+0.81 against +0.42), from the
        convective cloud diagnosis (question 13).
      - Sea ice now melts completely in each pair. Clear-sky SW is +0.9 W m⁻² K⁻¹, about
        +9.8 W m⁻² in total; it acts as a fixed push, so the 2.6 K from the old knobs does
        not transfer.
      - Diagnostic 1 (RRTMG on the 400 / 800 ppm states): the clear-sky response to warming
        agrees with RRTMG within 3 % (−1.51 against −1.56 W m⁻² K⁻¹). The 2×CO₂ forcing does
        not: semi-gray 3.85 → 5.99 W m⁻² from the cold to the warm state, RRTMG 3.57 → 4.66.
        The refit should weight the forcing at warm states (300–315 K) and its temperature
        dependence. Rough scaling: about 16 % less warming with RRTMG-like forcing.
      - With the larger excess, I treat the ruling "proceed if cause found" as not yet
        applicable. The cause is located but it is a scheme property, so it looks fixable.
        P5 is not started.
      - **Recommendation:** refit the semi-gray CO₂ term first, with forcing pairs over a range
        of states including 300–315 K surfaces weighted. It reopens the cloud tuning anyway.
    - **Owner decision (2026-11-21): refit the CO₂ term, first.** Fit forcing pairs across
      the whole state range, including 300–315 K surfaces, weighted so that the temperature
      dependence of the forcing matches RRTMG. Guard: the clear-sky response to warming
      (now within 3 % of RRTMG) must not degrade; report it before and after, with the P3
      tests. Expected effect about −16 %, so this alone will not pass criterion 4. If the
      warming is still above 6 K after questions 11 and 13 are done, with complete ice melt
      as the documented cause, the ruling above covers it. P5 stays unstarted until
      question 13 is settled (P5 must score frozen clouds).
    - **Refit result (2026-11-21 evening):** not adopted. Weighting the forcing pairs ×4
      fixes the warm-state forcing (4.90 against RRTMG 4.66; now 5.99) and keeps the
      clear-sky response at RRTMG's (−1.56), but breaks P3 criterion 2 (LW↓ 8.63 W m⁻² rms
      > 8). ×2 keeps criterion 2 but leaves the warm-state forcing at 5.64. The present CO₂
      term cannot do both. **Owner:** a structural change to the CO₂ term (water-vapour
      overlap), or accept the forcing error as a known limitation?
    - **Owner decision (2026-11-21, adopting a review):** check which term and whether
      overlap is the cause; if structural, a CO₂ band from a published form, adopt only if
      P3 tests and criterion 2 pass, state forcing within about 10 % and the clear-sky
      response within a few % of RRTMG; time box half a day, else accept the limitation.
    - **Result (2026-11-22; entry "Question 11: structural check and the CO₂ band"):** the
      cause is not overlap but stratospheric temperature: the P3 term's ln(CO₂/360)
      opacity makes the forcing 2.5× too sensitive to it, and the calibration set (fixed
      200 K stratosphere) never constrained it. The Jeevanjee et al. band, fitted with
      model-state columns, gets the state forcing within 7 % but no fit also keeps the
      clear-sky response (best −1.39 / −1.44 against −1.54) and criterion 2 (band5 LW↓
      9.57). Time box reached; nothing adopted; defaults unchanged.
    - **Owner, choose one:**
      - (i) accept the limitation (the rule as given): forcing +8 % cold / +29 % warm,
        clear-sky response −1.51;
      - (ii) adopt band3 with its clear-sky response 10 % weak (−1.39), i.e. relax the
        response guard to "within 10 %" — that is your criterion to change, not mine;
      - (iii) more time on the band: add calibration columns with a range of stratospheric
        temperatures (the blind spot) and refit, which may hold all three.
    - **Recommendation:** (iii), capped at another half day. The band is clearly the
      better physics for the state dependence (+33 % against RRTMG's +30 %, old term +56 %),
      and the remaining conflict appeared only after the state columns entered the fit.
    - **Owner decision (2026-11-22): (iii) with additions** (stratosphere varied in the
      calibration set, two-stage fit, SCM stratospheric check; adoption: P3 tests and
      criterion 2, forcing within 10 % on all four states, response within 5 %; if it fails
      again accept the limitation naming both the forcing error and the stratospheric
      cooling; not (ii)).
    - **Result (2026-11-22 afternoon):** two-stage fit ts1: forcing within 5.4 % on all four
      states, but criterion 2 fails (LW↓ rms 10.0, 0.3 bar 15.0) and the response is +5.2 %
      (−1.622 against −1.541). Not adopted. **Limitation accepted** (wording in the entry
      "Owner decisions on questions 11 and 13 carried out"): forcing +8 % cold / +29 % warm;
      stratospheric 2×CO₂ cooling SCM −7.5 K at 14 hPa against RRTMG −1.7 K, 3-D −31 K.
      **Settled.**
12. **Equilibrium test: the 5-year T_s trend is below the noise of a 20 m slab with sea ice**
    (2026-11-21; SPEC.md §9.5.2, P4 acceptance 4).
    - **Evidence:** daily diagnostics, annual means, years 31–35 of the P4 slab pair (old
      cloud knobs): trend +0.042 / +0.052 K/yr (limit 0.02); TOA − closure residual
      +0.015 / +0.107 W m⁻² (limit 1). Interannual standard deviation of annual-mean T_s
      0.080 / 0.094 K, so the standard error of a 5-point trend is 0.025 / 0.030 K/yr: the
      test fails about half the time in a perfectly equilibrated run. The 20-year trends of
      the same runs were −0.004 / +0.004 K/yr.
    - **Not done:** searching for a 5-year window that passes.
    - **Proposed replacement (owner to decide):** the trend over the last 10 years of annual
      means (standard error ≈ 0.009 K/yr) below 0.02 K/yr, or the 5-year trend below
      max(0.02, 2 × its standard error), keeping the TOA condition unchanged.
    - **Owner decision (2026-11-21): replace with the 10-year version, flux condition
      tightened.** Trend of the last 10 annual means below 0.02 K/yr, and the 10-year mean
      net TOA within ±0.3 W m⁻² of the closure residual (not "max(0.02, 2 × standard
      error)", which loosens as noise grows). Record the implied equilibrium warming next to
      the measured one. SPEC.md §9.5.2 updated; `tools/equilibrium.py` to follow.
