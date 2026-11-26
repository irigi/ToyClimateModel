# Progress log

## READ THIS FIRST — plan revised 2026-11-12, file compressed 2026-11-23

**The plan is `SPEC.md`.** Part I (M0–M6) is accepted; Part II (§9) is milestones **P0–P11**.
Current work: **the P4 completion plan** (SPEC.md §9.6 P4; "P4 state" and question 13 below).
PRs #5–#7 (P1–P3) are proposed for merging; #8 (P4) stays a draft. P0–P9 are authorised;
P10 and P11 need a go-ahead. The "M7–M11 first steps" are a **prototype** (SPEC.md §9.1),
not accepted results. `scripts/check` runs fmt + clippy + all tests (≈ 1 min in release).
This file: status, milestone summaries, P4 state, the latest entries, open questions; older
entries are archived unchanged in [docs/progress/](progress/) (open only where a summary points).

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
| P2 Land, snow, sea ice, precipitation phase | 2026-11-18 (PR #6): acceptance 1–5 and 7 met (5 after the owner restated G and the sea-ice rule, question 8: G 1.71, perennial ice ≤ 271.37 K); 6 (bistability) open, retest after P4 (question 7); defect 1 diagnosed |
| P3 Radiation (multi-band two-stream, semi-gray) | 2026-11-18 (PR #7): acceptance 1, 2, 3, 5 met (1 and 2 as restated by the owner against RRTMG, question 9); 4: numerically stable, closure and G met, but drifts to a snowball, retest in P4 with clouds (question 10). Earth T21: subtropical jets form, 14 of 19 P5 rows would pass |
| P4 Clouds, Earth-like energy balance | 2026-11-22 (PR #8, draft): gate with frozen knobs: 1, 2 and 5 met; 3: shape met, cover 0.497; 4: 2×CO₂ +11.1 K. Question 11 settled: CO₂ band refits failed the adoption conditions, the semi-gray CO₂ forcing error (+8 %/+29 %) and excess stratospheric cooling are an accepted limitation. Question 13: shallow convection off by default (regress identical); running-mean cloud (a) failed offline; both Frierson shallow treatments reproduce the paper in its slab setup but collapse the APE Hadley cell (−70 %) with prescribed SST. Question 13 (b), 2026-11-22/23: Slingo (1987) anvil option implemented (off by default); T21 Earth tropical net CRE −41 at best against CERES −14.6; slab pair with it ΔT 9.6 K but tropical SW cloud feedback larger (+1.08 against +0.81); review round 2026-11-23: Earth tropical excess = boundary-layer stratiform cloud (−27.7 of 27.9 W m⁻²); fixed-SST factorial reproduces 86 % of the slab loss (SST 79 %, CO₂ 16–21 %); owner chose Slingo (1987) low cloud (2026-11-23): implemented, off by default; separating run: the slab SST pattern carries 85 % of the SST-driven loss; **stopped at owner step 3**: L26 does not resolve the trade inversion that Slingo's eq. (8) needs (question 13). Heat-transport pair: 400 ppm 306.5 K ice-free, not equilibrated (trend −0.037 K/yr); 800 ppm failed the humidity envelope at 311 K; no warming estimate. Question 6 closed as a limitation; 7 to be rerun before P6. 2026-11-23/24: owner chose (vi)/(A), "slingo_lts" implemented (off by default); **stopped at step 4**: best tropical net CRE with criteria 1–2 and cover ≥ 0.5 is −39.8 to −42.2 against CERES −14.6 (question 13); 2026-11-24, step 4b: r6 (`water_max` 1e-4, `rh_crit_upper` 0.75, case files only): T42 gate 1, 2, 5 and cover 0.567 met, **shape row fails** (no northern storm-track maximum); 400 ppm slab **297.1 K** (ice-free), 2×CO₂ +3.53 K (800 ppm not equilibrated); **stopped at step 7** (question 13). Q7 rerun: cold start stays ice-covered, both branches hold; Q7 closed by the owner (2026-11-24). Screen after step 7 (2026-11-25): r1, r3, r5 all fail (r3 passes T42 but its 400 ppm slab melts to 295.6 K; at fixed SST the new low cloud adds +14.8 W m⁻² net CRE, mostly tropical SW); **stopped at step D** (question 13). Owner (2026-11-25): r3 candidate, the 283–293 K closing condition withdrawn (row 4 open, revisit with P6); r3 gate: T42 1, 2, 5 and cover 0.545 met, **shape row fails** (northern maximum on the 68.4° edge row by 0.005); slab 400 ppm 295.62 K, 2×CO₂ +2.96 K (both equilibrated); regress no regression; **stopped at step 1** (question 13). Owner (2026-11-25 evening): EIS law (Wood & Bretherton 2006) in P4, step A first; 2026-11-26: (a) the cover deficit is mid-latitude (−0.14 to −0.17 at 32–57°N, −0.24 to −0.30 over the Southern Ocean), the northern cap is below CERES too; **(b) fails**: EIS raises 68–80°N (+0.013) more than 45–65°N (+0.006); **stopped at step A** (question 13). Owner (2026-11-26): diagnosis. GOCCP: the deficit is low cloud; freeze-dry corrected to CAM (RH cloud only, p ≥ 750 hPa); T21 toggles (freeze-dry, gate, RH_c) bring 32–57°N at best to −0.123 against CERES (target within 0.08): **stopped at step 3** (question 13) Owner (2026-11-26 afternoon): **plan change, the P5 convection item is done inside P4; P5 starts after P4 closes with the final convection and clouds**. Boundary-layer humidity diagnosis: near-surface ocean RH 0.05–0.15 below ICOADS (winter 40–60° 0.67–0.70 against 0.80–0.82), winter evaporation +33/+62 %, global P 2.92 (T21) / 3.20 (T42) against GPCP 2.69; RH pinned at `rh_ref` through frequent mid-latitude convection and the mixing; only `rh_ref` 0.8 moves RH and cloud (32–57°N −0.091, but criterion 2 fails); **stopped for the owner** (question 13) Owner (2026-11-26 evening): screen, trigger T (SPEEDY) built and off; s1–s6 at T21, **no variant meets the bar** (winter RH met only by s6 with `ri_crit` 0.25, 0.782 / 0.797, but P correlation 0.627 against ≥ 0.694); **stopped at step 4** (question 13) Owner (2026-11-26 late evening): resolution / mixing-depth diagnosis; near-surface RH set by `ri_crit`, not by L30/L32; low cloud moved by neither (scheme thresholds); L32 tropical net CRE −26.8; **stopped and reported** (question 13) |
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

- Python tools: `netCDF4` and `xarray` (owner approval 2026-11-17), to read the netCDF
  reference data of §9.5.3 for the `earth_amip` scorecard. Installed in the git-ignored
  `.venv/` (`python3 -m venv .venv && .venv/bin/pip install -r tools/requirements.txt`).
  The Rust crates are unaffected.
- Radiation calibration (P3): RRTMG through `climlab` 0.9.2 + `climlab-rrtmg` (MIT), which
  exist only on conda-forge (owner approval 2026-11-17: micromamba environment). micromamba
  2.10.0 in `~/.local/bin`, environment in the git-ignored `.rrtmg/`:
  `micromamba create -p ./.rrtmg -c conda-forge python=3.12 climlab climlab-rrtmg numpy scipy
  matplotlib-base cycler pillow kiwisolver fonttools pyparsing python-dateutil contourpy`.
  Used only by `tools/calibrate_radiation.py`; the model does not depend on it.

---

## Milestone summaries

Accepted milestones, with the rows the owner sent to a retest; numbers copied from the archive.

### M0–M6 (Part I) and the prototype
- M0: CI config present, 13 tests pass (status: CI to be confirmed on first push). M1 tests 1–6
  met: orthonormality T170 ≤ 2.4e-14, round trip ≤ 7.6e-14, wind 3.3e-14, solid body 2.3e-16.
- M2 met: W2 T42 l2 1.1e-9 (< 1e-5), mass 7e-16; W5 T85 energy 3.1e-7; W6 balance 9.6e-14,
  enstrophy drift 2.0e-3, phase 11.28 °/day vs 12.20 (−7.5%); Galewsky 8.1e-3; 14.0 model days/s.
- M3 met: headless exit 0, 9210 steps = 127.9 model days. M4 met: rest max|u| = 0; gravity waves
  ≤ 1.5e-4 off; mass −9.8e-16; JW06 unperturbed 0.141 m/s, day-9 min p_s 940.8 hPa (recorded).
- M5 met (T42 L20 1200 d): jets 33.72 / 32.20 m/s at ±43.3°, σ 0.225; surface −5.66 / +4.31
  m/s; asymmetry 2.13 m/s (< 3); KE trend −1.52 %. Tidally locked and rotation sweeps recorded.
- M6 met: bitwise across threads/restart; 289.6 s per model year on 8 threads (≤ 10 min); f32
  1.31×, asymmetry 3.19 m/s > 3: question 3, owner (2026-11-18) chose (a), accepted as recorded.
- Deviations: W2 with rotated Coriolis (paper wins); JW06 top interface set to 0; global
  dry-mass fixer on by default.
- M7–M11 "first steps" (moisture, gray radiation, slab/land, clouds, tools) are a prototype,
  not accepted work (SPEC §9.1). Laptop-checklist item 5 (HS vs SpeedyWeather/Isca) never done.
- Archive: [docs/progress/M0-M6-prototype.md](progress/M0-M6-prototype.md)

### P0 Groundwork
- Regress: PASS, all 6 cases × {1, 8} threads byte-identical vs pre-P0 0f7d33a (met).
- Derivations met: Earth S = 1361 W m⁻², year 365.256 d (to 1e-4); TRAPPIST-1e (Agol et al.
  2021) year 6.098 d, S = 880 W m⁻² (within 5% of 900); inconsistent overrides rejected.
- Files < 700 lines: met, largest core file 478 lines.
- f32 item (question 3): 2200 d, asymmetry f64 1.52 / 1.79, f32 4.24 / 0.87 m/s; accepted as
  recorded (2026-11-18). Laptop item 2 (W6, Galewsky constants) done, all agree with the code.
- Deviations: typed physics blocks lower to the prototype bundle (inverted in P1); envelope
  enforced by warnings, not refusal; allocations inside prototype schemes left until P1;
  tidally locked case keeps S = 900 W m⁻² with a star L 2% above Agol et al. 2021.
- New question 5 (Courant trigger): owner replaced it (2026-11-18) with |v|_max Δt √(M(M+1)) / a.
- Archive: [docs/progress/P0.md](progress/P0.md)

### P1 Surface exchange, boundary layer, sequential physics, SCM
- 1 SCM conservation: max energy residual 4e-9 W m⁻² (limit 1e-6), water within 1e-9: met.
- 2 stiff land: no 2Δt oscillation over 10 days: met. 3 diurnal: dry land 8.6 K (8–25),
  stable night; 50 m ocean 0.73 K (< 1): met. 4 neutral C_D: 1.44e-3 (1.0e-3–1.6e-3): met.
- 5 APE Control T42 (semi-Lagrangian): easterlies, peak westerlies 5–12 m s⁻¹, P = E (8e-5),
  closure 0.006 W/m², G 1.19 met; westerlies at 35–60° open: −3.69 m/s at 60°.
  T63 −2.55 m/s; P3 retest −6.1 m/s; after P4 (2026-11-21) −4.95 m/s at 62.8°. Question 6
  closed by the owner on 2026-11-23 as a known limitation (question 14).
- 6 HS T21 regression identical: met. 7 viewer advances 1856 steps: met.
- §9.8 tracer-fixer trigger fired (2.29 %/day at T42); fixed by semi-Lagrangian humidity
  transport (0.12 %/day).
- Limitation: eddy-driven circulation 10–15° equatorward of APE models (jet near 32°).
- Deviations: APE cases use tau_sw = 0 (FHZ06); `psl` reduction unverified (written from
  memory); SBM shallow convection not implemented; hyperdiffusion energy fixer on.
- Archive: [docs/progress/P1.md](progress/P1.md)

### P2 Land, snow, sea ice, precipitation phase
- 1 SCM snow/sea-ice conservation: met (status table); numbers not recorded in the archive.
- 2 water budget 7.3e-4 mm/day (≤ 1e-3): met. 3 energy closure 0.11 W/m² (≤ 0.5): met.
- 4 bitwise restart and thread independence: met (`land_world_runs_and_restarts`).
- 5 (restated by the owner, question 8): G 1.71 (≤ 2.0, cells with neighbour mean ≥ 1 mm/day;
  was 2.18); 0 EF cells over ice-free ocean; perennial ice in 2804 cells, `tos` ≤ 271.369 K
  (T_f 271.35 K): met.
- 6 bistability: open, one climate (ice start deglaciates, ice edge 41.5°; no snowball down to
  92 % within 6 years); question 7, retest after P4 (status table: rerun before P6).
- 7 Earth scorecard recorded: 11 of 19 P5 rows would pass at T21 and T42.
- Defect 1: storms vanish only with P1 physics, pointing to (c), sequential physics.
- Limitations: excess sea ice poleward of ~45° (EF 30 %); too-wet subtropics, no subtropical
  jets under gray; NH February snow 30.8 vs 46.3; weaker defect 1 over the Amazon at T42 (G 2.53).
- Archive: [docs/progress/P2.md](progress/P2.md)

### P3 Radiation
- 1 offline tests 1–7: met as restated by the owner (question 9): tests 4 and 6 relative to
  RRTMG (OLR 246.2 vs 251.1 W m⁻², albedo 0.116 vs 0.107).
- 2 calibration: met as restated (question 9: rms over 0.3–1 bar): OLR 2.6–4.9, LW↓ 5.1–9.0,
  SW 7–8 %; maxima recorded (OLR 6.2–11.1, LW↓ 15–18, SW 30 %), 2–3 bar OLR max 23–37.
- 3 SCM RCE 285.20 K (283–293); 2×CO₂ ΔT = +3.95 K (1.5–4.5): met. 5 radiation cost ≈ 21 %
  of a T42 L26 step (< 40 %): met.
- 4 T21 aquaplanet: closure 0.13 W m⁻², G 1.38, numerically stable 20 years, but cools to a
  snowball (275 → 268 K); question 10, owner ruled "stable" = steady climate; P4 retest with
  clouds 286.5 K, trend −0.004 K/yr; closed by the owner on 2026-11-23.
- Earth T21 semigray: subtropical jets form, 14 of 19 P5 rows would pass. Limitations: water
  closure 2.0e-3 mm/day (new defect); monsoon row 1.0 (≥ 3);
  2×CO₂ forcing 4.91 vs 4.12 W m⁻² (later accepted, question 11: +8 % cold / +29 % warm).
- Deviations: PIFM not δ-Eddington; extra semi-gray terms; `every > 1` not implemented; worlds
  gray; RRTMG as reference (Koll & Cronin 2018 inaccessible).
- Archive: [docs/progress/P3.md](progress/P3.md)

## P4 state (as of 2026-11-26 afternoon)

**Plan change (owner, 2026-11-26 afternoon; SPEC.md not edited, criteria unchanged):** the convection
work item that was to be the first item of P5 is done now, inside P4, because the clouds cannot be
frozen while the humidity field they depend on is wrong. P5 starts only after P4 closes with the
final convection and clouds. Working cloud configuration until then: r3 with the corrected
freeze-dry; no more cloud tuning until the humidity question is answered.

**Frozen knobs** (2026-11-21, `docs/tuning.md`, gate binary `out/p4/toyclim_frozen`; **the gate numbers below
predate the 2026-11-26 freeze-dry correction and are not rerun**, owner 2026-11-26): freeze-dry on,
RH_c 0.95 / 0.90, β 0.03 (freeze-dry corrected to CAM on 2026-11-26: RH cloud only, p ≥ 750 hPa; changes only high ground with these knobs, gate not rerun); shallow convection, "anvil", `deep_requires_pt` and the CO₂ band off.
"slingo_lts" low cloud (decision (vi)/(A), 2026-11-23): implemented, off; in step 4 (question 13).

| # | Criterion (T42 Earth AMIP 1 + 5 years; 4: T21 slab aquaplanet) | Measured, frozen knobs | State |
|---|---|---|---|
| 1 | Albedo 0.27–0.33; OLR 228–250; net TOA ±8 | 0.280; 242.5 W m⁻²; +2.53 W m⁻² | met |
| 2 | SW CRE −30 to −60; LW CRE +15 to +35 | −44.6; +19.9 W m⁻² | met |
| 3 | Cover 0.5–0.75; ITCZ and storm-track maxima, subtropical minima | **0.497**; shape met | **not met** (cover, by 0.003) |
| 4 | 400 ppm T_s 283–293 K; 2×CO₂ warming 1.5–6 K | 288.376 … 288.516 K (**equilibrated**); **2×CO₂ warming 11.06 K** | warming open; P4 closes with it open (owner, 2026-11-23) |
| 5 | Energy closure ≤ 0.5 W m⁻² | 0.31 | met |

**r6 gate (2026-11-24, `low_cloud = "slingo_lts"`, `water_max` 1e-4, `rh_crit_upper` 0.75; case files
only, not adopted):** 1: 0.296; 237.1; +2.61 met. 2: −50.2; +25.8 met. 3: cover 0.567, **shape fails**
(northern cover rises to 0.71 at 79.5°, no storm-track maximum). 4: 400 ppm **297.1 K** (ice-free,
equilibrated), outside 283–293; 2×CO₂ +3.53 K (800 ppm still cooling). 5: 0.30 met. Stopped at step 7.

**Screen after step 7 (2026-11-25; r1, r3, r5; case files only):** T42 1 + 1 years: r3 alone meets
1, 2, cover (0.548) and the shape row (band error 10.19); r1 and r5 fail the shape row. 400 ppm slab,
years 46–60: all three lose their sea ice and end at 295.6–297.4 K. Stopped at step D; proposals in
question 13. At fixed SST the "slingo_lts" scheme itself adds +14.8 W m⁻² of net CRE (SW +10.0,
mostly 40°S–40°N; polar caps +3.5 to +4.5), so the polar upper cloud is the smaller part.

**r3 gate (2026-11-25 afternoon; `slingo_lts`, `water_max` 1.5e-4, `rh_crit_upper` 0.80; case files only):** 1: 0.301; 238.4; −0.56 met. 2: −52.0; +24.2 met. 3: cover 0.545 met, **shape fails** (northern 40–70° maximum 0.668 on the 68.4° edge row, 0.005 above 65.6°; plateau 0.65–0.67 to 80°N; local in 2 of 5 single years). 5: 0.30 met. Band error 10.09; 20°S–20°N net CRE −41.4. Slab: 400 ppm 295.62 K, 800 ppm 298.57 K (both equilibrated, ice-free), 2×CO₂ +2.96 K. Owner (2026-11-25): row 4's 283–293 K is no longer a closing condition (stays open). Stopped at step 1.

**Findings that still drive decisions** (details in [P4.md](progress/P4.md) and the two entries below):
- The Earth tropical cloud excess is boundary-layer stratiform cloud (−27.7 of 27.9 W m⁻²); it is
  hidden in the global means by too little cloud elsewhere.
- The tropical 2×CO₂ cloud loss is an SST response: S800 C800 reproduces 86 % of the slab loss;
  SST change 79 %, CO₂ alone 16–21 %; the slab SST pattern carries 85 % of the SST-driven loss.
- L26 does not resolve boundary-layer inversions; the bulk stability θ700 − θ_s is resolved.
- Semi-gray CO₂ forcing +8 % cold / +29 % warm (accepted); dOLR/dT_s 1.68 against RRTMG's 1.92.
- Slab pairs run ≥ 30 years after any scheme change (question 12).

**Tried and rejected** (one line each):
- CO₂ band refit — **No refit adopted**: the only candidate near RRTMG broke P3 criterion 2
  (LW↓ 8.63 > 8); limitation accepted ([P4.md](progress/P4.md), "Question 11: structural check and
  the CO₂ band"; [closed-questions.md](progress/closed-questions.md), 11).
- Running-mean convective cloud (a) — a 24 h mean keeps two thirds of the fall: **(a) fails; not
  built** ([P4.md](progress/P4.md), "Owner decisions on questions 11 and 13 carried out", step 2).
- Shallow convection ("shallower", "qref") — Hadley −27 % against −26 % on a slab, but 70 % with
  prescribed SST; off, P5 item ([P4.md](progress/P4.md), same entry, steps 4–6).
- Slingo anvil scheme ("anvil") — T21 Earth tropical net CRE **−41.0** at best; slab ΔT **9.61 K**
  while the tropical SW cloud term grows, from +0.81 to +1.08; its 400 ppm aquaplanet 295.1 K is outside
  283–293 K; not adopted ([P4.md](progress/P4.md), "Question 13 option (b)" entries).
- Slingo (1987) eq. (8) inversion cloud — L26 does not resolve the trade inversion; eq. (8) gives
  0.09–0.19 in the decks at T42 (entry "Owner steps 1–3" below; code kept, off).
- CAM's −0.125 K/hPa stratus trigger — decks at all seasons 0.09–0.22; peak 0.20–0.37; omitted
  as a stated deviation ([open-questions.md](progress/open-questions.md), 14).
- Heat-transport slab pair (Q₀ = 50 W m⁻²) — 400 ppm 306.5 K ice-free, not equilibrated; 800 ppm
  stopped at the humidity envelope at 311 K; record only (entry "Owner steps 1–3" below).

---

## 2026-11-23 — Question 13, review round: where the tropical cloud loss and the Earth excess sit (branch `p4-clouds`)

Owner plan (review adopted): one diagnostic round before deciding: (1) split the anvil
pair's feedback; (2) fixed-SST factorial on the frozen slab pair's own SST and ice; (3) locate
the Earth tropical CRE excess by level; (4) overnight, the frozen slab pair with ocean heat
transport, diagnostic only. Stop after the round and propose, do not implement. No tuning.

**Code (diagnostic only, off by default).**
- `[clouds] cre_split_low_sigma = σ_low` (needs `cre_split`): the extra radiation call drops
  the *stratiform* cloud at σ ≥ σ_low (keeps the convective cloud) instead of dropping the
  convective cloud. SCM test `cre_split_low_memory` (column and anvil geometries).
- Snapshot field `cl_split` (the fourth cloud-memory block).
- Byte check against the q13b references with the option off: identical (10-day
  `aquaplanet_t21`, 292 files; `ape_control_t21`, 297 files). `cargo fmt --check`, clippy pass.
- Tools: `tools/slab_bc.py` (prescribed SST/ice from a slab run's monthly climatology),
  `tools/review_round.py` (20°S–20°N CRE split, cover, convection statistics, cloud profiles
  from snapshots; `--fig`). Cases in `cases/bench/p4_review/`, output in `out/p4/review/`.
  Binary `out/p4/toyclim_rev`, 4 threads per run.

**Item 1 — the anvil pair split** (year 71 from the year-70 restarts, T21, `anvsplit_t21_*`;
20°S–20°N, W m⁻², SW / LW / net; figure `review_anvil_split.png`):

| | 400 ppm (303.4 K) | 800 ppm (314.7 K) | change |
|---|---|---|---|
| all cloud | −78.1 / +29.4 / −48.7 | −52.6 / +18.8 / −33.8 | +25.5 / −10.6 / **+14.9** |
| stratiform only | −60.0 / +22.3 / −37.7 | −45.2 / +16.1 / −29.0 | +14.8 / −6.2 / **+8.7** |
| convective part | −18.1 / +7.1 / −11.0 | −7.4 / +2.6 / −4.8 | +10.7 / −4.5 / **+6.2** |

- Cover 0.671 → 0.442; deep-convection frequency 0.27 → 0.12, skipped 0.60 → 0.72, CAPE
  1242 → 2362 J kg⁻¹. Stratiform cover at σ 0.87 (the boundary-layer top) 0.29 → 0.20.
- **With the anvil geometry the stratiform cloud carries 58 % of the tropical net CRE
  change.**
- Figure: at 400 ppm a sharp equatorial SW minimum (−103) and ITCZ LW peak (44); at 800 ppm
  the minimum is −67 at 3°S and the LW peak 26. The convective part's equatorial SW falls
  −35 → −15; in 10–30° the loss is mostly stratiform. One year only, some hemispheric noise.

**Item 2a — slab references** (frozen knobs, `cre_split` on, 2 years with a monthly
climatology: 400 ppm years 46–47 from `gate/slab_t21_400`, 800 ppm years 56–57 from
`gate/slab2_t21_800`, the 800 ppm state used for the feedbacks):

| 20°S–20°N | 400 ppm (297.9 K) | 800 ppm (310.1 K) | change |
|---|---|---|---|
| all cloud | −90.6 / +25.9 / −64.6 | −61.3 / +18.2 / −43.1 | +29.3 / −7.7 / **+21.5** |
| stratiform only | −61.5 / +18.5 / −43.0 | −47.3 / +15.4 / −31.9 | +14.2 / −3.1 / **+11.1** |
| convective part | −29.1 / +7.4 / −21.7 | −14.0 / +2.8 / −11.2 | +15.1 / −4.6 / **+10.5** |

- **Correction to step 1.3 (2026-11-22):** there the convective part carried 85 % (+11.3
  of +13.3), but the 800 ppm state was the year-45 restart of the first, unequilibrated
  800 ppm run (308.6 K). Against the equilibrated 800 ppm state the split is about half and
  half (convective 0.49). **Decision rule (ii) of step 1 (more than half) would have failed**
  with this state: the diagnosis that justified the anvil redesign depended on the 800 ppm
  state chosen, and was weaker than reported.

**Item 2b — fixed-SST factorial** (T21, 1 + 2 years from rest, same orbit and physics as the
slab, prescribed monthly SST and ice from item 2a; ice floe 3.23 / 2.20 m N / S from the slab
mean thickness; `fx_s<SST>_c<CO2>_t21`; figure `review_factorial.png`; net CRE 20°S–20°N):

| | slab 400 | S400 C400 | slab 800 | S800 C800 | S800 C400 | S400 C800 |
|---|---|---|---|---|---|---|
| SW / LW / net, all | −90.6 / +25.9 / −64.6 | −89.6 / +25.8 / −63.8 | −61.3 / +18.2 / −43.1 | −63.8 / +18.6 / −45.3 | −69.3 / +20.0 / −49.2 | −84.5 / +23.6 / −60.8 |
| net, stratiform only | −43.0 | −42.0 | −31.9 | −35.1 | −38.8 | −39.9 |
| net, convective part | −21.7 | −21.8 | −11.2 | −10.2 | −10.4 | −20.9 |
| cover | 0.669 | 0.666 | 0.471 | 0.485 | 0.517 | 0.643 |
| deep / skipped frequency | 0.31 / 0.65 | 0.31 / 0.63 | 0.17 / 0.67 | 0.15 / 0.72 | 0.16 / 0.73 | 0.30 / 0.62 |
| CAPE [J kg⁻¹] | 881 | 830 | 1761 | 1892 | 2077 | 700 |
| c_c mean / conv. path [kg m⁻²] | 0.079 / 0.34 | 0.080 / 0.35 | 0.060 / 0.39 | 0.054 / 0.35 | 0.056 / 0.35 | 0.077 / 0.35 |
| stratiform cover at σ 0.87 | 0.318 | 0.312 | 0.223 | 0.227 | 0.258 | 0.281 |

Changes of net CRE from S400 C400 (W m⁻²; total = stratiform + convective):

| | total | stratiform | convective |
|---|---|---|---|
| S800 C800 (both) | **+18.5** | +6.9 | +11.6 |
| S800 C400 (SST warming + pattern) | **+14.6** | +3.2 | +11.4 |
| S400 C800 (CO₂ alone, cold SST) | **+3.0** | +2.1 | +0.9 |
| S800 C800 − S800 C400 (CO₂ alone, warm SST) | **+3.9** | +3.7 | +0.2 |
| slab pair, for comparison | +21.5 | +11.1 | +10.5 |

- *Method check:* S400 C400 matches the slab within 0.8 W m⁻² (net) and 0.003 (cover).
- *S800 C800 reproduces 86 % of the slab loss* (+18.5 of +21.5). The 3 W m⁻² gap is all
  in the stratiform part (+6.9 against +11.1): coupling/variability, small.
- *SST change:* 79 % of the loss, almost all in the convective cloud (deep convection
  becomes intermittent: frequency 0.31 → 0.16, CAPE doubles), as in question 13.
- *CO₂ alone:* 16–21 % of the loss, almost all in the stratiform cloud: at fixed SST the
  boundary-layer-top cover falls 0.312 → 0.281 (cold) and 0.258 → 0.227 (warm). A fast
  adjustment of the low cloud to the CO₂ forcing; it is not a semi-gray CO₂ fault as such
  (question 11 concerned the forcing size, +8 %/+29 %, not its vertical structure), but the
  excess stratospheric/upper cooling of Q11 is not excluded as a contributor. Not tested.
- *Not separated:* "SST warming" and "SST pattern" (the S800 SST differs from S400 both in
  mean and in pattern). The uniform +10 K APE pair of step 4.1 gave no cloud loss, but it
  also differs in orbit (equinox), instellation and SST profile, so this does not isolate the
  pattern. A run S400 + uniform 12.2 K (the slab's tropical-mean change) at 400 ppm would
  (about 1.3 h at T21). Not run (outside the plan).
- Figure: the fixed-SST curves overlay their slab counterparts at all latitudes except the
  poles (prescribed ice has a fixed thickness and no snow); the equatorial convective SW part
  goes −53 → −24 W m⁻² with the warm SST at either CO₂; the CO₂-only run weakens the ITCZ SW
  minimum from −112 to −107 and the 10–30° bands by a few W m⁻².

**Item 3 — Earth tropical excess by level** (Earth AMIP T42 gate, frozen knobs, year 7
from the year-6 restart, two runs with identical states: convective split, and
`cre_split_low_sigma = 0.7`; `earth_split{conv,low}_t42`; figure `review_earth_split.png`):

| W m⁻², SW / LW / net | 20°S–20°N | global |
|---|---|---|
| all cloud (CERES EBAF Ed4.1: tropics −44.8 / +30.4 / −14.4, global −45.2 / +25.8 / −19.5) | −62.6 / +20.3 / **−42.3** | −44.0 / +20.1 / −23.9 |
| convective part | −23.3 / +5.1 / −18.1 | −12.7 / +2.8 / −9.9 |
| stratiform at σ ≥ 0.7 (boundary layer) | −30.1 / +2.4 / **−27.7** | −17.0 / +1.6 / −15.5 |
| all cloud without it | −32.5 / +17.9 / −14.6 | −27.0 / +18.5 / −8.5 |

- **The tropical boundary-layer stratiform cloud's net effect (−27.7 W m⁻²) equals the
  tropical excess against CERES (−42.3 against −14.4, i.e. 27.9).** It is SW-only (LW +2.4).
- Where: it peaks at −35 W m⁻² SW at 5–20° in both hemispheres (the trade belts, in the
  subsidence regions), while the convective part peaks at the ITCZ.
- Profile (20°S–20°N mean, total / stratiform): σ 0.867 0.24 / 0.17, σ 0.930 0.12 / 0.05,
  other levels ≤ 0.10. A single layer at the boundary-layer top.
- Not saying the tropics should have no low cloud: CERES tropical cover is 0.65 against the
  model's 0.56, so the model's tropical cloud is too much low, reflective cloud and too
  little high cloud (LW CRE +20 against +30).

**Item 4 — slab pair with ocean heat transport (Q₀ = 50 W m⁻², Merlis et al. 2013 form)**:
`oht_t21_{400,800}`, 30 years from the year-45/55 restarts, 8 threads each, started 11:46 UTC
(about 9 h). Results in the next entry.

**Reading against the review's table.**
- "Loss follows CO₂ alone": no, 16–21 %.
- "Prescribed slab SST does not reproduce it": no, 86 % reproduced.
- "Loss follows the SST" (warming and pattern together): yes, 79 %, mainly through the
  intermittent deep convection (convective cloud). In the slab, and with the anvil geometry,
  the stratiform boundary-layer cloud carries half or more, part of it a direct CO₂ adjustment.
- "Excess is boundary-layer cloud": yes, fully, on Earth.
- So both suspects are real: the SST response goes through the convective cloud, and the
  boundary-layer stratiform layer carries the whole present-day Earth excess and half of the
  slab feedback.

**Proposal (not implemented): Slingo (1987) low cloud**, from the paper the owner supplied
(§3, eqs. 6–9; constants to be transcribed carefully, the PDF text layer is garbled in eq. 9):
- RH-based low cloud C_L = [max(0, (RH_e − 0.8)/0.2)]², set to zero under subsidence (ω ≥ 0)
  with a linear ramp to ω = −0.1 Pa s⁻¹ (eqs. 6–7). Slingo: in the subtropics "it suppresses
  the excessive cloudiness which would otherwise occur". That is the 5–20° excess here.
- Inversion cloud C_L = −6.67 Δθ/Δp − 0.667 from the most stable layer below 750 mb, cut by
  the relative humidity at the inversion base (eqs. 8–9). That is a stability-based
  boundary-layer cloud, whose response to warming and CO₂ differs from an RH threshold.
- Needed: ω in the column context (not passed now; the §9.3.3 driver contract would gain it),
  SCM tests, then the factorial's S400 C400 / S800 C800 and Earth T21 screens before any T42 work.
- Alternative: accept as a limitation (Q13 (iii)), with this round's attribution recorded.
- Whatever follows: the T42 re-tune should add the CERES zonal bands as targets.

**Not done:** the SST warming/pattern separation run; the ω-dependence check on saved states
(how much of the 5–20° low cloud sits in subsidence); item 4 results.

## 2026-11-23 (afternoon) — Owner steps 1–3 for question 13 (iv): separating run, Slingo low cloud, L26 inversion check; **stopped at step 3** (branch `p4-clouds`)

Owner decisions of the morning recorded under question 14 before starting. Machine kept at 16
threads: the heat-transport pair was pinned to 12 cores (`taskset`), the other runs to 4–6.

**Step 1 — separating run** (`cases/bench/p4_review/fx_s400p12_c400_t21.toml`, 1 + 2 years):
the 400 ppm slab SST and ice plus a uniform 12.20 K (the 20°S–20°N annual mean of the 800 − 400
slab SST difference), 400 ppm CO₂. 20°S–20°N, same quantities as the factorial
(`tools/review_round.py`, `out/p4/review/separating.json`):

| | S4C4 | S4+12C4 | S8C4 | S8C8 |
|---|---|---|---|---|
| T_s [K] | 297.87 | 310.08 | 310.08 | 310.08 |
| SW / LW / net CRE | −89.6 / +25.8 / −63.8 | −82.1 / +20.5 / −61.6 | −69.3 / +20.0 / −49.2 | −63.8 / +18.6 / −45.3 |
| net CRE without the convective cloud (stratiform) | −42.0 | −49.7 | −38.8 | −35.1 |
| net CRE of the convective cloud | −21.8 | −11.9 | −10.4 | −10.2 |
| cover | 0.666 | 0.570 | 0.517 | 0.485 |
| deep-convection / skipped frequency | 0.310 / 0.631 | 0.171 / 0.741 | 0.155 / 0.729 | 0.151 / 0.716 |
| CAPE [J kg⁻¹] | 830 | 2546 | 2077 | 1892 |
| low stratiform max (σ 0.867) | 0.312 | 0.325 | 0.258 | 0.227 |

- Uniform warming (S4+12C4 − S4C4): net +2.2 W m⁻² — convective part +9.9 (deep convection
  halves, CAPE triples), stratiform −7.7 (more subtropical cloud, SW CRE −90 at 20–30° in the
  zonal mean).
- SST pattern at the same tropical mean (S8C4 − S4+12C4): net +12.4 — stratiform +10.9,
  convective +1.5.
- So of the SST-driven +14.6, the warmer base state removes convective cloud but adds
  stratiform cloud (nearly cancelling), and the slab's SST pattern removes the stratiform
  cloud (85 % of the total). Record only.
- Figure `docs/figures/P4/review_separating.png`: the uniform case (orange) is noisy and
  reflects most at 20–30°; the slab-pattern cases (green, red) coincide; the equatorial
  convective SW CRE falls −53 → −35 (uniform) → −24 (pattern).

**Heat-transport slab pair** (record only, owner decision; item 4 of the review round,
`cases/bench/p4_review/oht_t21_*.toml`, Q₀ = 50 W m⁻², from the frozen pair's restarts):
- 400 ppm: finished (rc = 0, model years 62–75). Last 10 years (65–75, `tools/equilibrium.py`):
  annual global T_s 306.62, 306.63, 306.55, 306.40, 306.38, 306.68, 306.79, 306.58, 306.37,
  305.97 K; trend −0.037 K/yr (fails < 0.02), net TOA −0.34 W m⁻², closure residual −0.05,
  TOA − residual −0.30 (passes ±0.3): **NOT EQUILIBRATED** by the trend test (it drifts down
  in the last 4 years). Climatology means over years 65–75: global T_s 306.5 K (equator 321.6
  K, |φ| > 80° 289.0 K), sea ice 0 everywhere, cover 0.41, SW / LW CRE −44.9 / +22.6 W m⁻²;
  20°S–20°N T_s 318.1 K, SW / LW / net CRE −35.5 / +12.9 / −22.6 (no `cre_split` in this
  run, so no convective/stratiform split);
- 800 ppm: 300.7 → 311.0 K over 11 years, still rising 0.2–0.5 K/yr, then **stopped at day
  24144 (year 67)**: "specific humidity above 0.1 kg/kg for 101 consecutive steps: the
  atmosphere is outside the dilute envelope this model supports (SPEC.md §9.4.4)". No restart
  was written (restarts are written at the end only).
- With this heat transport the 400 ppm aquaplanet is about 17 K warmer than the frozen pair's
  (306.5 vs 289.4 K) and far outside 283–293 K; it is ice-free. No warming estimate: the 400
  ppm run fails the trend test and the 800 ppm run stopped at the humidity envelope before
  equilibrating (its last annual mean, 311.0 K, was still rising). Attribution: with Q₀ = 50
  W m⁻² the slab states leave the envelope the model supports, so this configuration cannot
  serve criterion 4; it stays record only, as decided.

**Step 2 — Slingo (1987) low cloud** (commit 415e65d), `[clouds] low_cloud = "slingo"`, off by
default:
- eqs. (5)–(9) transcribed in `physics/clouds/low.rs` with the citation; the equations were
  checked on the page images (the PDF text layer garbles eq. 9: it reads
  C_L = C_L'{1.0 − (0.8 − RH_base)/0.2}). Eq. (6) with RH_e (eq. 5) or the Sundqvist law
  (`low_rh`), gated by eq. (7) at σ ≥ 0.8 (Fig. 1); inversion cloud from the most stable
  pair of levels below 750 hPa (eq. 8) with the RH-at-base factor (eq. 9), placed at the base
  level, combined with the RH cloud as the maximum (the paper says only that the two "combine
  without difficulty"); no cloud in the lowest layer (as in the paper). Not done: the paper's
  one-layer-per-region placement. Knobs in `docs/tuning.md`.
- ω in the column physics: `ColumnCtx::omega`, computed by the driver from the physics state
  only when a scheme asks (`ColumnScheme::needs_omega`), by the Simmons–Burridge
  discretisation of the dynamics (`SigmaLevels::omega_over_p`, unit test: ω/p = −δ for uniform
  divergence); snapshot field `omega` on request.
- SCM tests: `slingo_low_suppressed_under_subsidence` (ω = +0.05, 0, −0.05, −0.1, −0.3 Pa/s →
  0, 0, ½, 1, 1 of [(RH − 0.8)/0.2]², eq. (5) with convection, the Sundqvist law),
  `slingo_inversion_cloud` (15 K over 80 hPa → C' = 6.67·0.1875 − 0.667 at the base, RH_base
  0.9/0.8/0.7/0.6/0.5 → factor 1/1/½/0/0, both geometries, with and without `cre_split`;
  surface inversion ignored; saturation at 1), `slingo_random_columns_in_range` (400 random
  columns: fractions in [0, 1], no T or q tendency). Freeze-dry and σ_min unchanged.
- Byte check with the option off against the q13b references: identical (10-day
  `aquaplanet_t21`, 294 files; `ape_control_t21`, 299 files). `cargo test --release` all
  pass (core 120 + 7 ignored), clippy, fmt.
- `physics/clouds.rs` (1121 lines with the new code; it was 702) split into
  `physics/clouds/{mod, anvil, low, config, tests}.rs` and the cloud config moved out of
  `physics/mod.rs` (877 → 680), so no file in `toyclim-core/src` is over 700 lines (P0 rule).

**Step 3 — does L26 resolve the inversion Slingo's eq. (8) uses?** Frozen-knob Earth AMIP,
physics unchanged: T21 1 + 1 years (`cases/bench/p4_slingo/earth_base_t21.toml`) and the T42
gate continued one year from its year-6 restart (`earth_inv_t42.toml`), 5-daily `t`, `rh`,
`omega`, `ps` (73–74 snapshots in the analysis year). `tools/inversion_check.py` applies eqs.
(8)–(9) exactly as the code does, per snapshot and column. Ocean points; box regions (mine;
Klein & Hartmann 1993 could not be fetched, HTTP 403, no open copy found, so no observed LTS
values are quoted); peak = the stratocumulus season of each deck.

| region (peak season) | T21 Δθ/Δp [K/hPa] | T21 P(< −0.1) | T21 C_inv | T42 Δθ/Δp | T42 P(< −0.1) | T42 C' (eq. 8) | T42 C_inv (eqs. 8–9) | T42 LTS [K] | T42 ω ≥ 0 share, σ ≥ 0.8 |
|---|---|---|---|---|---|---|---|---|---|
| Peru (SON) | −0.100 | 0.41 | 0.02 | −0.122 | 0.84 | 0.16 | 0.09 | 21.7 | 0.85 |
| Namibia (JJASON) | −0.094 | 0.45 | 0.01 | −0.125 | 0.81 | 0.19 | 0.06 | 19.7 | 0.71 |
| California (JJA) | −0.096 | 0.44 | 0.00 | −0.109 | 0.67 | 0.11 | 0.05 | 21.0 | 0.55 |
| Canary (JJA) | −0.085 | 0.17 | 0.01 | −0.107 | 0.57 | 0.09 | 0.04 | 19.7 | 0.82 |
| Australia (DJF) | −0.062 | 0.00 | 0.00 | −0.088 | 0.21 | 0.02 | 0.01 | 16.0 | 0.91 |
| trades 5–20°N (all) | −0.052 | 0.00 | 0.00 | −0.057 | 0.02 | 0.00 | 0.00 | 11.8 | 0.49 |
| trades 5–20°S (all) | −0.057 | 0.03 | 0.00 | −0.064 | 0.07 | 0.01 | 0.00 | 12.3 | 0.52 |
| warm pool (all) | −0.048 | 0.00 | 0.00 | −0.056 | 0.00 | 0.00 | 0.00 | 10.5 | 0.49 |

No region at either resolution has a single snapshot below −0.25 K/hPa (full cover by eq. 8).

- **Trade belts: not resolved.** The most stable layer at 5–20° is as stable as over the warm
  pool (−0.057/−0.064 against −0.056 K/hPa at T42); its mean θ profile is smooth
  (300.6, 300.7, 300.9, 302.7, 307.1, 313.0 K at σ 0.993 … 0.696).
- **Stratocumulus decks: an inversion exists but L26 spreads it over two layers.** Peru T42:
  θ 291.9, 292.1, 293.5, 300.3, 308.0, 313.5 K at σ 0.993, 0.971, 0.930, 0.867, 0.787, 0.696
  — a well-mixed layer, then 6.8 K across 0.930 → 0.867 (63 hPa) and 7.7 K across 0.867 →
  0.787 (80 hPa), about 14.5 K over 143 hPa. LTS 20–22 K in the decks against 10.5–12 K in the
  trades and warm pool, so the bulk stability contrast is there; but per layer the decks sit
  just past Slingo's zero point (−0.1 K/hPa), so eq. (8) with the paper's constants gives a
  deck cover of 0.09–0.19 (T42) and 0.01–0.05 (T21), and 0.04–0.09 after eq. (9) (base RH
  often below 0.8).
- The eq. (7) gate works where intended: ω ≥ 0 at σ ≥ 0.8 in 55–91 % of the snapshots over the
  decks and the eastern subtropical oceans (figure), about half over the trades and warm pool.
- Figures `docs/figures/P4/inversion_check_t21.png`, `inversion_check_t42.png` (maps of the
  mean Δθ/Δp, C', C_inv, the subsidence share; mean θ profiles): the most stable layers
  (about −0.12 to −0.15 K/hPa) hug the eastern-boundary coasts of the Americas and Africa;
  C' and C_inv are pale everywhere; the subsidence share is red over the subtropical oceans
  and blue along the ITCZ and storm tracks; the deck profiles show the two-layer jump, the
  trade and warm-pool profiles none.

**Stop condition reached** ("step 3 fails (inversion not resolved)"): nothing tuned, no
variant run, steps 4–8 not started. Proposal in question 13.

**Not done:** steps 4–8; any run with `low_cloud = "slingo"` in 3-D (the SCM tests are its only
tests); observed LTS values (paper not fetched).



## 2026-11-23 (evening) – 2026-11-24 (night) — Question 13 (vi)/(A): stability stratus, T21 variants; **stopped at step 4** (branch `p4-clouds`)

Owner decisions recorded under question 14 (history file): option (vi), then form (A) (CAM's
trigger omitted). Housekeeping: this file compressed (d0c744b).

- **Sources.** CAM3 note and Klein & Hartmann (1993) not fetched (UCAR 401/TLS, AMS 403). Read:
  the CAM source (`cloud_fraction.F90`, tag cam_cesm2_2_rel) and its namelist defaults; constants
  0.057 and −0.5573 confirmed; CAM gates the stratus with Δθ/Δp < −0.125 K/hPa (premib 750 hPa).
- **Offline** (`tools/lts_check.py`, T42 Earth output, 74 snapshots; figures
  `docs/figures/P4/lts_check_{inv_t42,base_t21}.png`): law alone 0.50 / 0.68 (Peru, all / peak),
  0.49 / 0.56 (Namibia), 0.45 / 0.63 (California), 0.36 / 0.57 (Canary), 0.22 / 0.35 (Australia),
  trades 0.14–0.16, warm pool 0.07; with CAM's trigger 0.09–0.22 all seasons (stop, owner chose
  (A)). Regions the trigger excluded: Southern Ocean 45–60°S 0.17 / 0.29, N Pacific 0.30 / 0.66,
  N Atlantic 0.24 / 0.47, ocean poleward 60°N 0.22 / 0.64, 60°S 0.12 / 0.28; no band above 0.8
  (largest 0.17); estimated global cover 0.494 → 0.534 (CERES 0.674).
- **Implementation** (81373cf): `low_cloud = "slingo_lts"`, knobs `lts_slope`, `lts_intercept`
  (`lts_slope = 0` = gate only); open-ocean tiles only, θ_s from the ocean tiles' skin
  temperature, k700 = σ nearest 0.7, placement in the lower level of the most stable pair below
  750 hPa, cap max(RH_k, RH_k+1), maximum with the RH cloud, freeze-dry and σ_min unchanged. SCM
  tests (`clouds/tests_lts.rs`): rises with LTS, zero below 9.78 K, open ocean only, f_ocn
  scaling, RH cap, maximum, gate, no tendencies, random columns in range, config keys. Off:
  byte-identical (10-day `aquaplanet_t21` 295 files, `ape_control_t21` 300, `earth_amip_t21`
  369, 0 differing); `cargo test`: 125 core tests pass.
- **Step 4, T21 Earth AMIP 1 + 1 years** (year 2; `cases/bench/p4_lts/t21/`, `out/p4/lts/t21/`;
  (a) = `out/p4/slingo/earth_base_t21`, same physics; gated RH cloud with `low_rh = "sundqvist"`):

  | variant | albedo | net TOA | SW / LW CRE | cover | 20°S–20°N net CRE (CERES −14.6) | Peru / Namibia low cloud |
  |---|---|---|---|---|---|---|
  | (a) frozen | 0.301 | +0.21 | −52.4 / +23.6 | 0.534 | −51.6 | 0.01 / 0.03 |
  | (b) gate only | 0.262 | +12.06 | −39.1 / +22.2 | 0.457 | **−29.0** | 0.01 / 0.01 |
  | (c) gate + law | 0.305 | −0.81 | −53.6 / +23.7 | 0.500 | −47.5 | 0.38 / 0.32 |
  | (d) RH_s 0.90 / 0.85 / 0.80 | 0.303 / 0.303 / 0.306 | | | 0.497 / 0.497 / 0.500 | −48.5 / −47.1 / −49.9 | |
  | (c) + `water_max` 3e-4 | 0.301 | +1.14 | −52.3 / +24.2 | 0.501 | −45.2 | |
  | (c) + `water_max` 2e-4 | 0.292 | +3.70 | −49.3 / +23.7 | 0.499 | −39.8 | |
  | (c) + `lts_intercept` −0.65 | 0.289 | +4.19 | −48.3 / +23.3 | 0.485 | −39.9 | |
  | (c) + `water_max` 2e-4 + RH_u 0.85 | 0.303 | +1.85 | −53.2 / +25.9 | 0.529 | −42.2 | |

  OLR 235.2–238.9 everywhere; the shape row fails at T21 in all variants, as with the frozen
  knobs. Figures (opened): `lts_t21_bands.png`, `lts_t21_bands2.png` — every variant keeps the
  equatorial SW CRE at about −80 to −95 against CERES about −50 while the cover there matches
  CERES (0.65–0.69 against 0.73): the remaining excess is too-bright cloud in the ascending
  tropics, which the subsidence gate does not touch; subtropical cover stays 0.3–0.4 (CERES
  0.55–0.6) and the Southern Ocean 0.55–0.6 (0.85–0.9). `lts_t21_lowcloud.png`: the gate removes
  the frozen scheme's low cloud over the tropical oceans and the Indian Ocean (0.4–0.6 → < 0.2);
  the law puts stratus along the west coasts (0.3–0.5) and in the North Pacific, as offline.
- **Stop condition:** no T21 variant brings the tropical net CRE within 15 W m⁻² of CERES while
  keeping criteria 1–2 and cover ≥ 0.5; only (b) is within (−29.0), and it fails albedo, net TOA
  and cover. Nothing adopted; steps 5–8 not started. Options in question 13.

## 2026-11-24 (day) — Question 13 after step 4: brightness diagnostic, T21 round, steps 5–7; question 7 rerun (branch `p4-clouds`)

Owner decision after step 4 (2026-11-24, recorded under 13/14 in the history file): the stop
was correct; the gate + stability law with the published constants are adopted for the rest of
P4; step 4b (diagnostic, then one T21 round of ≤ 8 variants with T21 cover ≥ 0.56); the
15 W m⁻² line is retired; then steps 5–8; Q7 rerun now.

**Step 4b, diagnostic on variant (c)** (year 3 from its year-2 restart, three runs with the
extra radiation call: without convective cloud, without stratiform cloud at σ ≥ 0.7 and at
σ ≥ 0.4; `cases/bench/p4_lts/t21b/`, `out/p4/lts/t21b/`; `tools/review_round.py`):

| W m⁻², SW / LW / net, 10°S–10°N | value |
|---|---|
| all cloud (CERES 10°S–10°N: −49.9 / +34.8 / −15.1) | −84.3 / +27.3 / −57.0 |
| convective part | −20.4 / +7.6 / −12.8 |
| stratiform at σ ≥ 0.7 | −42.9 / +4.6 / −38.3 |
| stratiform at σ ≥ 0.4 (so 0.4–0.7 adds −2.5 SW) | −45.4 / +5.2 / −40.2 |

- The tropical excess sits in one layer: the mean cloud fraction peaks at σ = 0.787 (0.223,
  stratiform 0.155), the level just above the gate's σ ≥ 0.8 region (L26 has no level between
  0.787 and 0.867). Its grid-mean water path there is 70.5 g m⁻² of the column's 225.5.
- LW CRE 10°S–10°N +27.3 against CERES +34.8 (short by 7.5 W m⁻²; 20°S–20°N +24.9 against
  +29.3): below the owner's 8 W m⁻² line, so the anvil option did not enter.
- Water path (`tools/water_path.py`, figure `docs/figures/P4/lts_t21_waterpath.png`) against
  ESA Cloud_cci AVHRR-PM v3 (obs4MIPs `clwvi` and `clivi`, monthly 2003–2012, fetched
  2026-11-24 from CEDA, `https://dap.ceda.ac.uk/neodc/esacci/cloud/data/obs4MIPs/DWD/
  ESACCI-CLOUD-AVHRR-PM-3-0/mon/`, DOI 10.5285/919157930d7447caac6d42e84e377289; technical
  notes read: daytime passive retrieval, liquid bias −3.2 g m⁻², ice in-cloud bias −365 g m⁻²
  against A-Train, so the ice comparison is weak; MAC-LWP and MODIS need an Earthdata account
  and were not fetched):

| g m⁻² | 10°S–10°N | 20°S–20°N | 15–35°N | 15–35°S | 45–65°S | global |
|---|---|---|---|---|---|---|
| Cloud_cci liquid / ice / total | 21.1 / 68.4 / 89.5 | 21.5 / 53.9 / 75.4 | 26.2 / 34.7 / 60.9 | 31.3 / 36.0 / 67.2 | 85.0 / 69.3 / 154.3 | 39.2 / 52.3 / 91.5 |
| variant (c) liquid / ice | 213.5 / 12.1 | 170.1 / 10.6 | 93.1 / 9.9 | 78.1 / 6.1 | 48.6 / 37.5 | 100.6 / 17.6 |
| ratio of totals | 2.5 | 2.4 | 1.7 | 1.2 | 0.6 | 1.3 |

  Snapshot LWP + IWP agrees with the `clwvi` map to 0.8 % (74 snapshots).
- Optics: liquid r_e 10 µm (Slingo 1989), ice r_e 30 µm (Ebert & Curry 1992), δ-scaled
  PIFM, **no sub-grid inhomogeneity reduction**; phase by a linear ramp 273.15 → 253.15 K.
- **Rule case: water path clearly above 1.5× observed in the tropics** → the in-cloud water is
  too high; lever `water_max` with its range extended to 1e-4 (docs/tuning.md), target the
  Cloud_cci 10°S–10°N total. The cap binds only in warm cloud: the 45–65°S path is 93–94 g m⁻²
  for every `water_max` of step 4.

**Step 4b, T21 round** (`cases/bench/p4_lts/t21r/`; band error = area-weighted RMS of the SW
and LW CRE differences over the seven `tools/cre_bands.py` bands, defined before the results;
figure `docs/figures/P4/lts_t21_round.png`):

| variant | albedo | net TOA | SW / LW CRE | cover | 20°S–20°N net CRE | band error |
|---|---|---|---|---|---|---|
| (c) step 4 | 0.305 | −0.81 | −53.6 / +23.7 | 0.500 | −47.5 | 12.86 |
| r1 `water_max` 1.5e-4, `rh_crit_upper` 0.85 | 0.296 | +4.21 | −50.5 / +25.8 | 0.527 | −37.9 | 9.74 |
| r2 1e-4, 0.85 | 0.281 | **+8.99** | −45.5 / +25.5 | 0.523 | −30.9 | 8.34 |
| r3 1.5e-4, 0.80 | 0.300 | +4.31 | −52.0 / +27.2 | 0.543 | −38.4 | 10.65 |
| r4 1e-4, 0.80 | 0.291 | +7.28 | −49.0 / +27.5 | 0.548 | −33.2 | 8.92 |
| r5 1e-4, 0.80, `rh_crit_surface` 0.90 | 0.294 | +6.50 | −49.9 / +27.7 | 0.553 | −32.2 | 7.78 |
| **r6 1e-4, 0.75** | 0.298 | +6.62 | −51.4 / +29.2 | **0.567** | −34.1 | 9.99 |
| r7 = r4 + `low_sigma` 0.85 | bitwise identical to r4 (no L26 level between σ 0.80 and 0.85) | | | | | |
| r8 1.5e-4, 0.75, `rh_crit_surface` 0.90 | 0.313 | +1.40 | −56.4 / +29.1 | 0.569 | −42.6 | 13.15 |

- Criteria 1–2 hold in all but r2 (net TOA). The shape row fails at T21 in all (as before).
  Cover ≥ 0.56: r6 and r8; **r6 (smaller band error) went on**. `low_omega` (Slingo's
  published 0.1 Pa s⁻¹) was not moved; the only other gate knob, `low_sigma`, cannot weaken
  the gate within 0.75–0.85 at L26.
- Low cloud of r6 (`docs/figures/P4/lts_t21_r6_lowcloud.png`): the stability stratus is as in
  (c); deck boxes Peru 0.37, Namibia 0.21, California 0.22, Canary 0.22.

**Step 5, factorial (diagnostic)** — S400/C400 and S800/C800 of the review round rerun with r6
(`cases/bench/p4_lts/step5/`, `out/p4/lts/step5/factorial.json`), 20°S–20°N, years 2–3:

| | frozen S400C400 | frozen S800C800 | r6 S400C400 | r6 S800C800 |
|---|---|---|---|---|
| SW / LW / net CRE | −89.6 / +25.8 / −63.8 | −63.8 / +18.6 / −45.3 | −71.3 / +32.3 / −39.0 | −82.9 / +29.4 / −53.4 |
| convective part (net) | −21.8 | −10.2 | −15.3 | −5.4 |
| cover | 0.666 | 0.485 | 0.630 | 0.642 |
| low stratiform max (σ) | 0.312 (0.867) | 0.227 (0.867) | 0.186 (0.787) | 0.395 (0.787) |

- With r6 the tropical net CRE change between the matching cases reverses sign: −14.4 W m⁻²
  (more cloud cooling) against +18.5 with the frozen knobs, carried by stratiform cloud at
  σ 0.787 that doubles in the warm case. Diagnostic only; nothing was chosen by it.

**Step 6, T42 pass** (r6, Earth AMIP 1 + 1 years, `cases/bench/p4_lts/step6/`, figure
`docs/figures/P4/lts_t42_pass.png`): albedo 0.295, OLR 237.2, net TOA +2.81, SW / LW CRE
−50.1 / +25.8, **cover 0.566** (aim ≥ 0.55 met), closure 0.30 W m⁻²; band error 8.73 (frozen
gate 11.48); 20°S–20°N net CRE −36.0 (CERES −14.4; frozen gate −43.1). **Shape row fails:**
the northern 40–70° maximum sits at the band edge (0.68 at 68°, 0.69 at 71°; the frozen gate
passed it). Knobs frozen at r6 in the case files (code defaults unchanged: "rh" stays the
default).

**Step 7, gate** (knobs frozen at r6; `cases/bench/p4_lts/gate/`, `out/p4/lts/gate/`, binary
`out/p4/toyclim_6041548`; T42 on 6 threads, slabs on 4 each, regress after the T42 run).

T42 Earth AMIP, 1 + 5 years (`tools/scorecard.py --suite earth_energy --milestone P4
--no-write`, window 1826 days; figure `docs/figures/P4/lts_t42_gate.png`):

| # | Criterion | r6 gate | frozen gate (2026-11-21) | |
|---|---|---|---|---|
| 1 | albedo; OLR; net TOA | 0.296; 237.1; +2.61 W m⁻² | 0.280; 242.5; +2.53 | met |
| 2 | SW / LW CRE | −50.2 / +25.8 | −44.6 / +19.9 | met |
| 3 | cover | **0.567** | 0.497 | met |
| 3 | zonal shape | N: storm-track max 0.69 at 68° **not a local maximum** (cover rises to 0.71 at 79.5°); S: 0.69 at 60°, local; subtropical minima 0.41 / 0.36; ITCZ 0.70 | N: 0.65 at 66°, local | **FAIL** |
| 5 | energy closure | 0.30 W m⁻² | 0.31 | met |

- Other means: T_s 289.4 K, precipitation 3.22 mm/day, P − E relative 7e-5, G 1.86, Courant
  0.64. Band error 8.72 W m⁻² (pass 8.73, frozen gate 11.48); 20°S–20°N net CRE −36.3 against
  CERES −14.4 (frozen gate −43.1).
- Figure: the r6 gate (green) lies on the T42 pass (orange) everywhere; against the frozen gate
  (blue) it removes most of the 30°S and 30°N SW overshoot and the equatorial notch, raises
  LW CRE to near CERES in 60°S–30°N, and adds 0.05–0.10 cover at all latitudes. North of 30°N
  cover climbs monotonically to 0.69–0.71 at 70–80°N, where CERES has a storm-track maximum
  (0.76 near 60°N), a dip near 70°N, and its rise toward the pole. The model therefore has no
  northern storm-track maximum. LW CRE north of 65°N is +28 against CERES +14.8; net CRE there
  is −0.3 against −9.5. The SH storm-track cover is 0.64 against 0.87 (40–65°S band).
- **Stop condition met: the T42 run cannot meet criteria 1, 2, 3 and 5 together** (owner
  decision of 2026-11-24, unchanged stop conditions). Criteria 1, 2, 5 and cover are met; the
  shape row of 3 fails as in the T42 pass. Nothing was tuned after the T42 pass; steps 6–7
  used r6 unchanged (it qualified at T21 under the owner's rule: criteria 1–2, cover ≥ 0.56,
  smaller band error than r8).
- Where the shape fails: the northern polar cloud. Step 4 of the completion plan (2026-11-18)
  names the next action for this row if the RH_c knobs fail: a published treatment of cold,
  dry polar air, with SCM tests. That is a physics change and is for the owner to authorise
  after this stop. r6 differs from the frozen knobs, which passed the row, in `rh_crit_upper`
  (0.90 → 0.75), `water_max` (→ 1e-4) and the low-cloud scheme (open ocean only). The lower
  upper RH_c is the change that adds cloud everywhere, including over the Arctic. This was not
  separated by a run.

T21 slab aquaplanet pair, r6 (`slab_t21_400` 75 years, `slab_t21_800` 85 years, from the
frozen-gate restarts at years 45 / 55, 30 years each; climatology = last 10 years; `tools/equilibrium.py`,
`tools/scorecard.py --suite aquaplanet`; figure `docs/figures/P4/lts_slab_gate.png`):

| | frozen 400 | frozen 800 | **r6 400** | **r6 800** |
|---|---|---|---|---|
| T_s (K) | 288.51 | 299.57 | **297.14** | 300.67 |
| trend (K/yr); TOA − residual | equilibrated | equilibrated | −0.016; −0.03 **equilibrated** | **−0.046** (> 0.02); −0.16: **not equilibrated** |
| sea-ice fraction | 0.107 (edge 58°) | 0 | **0** | 0 |
| albedo (all-sky / clear-sky) | 0.318 / 0.134 | 0.280 / 0.105 | 0.290 / 0.106 | 0.297 / 0.105 |
| SW / LW / net CRE | −62.5 / +24.7 / −37.8 | −59.5 / +23.4 / −36.1 | −62.5 / +31.8 / −30.7 | −65.3 / +30.8 / −34.4 |
| 20°S–20°N net CRE | −64.2 | −48.6 | −48.9 | −51.6 |
| cover | 0.58 | 0.52 | 0.58 | 0.59 |
| closure (W m⁻²) | | | 0.05 | 0.04 |

- **Stop condition met: the 400 ppm slab aquaplanet ends at 297.1 K, outside 283–293 K**
  (equilibrated by the 10-year test). With r6 the 400 ppm aquaplanet loses all its sea ice:
  the frozen-gate state (288.5 K, ice edge 58°) warms by 8.6 K in 30 years. The global SW CRE
  is unchanged (−62.5); the LW CRE is +7.1 W m⁻² larger (more and higher cloud from
  `rh_crit_upper` 0.75), and this warms the polar caps most: net CRE poleward of 65° goes from
  +10 / +14 (over ice) to −0.2 / −0.3 (open water).
- **2×CO₂ warming, measured once and not tuned: 3.53 K** (300.67 − 297.14). The 800 ppm run
  is still cooling (−0.046 K/yr over its last 10 years, last two years 300.41 / 300.35), so
  the equilibrium value is probably lower; this was not extended. Attribution (difference of
  the 10-year means, W m⁻²): clear-sky ASR +0.56, clear-sky −OLR +2.82, net CRE **−3.73**
  (SW −2.8, LW −1.0; 20°S–20°N −2.7, 20–40° −7.5 to −8.7). Neither state has sea ice, so the
  ice-albedo feedback that dominated the frozen pair (11.06 K: clear-sky ASR +9.9 with the
  ice gone, net CRE +1.8) is absent. The cloud response is negative, as in the step-5
  factorial (tropical net CRE −14.4 with r6). This pair is not comparable with the frozen
  pair: the base states differ by 8.6 K and by the presence of ice.
- Figure: r6 (green, yellow) has polar LW CRE of 40–45 W m⁻² against 20–25 (frozen 400,
  blue) and a SW CRE of −40 at the poles, where the frozen 400 has −5 to −15 over ice. Between
  400 and 800 ppm, r6 changes the SW CRE mainly in 20–40° (more cloud, cover 0.46 → 0.52); the
  frozen pair instead loses tropical cloud (cover 0.73 → 0.55 at the equator). CERES (black)
  is Earth and is shown only as scale.
- APE westerlies row fails in both slabs (question 6, a known limitation).

**`scripts/regress`** (ratchet, HEAD 8976b28, `out/p4/lts/gate/regress.log`): ignored tests
PASS; Held–Suarez T42 M5 rows PASS (identical numbers: jets 33.72 / 32.20 m/s, asymmetry
2.13); APE T21 gate FAIL only on the 35–60° westerlies row, −6.85 m/s, the same value as
recorded in P4.md before this work (question 6, closed as a known limitation). The bitwise
`scripts/regress --against 852ca60 --threads 1` was PASS earlier today. **No regression.**

**Outcome: stopped at step 7 on two of the owner's stop conditions** (T42 criterion 3 shape
row; 400 ppm slab 297.1 K). Steps 8 and the P4 close are not done; the knobs stay frozen at
r6 in the case files only, with code defaults unchanged ("rh"; the 2026-11-21 frozen knobs).
The closing entry's limitations are recorded for when P4 closes: (1) L26 does not resolve
inversions; (2) for non-Earth cases the law is an extrapolation, and exoplanet benchmarks run
with low cloud on and off; (3) recheck the law when the P6 ocean replaces prescribed SST;
(4) the remaining tropical excess (T42 20°S–20°N net CRE −36.3 against CERES −14.4; water
path 2.5× Cloud_cci in 10°S–10°N before the round) is for the P5 convection item.
Options for the owner are under question 13.

**Question 7 rerun** (`cases/bench/p4_retest/bistab_cold.toml`, new key
`[surface.sea_ice] initial_snow`, default 0, test `cold_start_snow_on_ice`; frozen P4 physics,
400 ppm, 1361 W m⁻²; 3 m ice + 100 kg m⁻² snow everywhere, mixed layer at freezing, ice
surface and atmosphere 240 K isothermal and dry; 12 years, `out/p4/q7/cold`, figure
`docs/figures/P4/q7_bistability.png`): stays ice-covered — annual-mean concentration 1.0 in
every cell over years 8–12; global T_s 218.4 (year 1) → 208.1 K (year 12), still cooling
(TOA −7.2 W m⁻² in year 12). The warm branch (frozen gate, years 41–45) has no ice
equatorward of 50° (edge at 58°, T_s 288.5 K). **Both branches of P2 acceptance 6 hold at
1361 W m⁻²**; the instellation sweep (record) is not run.

## 2026-11-24 (night) – 2026-11-25 (morning) — Question 13 after step 7: polar profile (A), screen of r1, r3, r5 (B); **none passes, stopped at step D** (branch `p4-clouds`)

Owner decision after step 7 (2026-11-24, follows 923fb1c), recorded under question 13: r6 not
adopted; the low-cloud scheme ("slingo_lts", published constants) and the lower water cap stay
as the basis; the "T21 cover ≥ 0.56" rule withdrawn; the +3.53 K recorded only; question 7
closed. Option 4 as a screen: A diagnostic, B screen of r1, r3, r5, C gate with the best passer,
D stop and propose if none passes. Binary `out/p4/toyclim_923fb1c` (HEAD, no code change).
Nothing was chosen by, or looked at for, the 2×CO₂ warming.

**Machine.** The screen started at 21:45 on 15 threads. Another project's jobs (about 6
threads) were running too, so the three slabs were stopped by PID after 7 minutes and requeued
behind the T42 runs (9 threads, then 6). The slab years 46–60 below are complete reruns.

### Step A — polar cloud profile (existing output only; record)

Tool `tools/polar_cloud_profile.py` (new). For 65–90°N and 65–90°S it gives the zonal, time-mean
cloud fraction per level (`cl` snapshots in the climatology window) and the WMO lapse-rate
tropopause on the climatological zonal-mean T. Layer fractions are summed over four layers:
above the tropopause, the upper troposphere (tropopause to 440 hPa), middle (440–680 hPa) and
lower (> 680 hPa). These are sums of layer fractions, not cover. It also gives the TOA LW CRE
(olr_cs − olr).

No existing r6 run at T42 or in the slab saved 3-D cloud, so the pairs are:
- T42 Earth: the r6 T42 pass (year 2, 38 snapshots) against the frozen gate's year 7 (74).
- T21 at fixed SST: the r6 400 ppm factorial case against the frozen one, both on the frozen
  400 ppm slab's SST and ice (294 snapshots each).
- T21 Earth round, r6 against r2: only `rh_crit_upper` differs (0.75 against 0.85).
- The r6 slab itself (297 K, ice-free) has TOA maps only.

Figure `docs/figures/P4/q13_polar_profile.png`.

| Pair, cap | Tropopause r6 / ref (hPa) | Above trop. | Upper trop. | Middle | Lower | TOA LW CRE r6 / ref (W m⁻²) |
|---|---|---|---|---|---|---|
| T42 Earth, 65–90°N | 185 / 184 | +0.049 (8 %) | +0.375 (65 %) | +0.155 (27 %) | −0.301 | 27.3 / 23.4 (+3.9) |
| T42 Earth, 65–90°S | 169 / 179 | +0.078 (16 %) | +0.297 (62 %) | +0.108 (22 %) | −0.149 | 18.0 / 15.5 (+2.5) |
| T21 fixed SST, 65–90°N | 172 / 192 | +0.159 (17 %) | +0.641 (69 %) | +0.135 (14 %) | −0.281 | 28.7 / 25.2 (+3.5) |
| T21 fixed SST, 65–90°S | 182 / 193 | +0.192 (22 %) | +0.515 (60 %) | +0.148 (17 %) | −0.334 | 29.4 / 25.5 (+3.8) |
| T21 Earth r6 − r2, 65–90°N | 185 / 187 | +0.334 (27 %) | +0.555 (45 %) | +0.230 (19 %) | +0.112 (9 %) | 31.7 / 26.3 (+5.4) |
| T21 Earth r6 − r2, 65–90°S | 169 / 169 | +0.068 (17 %) | +0.235 (58 %) | +0.093 (23 %) | +0.010 (2 %) | 18.9 / 17.7 (+1.3) |

Columns 3–6 are changes in summed layer fraction; the percentage is the share of the increase.

- **Where the r6 increase sits:** 60–69 % in the upper troposphere and 14–27 % in the middle,
  in every pair. Above the tropopause it is 8–22 % against the frozen knobs, and 17–27 % for
  `rh_crit_upper` alone.
- **Low cloud:** r6 has less low cloud poleward of 65° than the frozen knobs (−0.15 to −0.33),
  from the new low-cloud scheme.
- **Polar LW CRE:** larger by 2.5–3.9 W m⁻² at fixed SST or SST/ice. In the gates: T42 +5.4 N
  and +2.8 S; slab +17.4 N and +16.3 S. The slab numbers compare an ice-free 297 K state with an
  iced 288.5 K one (clear-sky OLR 240 against 195), so they are not a cloud-only difference.
- **Figure:** r6 (red) lies right of the frozen run (blue) at every level from the tropopause to
  680 hPa, at both poles, by 0.03–0.07 per level. Below 700 hPa it lies left. The tropopause is
  at 170–190 hPa in all. The LW CRE poleward of 60° is higher in r6 at every latitude, most in
  the north.
- **No layer split of the LW CRE.** The runs saved neither humidity nor level fluxes, and
  `toyclim radiation` is clear-sky only. A bookkeeping kernel c ε (OLR_cs − σT⁴) was tried and
  failed its own check: −18 against the model's +27 W m⁻² over 65–90°N, because it ignores the
  emission above the cloud. It was dropped. A real split needs a run with an extra radiation
  call, not done.
- **The owner's reading (item 2) is confirmed for the polar profile:** the r6 increase is upper
  and middle cloud, mostly below the tropopause. **But step B shows that this is not what
  melts the slab** (below).

### Step B — screen (T42 Earth AMIP 1 + 1 years; T21 slab 400 ppm years 46–60 from the frozen gate's year-45 restart)

Cases `cases/bench/p4_lts/screen/{r1,r3,r5}_{t42,slab}.toml`, runs `out/p4/lts/screen/`. The
cases are copies of the step 7 gate cases with the candidate's knobs and shorter lengths:

| Candidate | `water_max` | `rh_crit_upper` | `rh_crit_surface` |
|---|---|---|---|
| r1 | 1.5e-4 | 0.85 | — |
| r3 | 1.5e-4 | 0.80 | — |
| r5 | 1e-4 | 0.80 | 0.90 |

T42 (scorecard `earth_energy`, window year 2; `tools/cre_bands.py`; figure
`docs/figures/P4/q13_screen_t42.png`):

| | albedo; OLR; net TOA | SW / LW CRE | cover | shape row | closure | band error | 20°S–20°N net CRE | T42 screen |
|---|---|---|---|---|---|---|---|---|
| r1 | 0.294; 239.7; +0.42 | −49.8 / +22.8 | 0.527 | **fails**: N max 0.64 at 68°, not local | 0.30 | 9.30 | −40.2 | **fail** |
| r3 | 0.302; 238.4; −0.97 | −52.4 / +24.4 | 0.548 | met: N 0.66 at 66°, S 0.68 at 60°, both local | 0.30 | 10.19 | −42.0 | pass |
| r5 | 0.288; 238.9; +3.35 | −47.8 / +24.1 | 0.544 | **fails**: N max 0.67 at 68°, not local | 0.30 | 7.98 | −35.0 | **fail** |
| (r6 gate, years 2–6) | 0.296; 237.1; +2.61 | −50.2 / +25.8 | 0.567 | fails | 0.30 | 8.72 | −36.3 | — |

CERES 20°S–20°N net CRE is −14.4.

- **Figure:** the four runs nearly coincide.
  - North of 60°N, r3's cover peaks at 0.66 near 66° and dips to about 0.61 at 73–76°N. r1's
    cover is almost flat from 70 to 85°N, and r5's rises to 0.70 at the pole.
  - All four share the Arctic LW CRE excess (20–28 against CERES 10–15), the tropical net CRE
    excess, and Southern Ocean cover of 0.60–0.65 against CERES 0.87–0.93.
  - r1 and r3 differ only in `rh_crit_upper` (0.85 against 0.80), yet the shape row flips
    between them. The row is marginal at one year.

Slab (`tools/slab_screen.py`, new; T_s from daily diagnostics; ice fraction and edge from the
year-end `sic` snapshots; "trend toward melting" fixed in the tool before the results; figure
`docs/figures/P4/q13_screen_slab.png`):

| Years 56–60 | T_s (K) | T_s trend (K/yr) | Ice fraction, year-end | Ice gone by year | Slab screen |
|---|---|---|---|---|---|
| r1 | **295.97** | +0.025 | **0** | 56 (0.094 in year 46) | **fail** |
| r3 | **295.61** | +0.024 | **0** | 56 (0.092 in year 46) | **fail** |
| r5 | **297.37** | −0.011 | **0** | 53 | **fail** |
| r6 (gate run, same years) | 297.00 | +0.054 | 0 | 54 | (fail) |
| frozen gate, years 41–45 | 288.52 | +0.019 | 0.113 (edge 66°) | — | (pass) |

- **Figure:** all four runs leave 288.5 K along the same curve.
  - They reach 290.1–290.7 K in year 46, and their ice is gone by years 53–56.
  - r1 and r3 stay 1–1.5 K below r6, and r5 runs about 0.4 K above it.
  - The 283–293 K band is left in year 48–49 in every run.

**Screen result: no candidate passes.**
- r1 and r5 fail the T42 shape row and the slab.
- r3 passes T42 but fails the slab: 295.6 K and ice-free.

Step C is therefore not run.

**Why the slab warms in every candidate (existing output, record).** At fixed SST the
CERES-supported low-cloud change itself warms the aquaplanet, not mainly the polar upper cloud.
Same frozen 400 ppm SST and ice, r6 clouds against frozen clouds (`tools/cre_bands.py`):

| | Global net CRE | Global SW | Global LW | 20°S–20°N net | 20–40° net | 65–90° net |
|---|---|---|---|---|---|---|
| r6 | −22.9 | −52.4 | +29.5 | −39.0 | −17.1 / −19.3 | +10.4 / +9.9 |
| frozen | −37.7 | −62.4 | +24.7 | −63.8 | −37.4 / −37.0 | +6.9 / +5.4 |
| change | **+14.8** | +10.0 | +4.8 | +24.8 | +20.3 / +17.7 | +3.5 / +4.5 |

- The SW part (+10 W m⁻²) is the tropical and subtropical reflection removed by the gate and
  stability law, and the cap on water. On Earth this is the change toward CERES: band error
  13.18 against 20.82 on this SST.
- The polar caps add only +3.5 to +4.5 W m⁻² of net CRE.
- The slab end states agree:
  - r1, with `rh_crit_upper` 0.85, has 3–5 W m⁻² less LW CRE than r6 and still melts.
  - Global SW CRE is −60 to −63 in all candidates, against −62.5 for the frozen state. That
    state's tropical net CRE of −64 is far brighter than Earth's −14.
- So the frozen 400 ppm aquaplanet sat inside 283–293 K with a tropical cloud excess that is 4×
  Earth's. Any variant that moves the Earth tropics toward CERES with this scheme loses that
  reflection on the aquaplanet. This was found after the screen and changes no criterion.

### Step D — proposals (nothing implemented)

**(i) Published treatment of cold polar air or cloud above the tropopause** (completion-plan
step 4, already authorised). Sources fetched this session:
- **Freeze-dry.** Vavrus & Waliser (2008), J. Climate 21, 5673–5687, DOI 10.1175/2008JCLI2299.1:
  - The paper is still **not fetched** (AMS 403, no open copy). Only the PI's report, OSTI 940966,
    was read: low cloud is reduced under very dry conditions, by more than 30 % Arctic-wide in
    winter.
  - The model already applies the form and constants transcribed from CAM's
    `cloud_fraction.F90` (cam_cesm2_2_rel, lines 500–518; re-fetched): f = max(0.15, min(1,
    q/3 g kg⁻¹)).
  - CAM applies it only for p ≥ `premib`, a namelist value that was **not verified**. Ours uses
    σ ≥ 0.75. The r6 increase sits above 680 hPa, where freeze-dry does not act.
  - Extending it upward is not in any source found.
- **Cloud above the tropopause.** No published rule tied to a diagnosed tropopause was found.
  - CAM3, Collins et al. (2004), NCAR/TN-464+STR, fetched: §4.7 and Appendix C have no
    tropopause or minimum-pressure cloud rule.
  - ECHAM5, Roeckner et al. (2003), MPI-M Report 349, fetched (archived copy): its tropopause
    (Eq. 11.14) is used only for aerosols.
  - CAM's code has a fixed cloud-top pressure (`trop_cloud_top_press`, a namelist value not
    verified). Our σ_min = 0.1 is the same kind of limit.
  - Step A puts only 8–22 % of the r6 polar increase above the tropopause. **This route
    therefore has no source and little to remove; not proposed.**

**(ii) Published levers that add low or middle cloud in the subtropics and the Southern Ocean
without upper polar cloud:**
1. **Estimated inversion strength in place of LTS in the stability law.** Wood & Bretherton
   (2006), J. Climate 19, 6425–6432, DOI 10.1175/JCLI3988.1, full text fetched (author's copy):
   - Eq. (4): EIS = LTS − Γ_m^850 (z700 − LCL).
   - Eq. (5): Γ_m from T and q_s.
   - Eq. (6): z700 = (R_a T0/g) ln(p0/700 hPa).
   - LCL from T0, p0 and RH0.
   - Fit: CF = 0.14 + 0.060 EIS (± 0.06, ± 0.010 K⁻¹; r = 0.92). It unifies the subtropical
     and midlatitude regimes; with LTS, midlatitude low cloud is higher than its LTS predicts,
     which is where we lack Southern Ocean cover.
   - Under uniform warming EIS barely changes (§4), unlike LTS.
   - It acts only below the existing gate (σ ≥ 0.8), so it adds no upper polar cloud.
   - Needs: the law swapped as an option (`low_cloud` variant), SCM tests, and a T21 round
     against CERES.
2. **CAM3 liquid/ice partition, −10 to −40 °C instead of our 0 to −20 °C.** Collins et al.
   (2004) §4.5.3, Eq. (4.132), fetched: f_i = (T − T_max)/(T_min − T_max), T_max = −10 °C,
   T_min = −40 °C.
   - More supercooled liquid (r_eff 10 µm against 30 µm ice) brightens the cold midlatitude and
     Southern Ocean cloud without adding cloud amount. Southern Ocean SW CRE is −27 to −29
     against CERES −37.5 in 40–65°S.
   - Side effects to expect:
     - liquid's LW coefficient (0.090 m² g⁻¹ against 0.038 for ice) raises polar LW CRE where
       cloud water is liquid;
     - the same ramp sets the precipitation phase and freezing in the condensable, so it would
       need a cloud-only ramp.

**Status.** P4 is stopped at step D.
- Nothing is adopted, and the code defaults are unchanged.
- The cases keep the candidates in `cases/bench/p4_lts/screen/` only.
- `scripts/regress` was not rerun: no code changed since 923fb1c, where it showed no
  regression.
- The P2 instellation sweep waits for frozen knobs (owner).

Options are under question 13.

## 2026-11-25 (day) — Question 13, owner decision after the screen: r3 gate; **stopped at step 1, T42 shape row fails** (branch `p4-clouds`)

Owner decision of 2026-11-25 (follows ad03af1), recorded under question 13 here and in the
history file. r3 = `low_cloud = "slingo_lts"`, `low_rh = "sundqvist"`, `water_max` 1.5e-4,
`rh_crit_upper` 0.80, in case files only (`cases/bench/p4_lts/gate_r3/`,
`cases/bench/p4_lts/close_r3/`). Binary `out/p4/toyclim_923fb1c` (no code change since).
Threads: T42 6, 800 ppm slab 5, 400 ppm slab 3, cold start 2 (16); regress run with
`taskset` on 8 free cores after the T42 run; no other project's jobs were running.

### Step 1a — T42 Earth AMIP 1 + 5 years

`out/p4/lts/gate_r3/earth_amip_t42`, `tools/scorecard.py --suite earth_energy --milestone P4
--no-write` (window 1822 days); figures `docs/figures/P4/q13_r3_gate_t42.png` and
`q13_r3_gate_shape.png`.

| # | Criterion | **r3 gate** | r6 gate | frozen gate | |
|---|---|---|---|---|---|
| 1 | albedo; OLR; net TOA | 0.301; 238.4; −0.56 W m⁻² | 0.296; 237.1; +2.61 | 0.280; 242.5; +2.53 | met |
| 2 | SW / LW CRE | −52.0 / +24.2 | −50.2 / +25.8 | −44.6 / +19.9 | met |
| 3 | cover | **0.545** | 0.567 | 0.497 | met |
| 3 | zonal shape | N: maximum in 40–70° **0.668 at 68.4°, the band's poleward row: not local**; S: 0.67 at 60°, local; subtropical minima 0.40 / 0.35; ITCZ 0.67 | N at the edge (68°) | met | **FAIL** |
| 5 | energy closure | 0.30 W m⁻² | 0.30 | 0.31 | met |

- Other means: T_s 289.29 K, precipitation 3.20 mm/day, P − E relative 1.2e-4, fixer
  0.12 %/day, G 1.84, spectral Courant max 1.05 (r6 0.64; the run was stable).
- **Shape row, profile and margin** (5-year climatology, northern rows from 40°N):
  0.474 (40.5°), 0.507, 0.533, 0.556, 0.579, 0.604, 0.618, 0.628 (60.0°), 0.646, 0.663 (65.6°),
  **0.668 (68.4°)**, then outside the band 0.665 (71.2°), 0.659, 0.668 (76.7°), 0.665 (79.5°),
  0.648, 0.636, 0.657 (87.9°). The rule (unchanged since 7ee997c, 2026-11-18) needs the
  maximum of 40–70° below the band's poleward row (68.4° at T42). Here that row exceeds the
  next one (65.6°) by **0.005**. Cover rises monotonically from 40°N and then forms a plateau of
  0.65–0.67 from 65° to 80°N; there is no storm-track peak in the north.
- Single years (10-day snapshots, 36–38 per year): the northern maximum is local at 65.6° in
  years 2 and 5 (0.665, 0.663) and on the edge row in years 3, 4 and 6 (0.679, 0.670, 0.691).
  **The screen's pass (1 + 1 years, "0.66 at 66°") rested on year 2 alone**, which this run
  reproduces (0.665 at 65.6°). The southern maximum is local in every year (57–63°).
- Figure `q13_r3_gate_shape.png`: NH, grey single years scatter by about ±0.02 north of 65°,
  the 5-year mean (blue) is flat from 65° to 80°. SH, a clear peak of 0.67 at 60° and a fall to
  0.54 at 77°.
- **CERES bands** (`tools/cre_bands.py`; net CRE / cover; band error 10.09, r6 8.72, frozen
  11.48):

| | 90–65°S | 65–40°S | 40–20°S | 20°S–20°N | 20–40°N | 40–65°N | 65–90°N | global |
|---|---|---|---|---|---|---|---|---|
| CERES net CRE | −2.8 | −37.5 | −22.8 | −14.4 | −16.3 | −23.7 | −9.5 | −19.5 |
| r3 net CRE | +4.3 | −28.8 | −26.1 | **−41.4** | −25.6 | −18.3 | −1.7 | −27.7 |
| CERES cover | 0.69 | 0.87 | 0.63 | 0.65 | 0.54 | 0.71 | 0.72 | 0.67 |
| r3 cover | 0.59 | **0.62** | 0.38 | 0.60 | 0.44 | 0.56 | 0.66 | 0.55 |

- Figure `q13_r3_gate_t42.png`:
  - r3 (blue) lies between r6 (orange) and the frozen gate (green) everywhere.
  - Tropical SW CRE peaks at −76 near 7°N against CERES −45 to −58, the remaining excess.
  - LW CRE is short in 20°S–20°N (25 against 30) and too large poleward of 65°N (26 against 15).
  - Southern Ocean cover is 0.62–0.67 against CERES 0.87–0.93.
  - In the north, cover climbs to 0.66 and stays flat. CERES has its 0.76 peak near 60°N and a
    dip near 70°N.
- **Cloud_cci water path** (`tools/water_path.py`; g m⁻²; snapshot check 0.998):

| | 10°S–10°N | 20°S–20°N | 15–35°N | 15–35°S | 45–65°S | global |
|---|---|---|---|---|---|---|
| Cloud_cci liquid / ice | 21.1 / 68.4 | 21.5 / 53.9 | 26.2 / 34.7 | 31.3 / 36.0 | 85.0 / 69.3 | 39.2 / 52.3 |
| r3 liquid / ice | 85.9 / 11.5 | 78.8 / 11.0 | 52.8 / 10.6 | 48.4 / 7.5 | 58.3 / 42.8 | 57.1 / 18.3 |
| ratio of totals (variant (c), 2026-11-24) | 1.09 (2.5) | 1.19 (2.4) | 1.04 (1.7) | 0.83 (1.2) | 0.66 (0.6) | 0.82 (1.3) |

  - Total paths are now near Cloud_cci in the tropics.
  - Liquid is still 4.1× in 10°S–10°N, and ice is 0.17×, the weak part of the comparison.
  - The Southern Ocean is short in both phases.

### Steps 1b–1c — slab pair (T21, 20 m slab, 400 / 800 ppm)

Both runs started from the frozen gate's year-45 restart (`out/p4/gate/slab_t21_400`):
- 400 ppm: continues the r3 screen run (years 46–60) to year 75;
- 800 ppm: years 46–75.

Climatology over years 66–75; `tools/equilibrium.py`; aquaplanet suite. Figure
`docs/figures/P4/q13_r3_slab_gate.png`.

| | frozen 400 | frozen 800 | r6 400 | **r3 400** | **r3 800** |
|---|---|---|---|---|---|
| T_s (K) | 288.51 | 299.57 | 297.14 | **295.62** | **298.57** |
| trend (K/yr); TOA − residual | equilibrated | equilibrated | equilibrated | −0.013; +0.02 **equilibrated** | −0.019; −0.03 **equilibrated** |
| sea ice | 0.107 (edge 58°) | 0 | 0 | 0 (since year 56) | 0 (since year 53) |
| albedo all-sky / clear-sky | 0.318 / 0.134 | 0.280 / 0.105 | 0.290 / 0.106 | 0.291 / 0.107 | 0.300 / 0.106 |
| SW / LW / net CRE | −62.5 / +24.7 / −37.8 | −59.5 / +23.4 / −36.1 | −62.5 / +31.8 / −30.7 | −62.8 / +29.2 / −33.5 | −66.0 / +28.6 / −37.4 |
| net CRE 20°S–20N; 65–90° | −64.2; +11.8 | −48.6; −9.7 | −48.9; −0.3 | **−55.2; −2.6** | −58.3; −2.9 |
| cover global; 20°S–20°N | 0.58; 0.67 | 0.52; 0.51 | 0.58; 0.64 | 0.55; 0.62 | 0.56; 0.62 |
| closure (W m⁻²) | | | 0.05 | 0.05 | 0.03 |

- Both equilibrated 30 years after the scheme change (owner limit 45). Aquaplanet suite: 400
  ppm passes every row (the 35–60° westerlies row too); 800 ppm fails only that row.
- **Row 4, measured once (not tuned, not marked met): 400 ppm T_s 295.62 K (outside
  283–293 K); 2×CO₂ warming +2.96 K (within 1.5–6 K).**
- **Attribution of the 400 ppm state against the frozen one (+7.1 K)**, differences of the
  10-year means:
  - *Tropical cloud.* 20°S–20°N net CRE −55.2 against −64.2 (+9.0 W m⁻² less cooling;
    global net CRE +4.3). On Earth the same band is −41.4 against CERES −14.4 (frozen −43.1).
  - *Ice.* The ice is gone (fraction 0.107 → 0), clear-sky albedo 0.134 → 0.107, about
    +9 W m⁻² of clear-sky absorbed SW globally. Over the polar caps the net CRE goes from
    +11.8 over ice to −2.6 over open water.
  - *CO₂ forcing error.* The semi-gray CO₂ forcing error (+8 % cold, +29 % warm against RRTMG,
    question 11) concerns the 400 → 800 ppm step, not the 400 ppm base. It was not separated.
- **2×CO₂ attribution** (`tools/feedbacks.py`, semi-gray forcing 3.775 W m⁻² from
  `out/p4/snap_t21_400/co2_forcing.json` as before; W m⁻² K⁻¹, contributions to the global
  value):

| | frozen (11.06 K) | r6 (3.53 K, 800 not equilibrated) | **r3 (2.96 K)** |
|---|---|---|---|
| clear-sky LW (0–30 / 30–60 / 60–90°) | −1.38 (−0.57 / −0.43 / −0.38) | −0.27 | −0.16 (−0.18 / +0.01 / +0.01) |
| clear-sky SW (ice) | +0.90 | +0.16 | +0.15 |
| SW CRE (0–30 / 30–60 / 60–90°) | +0.28 (+0.81 / −0.24 / −0.29) | −0.80 (−0.33 / −0.43 / −0.04) | **−1.11 (−0.35 / −0.69 / −0.07)** |
| LW CRE (0–30 / 30–60 / 60–90°) | −0.12 (−0.27 / +0.05 / +0.10) | −0.26 (−0.21 / −0.07 / +0.02) | −0.21 (−0.28 / +0.02 / +0.05) |
| sum; implied equilibrium ΔT | −0.32; 11.7 K | −1.17; 3.24 K | −1.33; 2.84 K |

  - Tropical (0–30°) cloud term: −0.64 (SW −0.35, LW −0.28) against +0.54 in the frozen pair.
    Tropical cover holds at 0.62, where the frozen pair lost 0.67 → 0.51.
  - Polar (60–90°) cloud term: −0.01 against −0.19.
  - The largest term is SW CRE in 30–60° (−0.69): 20–40° net CRE −29.3 → −38.0, with cover at the
    subtropical minima 0.39 → 0.44.
  - With no ice in either state, the ice-albedo term that dominated the frozen pair (+0.90) is
    small.
  - The clear-sky LW term is small (−0.16), because the forcing used is the frozen 400 ppm
    state's (+29 % warm-state error; question 11). The term absorbs that error. Not separated.
- Figure `q13_r3_slab_gate.png`:
  - r3 400 (green) and r3 800 (yellow) almost overlap: equatorial SW CRE about −108 and
    tropical cover peak 0.72 in both.
  - The 800 ppm state adds SW cooling and cover in 20–40°.
  - The frozen pair (blue, red) instead loses tropical cloud: equatorial cover 0.73 → 0.55, SW
    −115 → −83.
  - Frozen 400 has +15 W m⁻² net CRE over its polar ice.
  - CERES (black) is Earth, shown as scale only.

### Step 1e — `scripts/regress`

Ratchet (`out/p4/lts/gate_r3/regress.log`, HEAD ad03af1, no code change): ignored tests PASS;
Held–Suarez T42 jets 33.72 / 32.20 m/s, asymmetry 2.13 (PASS, identical); APE T21 gate fails
only the 35–60° westerlies row at −6.85 m/s, the value recorded before (question 6). **No
regression.**

### Step 2 — TRACMIP (Voigt et al. 2016)

Voigt et al. (2016), JAMES 8, 1868–1891, DOI 10.1002/2016MS000748. Full text fetched (KIT
repository copy; Wiley refused automated download) and read.

- **AquaControl global-mean T_s over 14 models: 290.4–300.7 K, median 295.1 K** (Table 4).
  8 of the 14 lie within ±2 K of the median.
- The paper attributes the models being 3–12 K warmer than present-day Earth to the absence of
  sea ice, land and aerosols.
- Climate sensitivity (half the 4×CO₂ warming): 1.5–4.8 K, median 3.3 K.
- How its setup differs from ours:

| | TRACMIP AquaControl | our slab aquaplanet |
|---|---|---|
| Slab | 30 m | 20 m |
| Ocean heat transport | q-flux: 0.5 PW northward across the equator, global mean zero | none |
| Sea ice | off; the ocean may go below freezing | on |
| CO₂ | 348 ppm, with CH₄ 1650 ppb and N₂O 306 ppb | 400 ppm |
| Solar constant | 1365 W m⁻² | 1361 W m⁻² |
| Eccentricity; obliquity | 0; 23.5° | 0.0167; 23.44° |
| Radiation | full radiation codes | semi-gray |
| Spin-up and averaging | 15 years + 30 years | 30 years after the change, 10-year mean |

  Our diurnal cycle is on.
- Our 295.6 K sits 0.5 K above the TRACMIP median. This is a comparison only; the
  criterion is unchanged.

### Step 3 — record only (partial)

- **Cold start with r3** (`cases/bench/p4_lts/close_r3/bistab_cold.toml`, 12 years,
  `out/p4/lts/close_r3/bistab_cold`):
  - stays fully ice-covered: concentration 1.0 everywhere in the years 8–12 mean;
  - T_s 218.4 (year 1) → 208.3 K (year 12), still cooling at −7.2 W m⁻² in year 12;
  - this is within 0.2 K of the frozen-knob run of 2026-11-24.
  - The warm branch at 1361 W m⁻² is the r3 400 ppm state (ice-free).
  - **Both branches hold with r3.**
- **Instellation sweep** (98–88 %, from the r3 year-75 restart): **stopped by me after about one
  model year** (rc 143, PIDs killed at 12:34 UTC) once the T42 stop condition was known. The
  cases stay in `cases/bench/p4_lts/close_r3/` for whenever the knobs are frozen; at 1 thread a
  15-year member takes about 13 h.
- **APE Control T42 with r3: not run.**

### Step 4 — not done

The T42 run fails criterion 3's shape row, the owner's first stop condition, so no further
variants were tried. r3 is not made the code default, `docs/tuning.md` is not changed (no knob
changed in code), `scripts/check` was not rerun (no code change), and the P4 closing entry is
not written. Options are under question 13.

## 2026-11-26 (night) — Question 13, owner decision after the r3 gate: EIS law, step A; **stopped at step A, (b) fails** (branch `p4-clouds`)

Owner decision of 2026-11-25 evening (follows a526a57), recorded under question 13 here and in
the history file before starting: option 3, the Wood & Bretherton (2006) inversion-strength law,
brought into P4; step A first, on existing output only. Machine idle at the start (03:30, load
0.4); no runs were started.

**Paper.** Wood, R. and C. S. Bretherton (2006), J. Climate 19, 6425–6432, DOI
10.1175/JCLI3988.1. Read in full from the author's final copy
(atmos.uw.edu/~robwood/papers/eis/eis_final.pdf, fetched 2026-11-26; the AOML mirror and AMS
returned 403). Transcribed:
- eq. (4) EIS = LTS − Γ_m^850 (z700 − LCL), Γ_m^850 = Γ_m([T0 + T700]/2, 850 hPa) (eq. 3 with
  two adiabats; "our results are insensitive" to the simplification);
- eq. (5) Γ_m(T, p) = (g/c_p)[1 − (1 + L_v q_s/(R_a T)) / (1 + L_v² q_s/(c_p R_v T²))];
- eq. (6) z700 = (R_a T0/g) ln(p0/700 hPa);
- LCL from T0, p0 and a **constant RH0 = 0.8** (COADS 60°S–60°N mean 0.8, s.d. 0.07); the paper
  gives no LCL formula ("standard procedures exist");
- §3: **CF = 0.14 ± 0.06 + (0.060 ± 0.010 K⁻¹) EIS** (95 %), r = 0.92, seasonal means of low
  stratiform amount (St, Sc, fractocumulus, fractostratus, fog; no cumulus) for the Klein &
  Hartmann regions with LTS > 10 K. **The agent's summary (0.14 + 0.060 EIS) is verified.**
- T0 in the paper is the surface air temperature; LTS = θ700 − θ0.
- §4: under moist-adiabatic warming EIS′ ≈ 0.75 T′700 − T′0 ≈ 0 near 285 K.

**Tool** `tools/eis_check.py` (new). It applies the law as the model would, mirroring
"slingo_lts": LTS exactly as the scheme; T0 = SST − 6.5 K km⁻¹·orog (the scheme has no surface
air temperature, about 1 K warmer than W&B's T0); k700 for 700 hPa; 0.85 p_s for 850 hPa; the
model's saturation curve; LCL by dry-adiabatic lift of RH0 0.8 air to saturation. Both laws are
applied without the RH cap and freeze-dry, because the r3 run saved no humidity.
- Check against W&B Table 1 (EIS rebuilt from the tabulated LTS and T0, p0 1010 hPa): the code
  gives 0.6–1.7 K less than the table (Peru 6.3 against 6.9, Canary 0.8 against 2.5, North
  Pacific 5.0 against 6.7). With p0 = 1000 hPa the gap is 0.3–1.6 K. The table averages
  seasonal values, which may account for part of the gap; not resolved, recorded.
- RH0 sensitivity on the frozen T42 run with saved `rh` (`out/p4/slingo/earth_inv_t42`, ocean
  60°S–60°N, 103 333 columns): the model's lowest-level RH is 0.70 ± 0.08. EIS with that RH is
  higher by 1.17 K on average (5–95 %: −0.45 to +2.51 K).

Run: r3 T42 Earth AMIP gate (`out/p4/lts/gate_r3/earth_amip_t42`, years 2–6, 184 10-daily
snapshots; climatology for (a)). Output `out/p4/eis/stepA/eis_check_r3_t42.{txt,json}`.

### (a) Total cover against CERES EBAF Ed4.1 (`cldarea_total_daynight`, 2001–2020)

Fetched 2026-11-22 (Zenodo extract, `reference/derived/SOURCES.md`). Cloud_cci cover was not
fetched; only its water paths are on disk.

| lat (°N) | 32.1 | 37.7 | 40.5 | 46.0 | 51.6 | 57.2 | 60.0 | 62.8 | 65.6 | 68.4 | 71.2 | 73.9 | 76.7 | 79.5 | 85.1 | 87.9 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r3 | 0.420 | 0.451 | 0.474 | 0.533 | 0.579 | 0.618 | 0.628 | 0.645 | 0.663 | 0.668 | 0.665 | 0.659 | 0.668 | 0.665 | 0.636 | 0.657 |
| CERES | 0.562 | 0.619 | 0.639 | 0.689 | 0.729 | **0.759** | 0.745 | 0.719 | 0.699 | 0.711 | 0.738 | 0.744 | 0.723 | 0.697 | 0.762 | 0.788 |
| r3 − CERES | −0.142 | −0.168 | −0.165 | −0.156 | −0.150 | −0.140 | −0.117 | −0.073 | −0.036 | −0.043 | −0.072 | −0.085 | −0.054 | −0.032 | −0.126 | −0.131 |

| lat (°S) | 32.1 | 37.7 | 40.5 | 46.0 | 51.6 | 57.2 | 60.0 | 62.8 | 65.6 | 68.4 | 71.2 | 73.9 | 76.7 | 79.5 | 85.1 | 87.9 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r3 | 0.369 | 0.455 | 0.510 | 0.600 | 0.647 | 0.663 | 0.670 | 0.670 | 0.646 | 0.622 | 0.604 | 0.569 | 0.543 | 0.562 | 0.533 | 0.600 |
| CERES | 0.645 | 0.750 | 0.789 | 0.845 | 0.884 | 0.923 | 0.930 | 0.927 | 0.894 | 0.816 | 0.732 | 0.642 | 0.546 | 0.478 | 0.515 | 0.541 |
| r3 − CERES | −0.276 | −0.295 | −0.279 | −0.245 | −0.237 | −0.260 | −0.260 | −0.257 | −0.248 | −0.194 | −0.128 | −0.073 | −0.003 | +0.084 | +0.018 | +0.059 |

- **The model is short everywhere from 30° to 77° in both hemispheres.**
  - North: by 0.14–0.17 from 32° to 57°N, falling to 0.04 at 65–68°N.
  - South: by 0.24–0.30 from 32° to 66°S.
- **The northern cap is below the observations:** −0.03 to −0.09 at 65–80°N and −0.13 poleward of
  82°N. So the deficit is the mid-latitudes' (0.15) and not the cap's. That matches the owner's
  reading, and **(a) passes** (the stop condition does not apply).
- CERES has its northern maximum (0.759) at 57.2°N, a dip at 65.6°N (0.699) and a second rise
  over the Arctic. The model has no northern maximum because its mid-latitudes are too clear,
  not because its cap is too cloudy.
- Figure `docs/figures/P4/q13_eis_stepA_cover.png`:
  - CERES (black) lies above r3 (blue) everywhere north of 30°N and between 30° and 76°S. The
    largest gap is over the Southern Ocean: 0.93 against 0.67 at 60°S.
  - The offline EIS estimate (orange dashes) leaves the r3 curve by at most 0.03 (Southern
    Ocean) and about 0.015 at 68–75°N.

### (b) EIS law against the LTS law, offline

Mean over open ocean (land fraction < 0.5) in each region, values weighted by f_ocn. "Low" is
the column's largest cloud fraction at σ ≥ 0.8, as is now and as estimated with EIS in place of
LTS.

| region | season | LTS (K) | EIS (K) | LTS law | EIS law | low now → EIS |
|---|---|---|---|---|---|---|
| Peru | annual / SON | 19.0 / 21.6 | 4.6 / 7.5 | 0.52 / 0.67 | 0.41 / 0.59 | 0.45 → 0.37 / 0.61 → 0.54 |
| Namibia | annual / JJASON | 18.4 / 19.7 | 3.9 / 5.6 | 0.49 / 0.57 | 0.38 / 0.48 | 0.42 → 0.35 / 0.51 → 0.44 |
| California | annual / JJA | 18.4 / 22.0 | 4.4 / 7.4 | 0.49 / 0.70 | 0.40 / 0.58 | 0.36 → 0.30 / 0.64 → 0.54 |
| Canary | annual / JJA | 16.4 / 19.4 | 1.5 / 4.0 | 0.38 / 0.55 | 0.24 / 0.38 | 0.33 → 0.25 / 0.48 → 0.37 |
| Australia | annual / DJF | 13.6 / 16.3 | 0.6 / 2.5 | 0.24 / 0.37 | 0.19 / 0.29 | 0.14 → 0.13 / 0.24 → 0.21 |
| trades 5–20°N | annual | 12.5 | −2.9 | 0.15 | 0.03 | 0.18 → 0.16 |
| trades 5–20°S | annual | 13.0 | −2.1 | 0.19 | 0.08 | 0.19 → 0.17 |
| warm pool | annual | 11.3 | −4.4 | 0.08 | 0.00 | 0.18 → 0.18 |
| N Pacific 40–60°N | annual / DJF / JJA | 13.8 / 6.6 / 22.8 | 4.2 / −1.1 / 11.0 | 0.33 / 0.10 / 0.71 | 0.41 / 0.18 / 0.74 | 0.32 → 0.34 / 0.19 → 0.20 / 0.61 → 0.63 |
| N Atlantic 40–60°N | annual / DJF / JJA | 12.2 / 6.5 / 18.4 | 1.7 / −2.5 / 6.3 | 0.24 / 0.06 / 0.48 | 0.28 / 0.11 / 0.50 | 0.23 → 0.25 / 0.17 → 0.18 / 0.37 → 0.38 |
| Southern Ocean 45–65°S | annual / DJF / JJA | 11.5 / 15.2 / 8.5 | 3.5 / 6.4 / 1.2 | 0.18 / 0.31 / 0.09 | **0.35 / 0.51 / 0.23** | 0.21 → 0.26 / 0.28 → 0.36 / 0.18 → 0.21 |
| open ocean > 65°N | annual / DJF / JJA | 13.0 / 4.9 / 24.2 | 6.2 / −0.6 / **15.6** | 0.12 / 0.01 / 0.32 | 0.18 / 0.05 / **0.42** | 0.17 → 0.21 / 0.09 → 0.10 / 0.31 → 0.37 |
| open ocean > 65°S | annual / DJF / JJA | 9.5 / 15.2 / 6.1 | 3.7 / 8.5 / 0.9 | 0.07 / 0.18 / 0.01 | 0.16 / 0.37 / 0.05 | 0.16 → 0.20 / 0.20 → 0.30 / 0.13 → 0.14 |

All four seasons for every region are in the `.txt` file. W&B's own North Pacific and North
Atlantic boxes: annual EIS 4.7 and 2.8 K (their Table 1: 6.7 and 7.2 K), EIS law 0.47 and 0.34.

Zonal means (land counts as 0 for the laws; cover = total cover estimated from the snapshots,
random overlap of the change):

| band | LTS law | EIS law | low now → EIS | cover now → EIS (central / lower estimate) | CERES |
|---|---|---|---|---|---|
| **68–80°N** | 0.093 | 0.146 (+0.053) | 0.166 → 0.197 (+0.031) | 0.662 → 0.675 / 0.675 (**+0.013**) | 0.724 |
| **45–65°N** | 0.121 | 0.160 (+0.039) | 0.186 → 0.200 (+0.014) | 0.587 → 0.593 / 0.593 (**+0.006**) | 0.725 |
| 45–65°S | 0.176 | 0.342 (+0.166) | 0.212 → 0.258 | 0.645 → 0.664 / 0.661 | 0.893 |
| 20°S–20°N | 0.129 | 0.039 | 0.171 → 0.158 | 0.606 → 0.597 | 0.652 |
| global | 0.137 | 0.120 | 0.157 → 0.158 | 0.547 → 0.545 | 0.674 |

- **(b) fails: EIS does not raise 45–65°N relative to the cap.** It raises 68–80°N by twice as
  much (+0.013 against +0.006 in cover; +0.053 against +0.039 in the law alone). The 40–70°
  maximum would stay on the 68.4° edge row (0.679 at 68.4° and 71.2° against 0.668 at 65.6°).
- The result does not depend on the uncertain parts:
  - The lower estimate (layers set by the RH cloud left unchanged) gives the same band numbers.
  - EIS shifted by +1.2 K everywhere (the model's own surface RH, or the Table 1 gap) gives
    +0.010 against +0.018.
- **Why.** EIS = LTS − Γ_m(z700 − LCL), and Γ_m is small at cold temperatures. EIS therefore
  rises most where the surface is coldest, which is the summer Arctic Ocean (JJA EIS 15.6 K,
  law 0.42). In 45–65°N the oceans are warmer and in winter unstable (DJF EIS −1 to −2.5 K).
- **What EIS does as W&B describe it** (record):
  - it removes the trade and warm-pool stratus (law 0.15–0.19 → 0.03–0.08, warm pool 0.08 → 0);
  - it lowers the decks by 0.06–0.14;
  - it doubles the Southern Ocean law (0.18 → 0.35).
  The Southern Ocean low cloud rises by only 0.05 and total cover by 0.02 there, against a
  deficit of 0.25.
- **Size of the lever.** Even where the law rises, the zonal cover moves by 0.006–0.02. The
  model's mid-latitude deficit (0.14–0.17 N, 0.24–0.30 S) is total cover, and one low layer per
  column cannot carry it.
- Figure `docs/figures/P4/q13_eis_stepA_laws.png`:
  - Maps: EIS is largest (> 10 K) over the summer-ice-free Arctic, Hudson Bay and the Barents
    Sea, and along the eastern-boundary coasts. It is negative over the whole tropical ocean.
  - The EIS-law map shows a 0.3–0.5 band at 50–65°S and the decks, and nothing in the tropics.
  - The zonal panel: the EIS law peaks at 0.37 at 57°S. In the north it has two humps (0.20 at
    45°N, 0.19 at 74°N), and the estimated low cloud with EIS peaks at 74°N (0.22).

**Stop condition met: step A (b) does not raise 45–65°N relative to the cap.** Steps B–E were
not started. There is no code change in `crates/` and no knob change. `docs/tuning.md` is
unchanged, so `scripts/check` and `scripts/regress` were not rerun. The decision is recorded
under question 13, and options are below it.

## 2026-11-26 (day) — Question 13, owner decision after step A: diagnosis of the mid-latitude deficit; **stopped at step 3, no toggle restores mid-latitude cover** (branch `p4-clouds`)

Owner decision of 2026-11-26 (follows ee43b8e), recorded under question 13 here and in the
history file before starting: EIS not adopted in P4 (P5 candidate), polar air refused for now;
option 3, diagnosis, with limited authority to act (steps 1–4). Machine: 16 threads, all runs at
≤ 16 in total.

### Step 1 — observed vertical split (CALIPSO-GOCCP)

**Fetched:** CALIPSO-GOCCP v3.1.2 (Chepfer et al. 2010, JGR 115, D00H16), file
`MapLowMidHigh330m_200606-201803_avg_CFMIP2_sat_3.1.2.nc` (140 monthly means, June 2006 – March
2018, day + night, 2° grid; variables `cllcalipso`, `clmcalipso`, `clhcalipso`, `cltcalipso`)
from climserv.ipsl.polytechnique.fr/cfmip-obs/data/GOCCP_v3/2D_Maps/grid_2x2xL40/climato/, into
`reference/raw/goccp/` (git-ignored). Classes as in the product: low p > 680 hPa, middle
440–680, high < 440 hPa. ISCCP was not fetched.

**Tool** `tools/cloud_split.py` (new): model cover of each class by maximum-random overlap of
its levels (p = σ p_s; the overlap of the radiation). A lidar does not see low cloud under
opaque upper cloud, so the model's low and middle cover are given raw (all cloud in the class)
and "exclusive" (cloud in the class with none above). The observed value should lie between the
two; no lidar simulator is used. Regions over all longitudes (land included).

r3 T42 gate run (years 2–6, 184 snapshots; that run saved `cl`, not `cl_split`; the T21 runs
below have both). Annual means:

| band | low raw / excl. | GOCCP low | mid raw / excl. | GOCCP mid | high | GOCCP high | total (snapshots) | GOCCP / CERES total |
|---|---|---|---|---|---|---|---|---|
| 30–45°N | 0.219 / 0.115 | 0.306 | 0.117 / 0.033 | 0.182 | 0.311 | 0.345 | 0.459 | 0.628 / 0.613 |
| 45–60°N | 0.304 / 0.099 | 0.393 | 0.291 / 0.071 | 0.270 | 0.410 | 0.344 | 0.580 | 0.724 / 0.726 |
| 60–77°N | 0.324 / 0.072 | 0.436 | 0.409 / 0.084 | 0.291 | 0.503 | 0.259 | 0.659 | 0.712 / 0.720 |
| 32–57°N | 0.250 / 0.112 | 0.344 | 0.175 / 0.047 | 0.223 | 0.341 | 0.349 | 0.500 | 0.674 / 0.653 |
| 30–45°S | 0.267 / 0.148 | 0.478 | 0.107 / 0.031 | 0.151 | 0.281 | 0.313 | 0.460 | 0.730 / 0.737 |
| 45–60°S | 0.433 / 0.127 | 0.599 | 0.352 / 0.086 | 0.275 | 0.429 | 0.324 | 0.642 | 0.869 / 0.889 |
| 60–77°S | 0.256 / 0.037 | 0.512 | 0.437 / 0.083 | 0.321 | 0.498 | 0.279 | 0.619 | 0.777 / 0.790 |

- **The missing cloud is low cloud.** Even the raw low cover, which is an upper bound for what
  a lidar would see, is below GOCCP in every band: by 0.09 at 32–57°N and 45–60°N, by 0.11 at
  60–77°N, by 0.17–0.26 in the south.
- Middle and high cloud are at or above GOCCP poleward of 45° (60–77°N high 0.50 against 0.26;
  45–60°S middle 0.35 against 0.28). So the total-cover deficit is low cloud, partly hidden in
  the total by too much upper cloud.
- In winter the low deficit is larger: 32–57°N DJF 0.236 against 0.369; in JJA 0.309 against
  0.330.
- The total from the snapshots (0.459 at 30–45°N) matches the online `clt` (0.456).
- Figure `docs/figures/P4/q13_diag_step1_classes.png`:
  - Low panel: the GOCCP curve (black) lies above the model's raw low curve in all seven
    bands; the gap is largest at 30–45°S and 60–77°S.
  - Middle and high panels: the model's curves rise poleward of 45° above GOCCP, most at
    60–77°N in high cloud (0.50 against 0.26).
  - The exclusive low cover (dashed) is only 0.04–0.15: most model low cloud lies under upper
    cloud.

### Step 2 — freeze-dry against CAM `cloud_fraction.F90`

Source: ESCOMP/CAM `cam_cesm2_2_rel`, `src/physics/cam/cloud_fraction.F90`, subroutine
`cldfrc` (copy fetched 2026-11-22, read again here; compared only, nothing copied).

| item | CAM `cldfrc` | ours before | match |
|---|---|---|---|
| threshold | q / 0.0030 | `freeze_dry_q` 3e-3 kg/kg | yes |
| floor | max(0.15, min(1, q/0.003)) | `freeze_dry_min` 0.15, same form | yes |
| levels | low-cloud block, `pmid ≥ premib` (750 hPa for CAM4 physics, `namelist_defaults_cam.xml`), the lowest level included | σ ≥ 0.75 (`freeze_dry_sigma`) | **no** (pressure, not σ; differs over high ground) |
| surface types | every column (the land/snow test only changes `rhlim`) | every column | yes |
| what it multiplies | `rhcloud` only; afterwards `cloud = max(rhcloud, cldst)` + convective cloud | the layer's stratiform cloud *after* the stratus / inversion cloud was combined by maximum | **no** (our factor also reduced the LTS stratus) |
| switch | namelist `cldfrc_freeze_dry` | `freeze_dry_q = 0` | — |

**Correction made (step 4, first limit):** freeze-dry now multiplies the RH cloud only, before
the stratus or Slingo inversion cloud is combined with it, and applies at p ≥
`freeze_dry_pressure` = 75 000 Pa (key renamed from `freeze_dry_sigma`). With `low_cloud =
"rh"` (the P4 frozen default) only high-ground levels change. SCM tests:
`freeze_dry_rh_cloud_only_and_by_pressure` (new: stratus above the reduced RH cloud is kept
whole; a column with p_s = 700 hPa is not reduced), `lts_stratus_cap_gate_and_combination`
(updated: the capped stratus is no longer reduced), `freeze_dry_column` (unchanged, passes).
Tuning-log entry 2026-11-26. Doc comments in `clouds/mod.rs` and `clouds/low.rs` updated.

Also new, for step 3 only: `[clouds] low_gate` (default true; false drops the Slingo eq. (7)
factor; diagnostic switch, byte-identical when not set; SCM test
`low_gate_off_keeps_rh_cloud_under_subsidence`).

### Step 3 — toggle runs (T21 Earth AMIP 1 + 1 years from r3, year 2; diagnostics only)

Cases `cases/bench/p4_q13diag/`, runs `out/p4/q13diag/` (4 × 4 threads, then 2 × 8). All r3
(`slingo_lts`, `low_rh = "sundqvist"`, `water_max` 1.5e-4, `rh_crit_upper` 0.80), with
`cre_split` for `cl_split` and 5-daily snapshots of `cl`, `cl_split`, `q`, `rh`, `t`, `omega`.
a–d with the binary before the step 2 correction (`out/p4/toyclim_q13diag`; a and b identical
to r3's code), e and f after it (`out/p4/toyclim_q13fd`; e has freeze-dry off, so the correction
does not act there).

- a: r3. b: freeze-dry off. c: gate off (stability law kept). d: both off.
- e: d (it restored more than c) with `rh_crit_surface` 0.85.
- f (free; reason stated before running): steps 1 and 3 place the deficit in low-level RH
  below RH_c (median RH 0.67–0.80 at σ ≥ 0.87 against RH_c 0.92–0.95, table below), so the only
  lever inside the step 4 limits is RH_c near the surface, with the corrected freeze-dry: r3 +
  correction + `rh_crit_surface` 0.75 (the bottom of its logged range 0.75–0.95).

| run | albedo | OLR | net TOA | SW / LW CRE | cover | 20°S–20N net CRE | 30–45°N | 45–60°N | 60–77°N | **32–57°N** | 30–45°S | 45–60°S | 60–77°S |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| a r3 | 0.300 | 233.9 | +4.31 | −52.0 / +27.2 | 0.543 | −38.4 | −0.155 | −0.166 | −0.073 | **−0.170** | −0.296 | −0.285 | −0.192 |
| b fd off | 0.307 | 233.9 | +2.14 | −54.3 / +27.8 | 0.555 | −40.0 | −0.160 | −0.149 | −0.038 | **−0.166** | −0.264 | −0.257 | −0.160 |
| c gate off | 0.331 | 232.5 | −4.96 | −62.8 / +28.8 | 0.604 | −54.1 | −0.124 | −0.136 | −0.013 | **−0.147** | −0.227 | −0.241 | −0.152 |
| d both off | 0.333 | 232.5 | −5.53 | −63.4 / +28.9 | 0.611 | −54.8 | −0.128 | −0.108 | +0.020 | **−0.136** | −0.233 | −0.230 | −0.119 |
| e d + RH_s 0.85 | 0.342 | 232.1 | −8.30 | −66.5 / +29.2 | 0.624 | −59.7 | −0.112 | −0.099 | +0.029 | **−0.123** | −0.237 | −0.223 | −0.120 |
| f fd corr. + RH_s 0.75 | 0.308 | 233.6 | +1.79 | −54.9 / +27.7 | 0.558 | −42.7 | −0.139 | −0.144 | −0.045 | **−0.152** | −0.286 | −0.272 | −0.165 |
| CERES (cover) | 0.291 | | | −45.2 / +25.8 | 0.67 | −14.6 | 0.597 | 0.724 | 0.721 | 0.664 | 0.709 | 0.886 | 0.801 |

Band columns: online total cover minus CERES EBAF Ed4.1 (T21 grid; CERES values in the last
row). Criteria 1–2 (albedo 0.27–0.33, OLR 228–250, net ±8; SW −30 to −60, LW +15 to +35): a, b
and f meet both; c and d fail albedo (0.331, 0.333) and SW CRE (−62.8, −63.4); e fails albedo,
net TOA (−8.30) and SW CRE.

**No run brings 32–57°N within 0.08 of CERES.** The best is e at −0.123 (outside the limits and
failing criteria 1–2); the only run inside the limits, f, is at −0.152 (+0.018 on r3). Every
toggle raises 60–77°N more than 32–57°N (d: +0.093 against +0.034), so the northern maximum
would move further poleward, not into the mid-latitudes.

Low cover against GOCCP (raw / exclusive; GOCCP 30–45°N 0.306, 45–60°N 0.393, 60–77°N 0.436,
30–45°S 0.478, 45–60°S 0.599, 60–77°S 0.512):

| run | 30–45°N | 45–60°N | 60–77°N | 32–57°N | 30–45°S | 45–60°S | 60–77°S |
|---|---|---|---|---|---|---|---|
| a | 0.179 / 0.081 | 0.290 / 0.081 | 0.312 / 0.056 | 0.234 / 0.084 | 0.185 / 0.092 | 0.394 / 0.090 | 0.232 / 0.028 |
| b | 0.183 / 0.089 | 0.334 / 0.097 | 0.406 / 0.069 | 0.251 / 0.098 | 0.203 / 0.103 | 0.431 / 0.100 | 0.352 / 0.053 |
| c | 0.227 / 0.114 | 0.346 / 0.111 | 0.424 / 0.091 | 0.277 / 0.112 | 0.239 / 0.130 | 0.467 / 0.122 | 0.337 / 0.061 |
| d | 0.220 / 0.110 | 0.386 / 0.135 | 0.521 / 0.124 | 0.289 / 0.121 | 0.241 / 0.133 | 0.490 / 0.128 | 0.461 / 0.087 |
| e | 0.241 / 0.124 | 0.415 / 0.144 | 0.537 / 0.134 | 0.316 / 0.133 | 0.256 / 0.148 | 0.500 / 0.141 | 0.458 / 0.099 |
| f | 0.188 / 0.083 | 0.322 / 0.095 | 0.373 / 0.070 | 0.250 / 0.093 | 0.196 / 0.100 | 0.409 / 0.099 | 0.332 / 0.043 |

Middle and high cover hardly move between runs (mid 32–57°N 0.209–0.220, high 0.336–0.353;
60–77°N high 0.50–0.54 in all, GOCCP 0.26).

**Cloud fraction by level** (all longitudes; total / stratiform-only from `cl_split`), a → b, c,
d, e, f, at the four lowest cloudy levels:

| region, season | σ | a | b | c | d | e | f |
|---|---|---|---|---|---|---|---|
| 40–60°N DJF (winter) | 0.787 | 0.21 / 0.18 | 0.29 / 0.25 | 0.22 / 0.19 | 0.29 / 0.26 | 0.33 / 0.30 | 0.27 / 0.24 |
| | 0.867 | 0.10 / 0.06 | 0.10 / 0.06 | 0.19 / 0.16 | 0.22 / 0.18 | 0.27 / 0.23 | 0.11 / 0.07 |
| | 0.930 | 0.05 / 0.02 | 0.06 / 0.02 | 0.10 / 0.07 | 0.09 / 0.05 | 0.13 / 0.09 | 0.07 / 0.03 |
| | 0.971 | 0.04 / 0.00 | 0.04 / 0.00 | 0.05 / 0.01 | 0.05 / 0.01 | 0.06 / 0.02 | 0.05 / 0.01 |
| 40–60°N JJA (summer) | 0.787 | 0.21 / 0.20 | 0.18 / 0.17 | 0.18 / 0.17 | 0.19 / 0.18 | 0.19 / 0.18 | 0.20 / 0.19 |
| | 0.867 | 0.08 / 0.07 | 0.07 / 0.06 | 0.20 / 0.18 | 0.19 / 0.18 | 0.20 / 0.19 | 0.08 / 0.07 |
| | 0.930 | 0.06 / 0.05 | 0.08 / 0.07 | 0.19 / 0.18 | 0.19 / 0.17 | 0.22 / 0.21 | 0.09 / 0.08 |
| | 0.971 | 0.08 / 0.06 | 0.08 / 0.07 | 0.15 / 0.14 | 0.15 / 0.13 | 0.18 / 0.17 | 0.08 / 0.06 |
| 40–60°S JJA (winter) | 0.787 | 0.25 / 0.20 | 0.28 / 0.23 | 0.26 / 0.21 | 0.30 / 0.25 | 0.29 / 0.25 | 0.25 / 0.20 |
| | 0.867 | 0.12 / 0.06 | 0.12 / 0.07 | 0.19 / 0.14 | 0.19 / 0.13 | 0.20 / 0.14 | 0.12 / 0.06 |
| | 0.930 | 0.07 / 0.01 | 0.07 / 0.01 | 0.10 / 0.04 | 0.09 / 0.04 | 0.09 / 0.04 | 0.07 / 0.02 |
| | 0.971 | 0.06 / 0.00 | 0.06 / 0.00 | 0.06 / 0.01 | 0.07 / 0.01 | 0.06 / 0.01 | 0.06 / 0.00 |
| 40–60°S DJF (summer) | 0.867 | 0.15 / 0.14 | 0.16 / 0.15 | 0.33 / 0.32 | 0.33 / 0.32 | 0.36 / 0.34 | 0.15 / 0.13 |
| | 0.930 | 0.07 / 0.06 | 0.08 / 0.07 | 0.25 / 0.24 | 0.24 / 0.22 | 0.26 / 0.24 | 0.09 / 0.07 |
| 68–80°N DJF (winter) | 0.787 | 0.19 / 0.18 | **0.52 / 0.51** | 0.21 / 0.21 | 0.51 / 0.50 | 0.48 / 0.48 | 0.32 / 0.31 |
| | 0.867 | 0.07 / 0.05 | 0.13 / 0.12 | 0.24 / 0.23 | 0.50 / 0.48 | 0.50 / 0.49 | 0.09 / 0.08 |
| | 0.930 | 0.04 / 0.03 | 0.08 / 0.07 | 0.17 / 0.16 | 0.36 / 0.34 | 0.38 / 0.36 | 0.06 / 0.04 |
| 68–80°N JJA (summer) | 0.867 | 0.09 / 0.08 | 0.09 / 0.09 | 0.23 / 0.23 | 0.25 / 0.25 | 0.28 / 0.27 | 0.08 / 0.07 |
| | 0.930 | 0.06 / 0.05 | 0.10 / 0.09 | 0.25 / 0.24 | 0.26 / 0.24 | 0.31 / 0.29 | 0.11 / 0.10 |

The lowest level (σ 0.993) has 0.00–0.07 everywhere (no RH cloud there under "slingo_lts";
what remains is convective).

**RH at the low levels against RH_c**, run a (b–f within ±0.02 in the median, ±0.06 in the
share above RH_c). RH_c of r3 (`rh_crit_surface` 0.95, `rh_crit_upper` 0.80): 0.910 at σ 0.787,
0.923 at 0.867, 0.936 at 0.930, 0.944 at 0.971, 0.949 at 0.993.

| region, season | σ | RH mean | RH q10 / q50 / q90 | share RH > RH_c | share q < 3 g/kg | share ω ≥ 0 |
|---|---|---|---|---|---|---|
| 40–60°N DJF | 0.787 | 0.80 | 0.48 / 0.84 / 1.01 | 0.33 | 0.86 | 0.50 |
| | 0.867 | 0.82 | 0.67 / 0.80 / 1.00 | 0.27 | 0.72 | 0.51 |
| | 0.930 | 0.76 | 0.64 / 0.73 / 0.95 | 0.11 | 0.62 | 0.51 |
| | 0.971 | 0.70 | 0.59 / 0.69 / 0.85 | 0.03 | 0.55 | 0.51 |
| | 0.993 | 0.70 | 0.59 / 0.69 / 0.84 | 0.02 | 0.50 | 0.50 |
| 40–60°N JJA | 0.867 | 0.74 | 0.40 / 0.76 / 1.00 | 0.24 | 0.03 | 0.43 |
| | 0.930 | 0.75 | 0.39 / 0.80 / 1.00 | 0.25 | 0.00 | 0.43 |
| | 0.971 | 0.74 | 0.35 / 0.79 / 0.99 | 0.19 | 0.00 | 0.40 |
| 40–60°S JJA | 0.867 | 0.80 | 0.71 / 0.77 / 0.99 | 0.16 | 0.51 | 0.47 |
| | 0.930 | 0.73 | 0.65 / 0.71 / 0.86 | 0.05 | 0.38 | 0.46 |
| | 0.971 | 0.68 | 0.60 / 0.67 / 0.77 | 0.01 | 0.29 | 0.45 |
| | 0.993 | 0.68 | 0.61 / 0.67 / 0.76 | 0.01 | 0.23 | 0.43 |
| 40–60°S DJF | 0.867 | 0.85 | 0.70 / 0.87 / 1.01 | 0.39 | 0.18 | 0.45 |
| | 0.930 | 0.84 | 0.69 / 0.85 / 1.00 | 0.29 | 0.06 | 0.44 |
| | 0.971 | 0.78 | 0.65 / 0.77 / 0.96 | 0.11 | 0.02 | 0.42 |
| 68–80°N DJF | 0.867 | 0.89 | 0.69 / 0.96 / 1.01 | 0.55 | 0.98 | 0.47 |
| | 0.930 | 0.89 | 0.70 / 0.96 / 1.01 | 0.53 | 0.95 | 0.46 |
| | 0.971 | 0.85 | 0.66 / 0.88 / 1.00 | 0.31 | 0.93 | 0.45 |

What the profiles show:
- **The mid-latitude boundary layer is too dry for the RH cloud, whatever the switches.** In
  winter (40–60°N DJF, 40–60°S JJA) the median RH at σ 0.93–0.99 is 0.67–0.73 and the 90th
  percentile 0.76–0.95, so only 1–11 % of points exceed RH_c. Even with `rh_crit_surface` 0.75 (f;
  RH_c 0.76 at σ 0.97) the winter low cloud below σ 0.9 stays at 0.05–0.07. For comparison,
  the surface RH over the ocean that Wood & Bretherton (2006) quote from COADS is 0.80 ± 0.07
  (60°S–60°N); the model's lowest level over the ocean there is 0.70 ± 0.08 (step A, 2026-11-26).
- **Freeze-dry is a high-latitude lever, not a mid-latitude one.** It acts where q < 3 g/kg:
  91–98 % of points at 68–80°N in winter, where switching it off raises σ 0.79 from 0.19 to 0.52.
  At 40–60°N in winter the share is 50–62 % at σ ≥ 0.93, but the RH there is mostly below RH_c
  anyway, so b adds 0.00–0.08 per level and +0.004 to 32–57°N.
- **The gate removes about half the potential RH low cloud everywhere** (ω ≥ 0 at 40–51 % of
  points at all latitudes). Switching it off doubles σ 0.87–0.93 cloud in mid-latitude summer
  (0.07 → 0.20) and over the Southern Ocean in summer (0.15 → 0.33). But it costs 15–16 W m⁻² of
  tropical net CRE (−38 → −54; CERES −14.6) and breaks criteria 1–2. It is outside the limits,
  and in the zonal mean it still gives only +0.023 at 32–57°N.
- **The corrected freeze-dry (f)** acts mainly at 68–80°N in winter (σ 0.79: 0.19 → 0.32),
  where the stability stratus is no longer reduced. Its global effect at T21 with
  `rh_crit_surface` 0.75 is +0.015 in cover and −4.3 W m⁻² in tropical net CRE (−42.7).

Figures:
- `docs/figures/P4/q13_diag_step3_classes.png`:
  - Low panel: every run stays below GOCCP at 30–45°N and in all southern bands.
  - d and e (red, purple) pass GOCCP at 60–77°N (0.52–0.54 against 0.44), i.e. the switches add
    low cloud mainly at high latitude.
  - Middle and high panels: the six runs lie on top of each other, above GOCCP poleward of 45°.
- `docs/figures/P4/q13_diag_step3_profiles.png`:
  - 40–60°N and 40–60°S winter: below σ 0.9 all curves stay under 0.13. At σ 0.87 the gate-off
    runs add 0.07–0.17.
  - Summer, both hemispheres: the gate-off runs (c, d, e) carry 0.10–0.36 at σ 0.87–0.97,
    against 0.03–0.15 with the gate.
  - 68–80°N winter: the freeze-dry-off runs (b, d, e) have 0.5 at σ 0.79–0.87; f lies between a
    and b at σ 0.79.

### Stop

**Stop condition met: no toggle restores mid-latitude cover** (best 32–57°N −0.123 in e, against
the target of within 0.08; inside the limits, f, −0.152). The lever that comes closest (gate off,
plus freeze-dry off and a lower RH_c) is outside the step 4 limits, fails criteria 1–2 and moves
the cover poleward. The T42 screen (step D) and the gate (step E) were not started.

Changes kept:
- The step 2 correction (freeze-dry on the RH cloud only, at p ≥ 750 hPa). This is a correction
  under the step 4 limits; with the P4 frozen default it changes only high ground.
- The diagnostic switch `low_gate` and `tools/cloud_split.py`.
- `scripts/check` passes (fmt, clippy, 138 tests).
- `scripts/regress --against ee43b8e` (P0 list, 1 and 8 threads): PASS, all six cases
  byte-identical; none of them uses the diagnostic clouds. 10-day byte checks of the cloud cases
  (`out/p4/q13diag/bytecheck/`):
  - `aquaplanet_t21` and `ape_control_t21`: identical, before against after the correction;
  - `earth_amip_t21` (P4 frozen defaults): 241 of 369 files differ, as expected from the
    pressure bound over high ground. The frozen gate numbers were made with the old bound and
    were not rerun;
  - the `low_gate` binary against r3's binary (923fb1c), `earth_amip_t21`: identical.

Options are below question 13.


## 2026-11-26 (afternoon) — Question 13, owner decision after the diagnosis: boundary-layer humidity; **diagnosis done, stopped for the owner as instructed** (branch `p4-clouds`)

Owner decision (follows 71aa0a8; recorded under question 13 and in the history file): push the
branch first (done). The freeze-dry correction is accepted; the frozen-knob gate numbers of
2026-11-21 predate it and are not rerun. The low-cloud deficit comes from a too-dry boundary
layer, not from the cloud scheme. Options 2 and 3 were refused, and option 4 was not ruled.
Option 1 is taken and widened. **Plan change:** the convection item that was to be first in P5
is done now, inside P4, and P5 starts after P4 closes with the final convection and clouds
(SPEC.md not edited, criteria unchanged). r3 with the corrected freeze-dry is the working cloud
configuration, with no more cloud tuning until the humidity question is answered. Steps 1–4
are diagnosis only; then stop and report. Nothing fixed, no default changed.

**Tools and switches (all diagnostic, off by default; byte-neutral):**
- `physics/budget.rs`, `[convection] budget_sigma`: moisture-budget probes after the boundary
  layer, the convection and the large-scale condensation. They record the accumulated q
  tendency of the levels σ ≥ 0.87 and of the lowest level, plus W, the lowest-level RH and the
  convection top. SCM test: the probe changes nothing and reports the layer integral. 1-day
  Earth run with and without the probes: 20 field and 224 restart files identical.
- `[convection] keep_lowest = N`: deep convection leaves the lowest N levels unchanged, with
  the closure over the remaining levels. It is not a published variant; no existing switch
  does this. SCM test: the bottom levels are untouched, enthalpy is conserved and the water
  lost equals the precipitation.
- `tools/bl_humidity.py`: observations by band, the budget and the toggle table.
- Binary `out/p4/toyclim_q13bl` (71aa0a8 plus the above). Runs are in `out/p4/q13bl/`, with
  reports `step123.txt` and `step123.json`.

### Step 1 — observations (fetched 2026-11-26 into `reference/raw/`, monthly climatologies)

Fetched:
- **ICOADS** 2° enhanced `rhum.mean.nc` (NOAA PSL; Freeman et al. 2017), averaged over
  1991–2020. It holds ship and buoy reports at about 10–20 m height, ocean only.
- **NCEP/NCAR** RH at σ 0.995 (`rhum.sig995.mon.ltm.1991-2020.nc`). This level is close to
  our lowest, σ 0.9925.
- **NCEP/NCAR** latent heat flux (`lhtfl`), giving E = LHF / 2.5e6.
- **NCEP/NCAR** `prate` and `cprat`, for the convective share.
- **GPCP v2.3** (already on disk).

Not fetched:
- **OAFlux:** the WHOI ftp did not answer, so evaporation is reanalysis model output.
- **An observed convective share:** the NCEP share comes from its own convection scheme.

Ocean cells only (OISST mask). The model RH is the lowest level at the start of each physics
step. Model values are year 2 of run a (r3, corrected freeze-dry).

**Global precipitation:** GPCP **2.69** mm/day (NCEP 2.82). The model gives **2.92** at T21
(run a; +8.5 %) and **3.20** at T42 (r3 gate; +19 %). The owner's 2.7 is confirmed.

| Band (ocean) | RH ICOADS / NCEP σ0.995 / model, annual | RH in winter (40–60°: DJF N, JJA S) | E NCEP / model, winter [mm/day] | P GPCP / model, annual | Convective share NCEP / model, annual |
|---|---|---|---|---|---|
| 10°S–10°N | 0.807 / 0.811 / **0.721** | — | 3.79 / 3.86 (annual) | 4.42 / 4.89 | 0.99 / 0.82 |
| 10–30°N | 0.786 / 0.791 / **0.704** | — | 4.70 / 5.05 (annual) | 2.51 / 3.38 | 0.97 / 0.72 |
| 10–30°S | 0.773 / 0.804 / **0.726** | — | 4.67 / 4.39 (annual) | 2.20 / 2.07 | 0.94 / 0.65 |
| 40–60°N | 0.821 / 0.850 / **0.747** | 0.802 / 0.843 / **0.700** | 2.73 / **3.64** | 3.38 / 3.02 | 0.29 / **0.57** (DJF 0.29 / 0.68) |
| 40–60°S | 0.836 / 0.864 / **0.706** | 0.819 / 0.864 / **0.673** | 1.85 / **2.99** | 2.99 / 2.92 | 0.22 / **0.60** (JJA 0.26 / 0.66) |

- **RH:** near-surface ocean RH is 0.05–0.13 below ICOADS everywhere. The gap is largest in
  the winter mid-latitudes (−0.10 and −0.15), where the summer values match (40–60°N JJA
  0.847 against 0.855).
- **Evaporation:** winter evaporation at 40–60° is 33 % (N) and 62 % (S) above NCEP. Elsewhere
  it is within about 10 %.
- **Precipitation:** too much in the deep tropics and the northern subtropics (2.51 against
  3.38); about right at 40–60° annually.
- **Convective share:** convection makes 57–68 % of the mid-latitude precipitation, against
  NCEP's 16–29 %.

Figure `docs/figures/P4/q13_bl_bands.png` (opened) has three panels, observations in black and
runs a–e dashed:
- **RH panel:** every run lies 0.03–0.15 below ICOADS. The gap widens toward both winter
  mid-latitude bands. Only b and d lift the curve, by about 0.04.
- **Evaporation and precipitation panels:** all runs nearly coincide. They are above the
  observations in the northern subtropics and over the Southern Ocean (evaporation), and above
  them in the deep tropics (precipitation).

### Step 2 — moisture budget of the levels σ ≥ 0.87 (run a, year 2, ocean)

The layer is σ 0.903–1, three levels (σ 0.9295, 0.9705, 0.9925). Terms are in mm/day into the
layer. Mixing is the boundary-layer tendency minus E. Dynamics (advection, hyperdiffusion,
fixer) is the residual against the storage change. The climatology samples every 6 h.

| Term | 40–60°N DJF | 40–60°S JJA | trades 10–25°N | trades 10–25°S |
|---|---|---|---|---|
| surface evaporation | +3.64 | +2.99 | +5.05 | +4.39 |
| boundary-layer mixing (export upward) | **−3.73** | **−3.87** | −3.30 | −2.60 |
| convection | **+0.34** | **+0.92** | −0.75 | −0.67 |
| of it, lowest level only | +0.22 | +0.35 | +0.17 | +0.07 |
| large-scale condensation | −0.01 | −0.01 | −0.05 | −0.09 |
| dynamics (residual) | −0.24 | −0.04 | −0.94 | −1.04 |
| storage | −0.01 | −0.01 | 0 | 0 |
| deep convection active (share of samples) | **0.61** | **0.53** | 0.22 | 0.16 |
| skipped (CAPE > 0, P_q ≤ 0) | 0.10 | 0.09 | 0.65 | 0.67 |
| mean convection top [hPa] | 676 | 651 | 559 | 703 |
| mean boundary-layer depth [m] | 1857 | 2081 | 1279 | 1159 |
| lowest-level RH | 0.700 | 0.673 | 0.704 | 0.726 |
| precipitation / convective share | 4.67 / 0.68 | 3.89 / 0.66 | 3.38 / 0.72 | 2.07 / 0.65 |

- **Parcels** always start at the lowest level (σ 0.9925), by construction of the scheme
  (Frierson 2007 §2d).
- **Mid-latitude winter:**
  - Deep convection is active in 53–61 % of the samples, with shallow tops near 650–680 hPa.
  - Its net tendency in the low layer is **moistening**. It does not dry the layer. It
    removes the water above σ 0.87 (convective precipitation about 3.2 mm/day at 40–60°N
    DJF), and the boundary-layer mixing feeds that from below (export 3.7–3.9 mm/day).
- **Trades:** convection dries the layer (−0.7), and the dynamics export about 1 mm/day.
- **Pinning:** the lowest-level RH in winter at 40–60° (5-day snapshots, ocean) is tightly
  held near `rh_ref`:
  - median 0.69 (N) and 0.67 (S), interquartile 0.66–0.73 and 0.64–0.70, 59–63 % of points
    in 0.65–0.75;
  - with `rh_ref` 0.8 (run b), the median is 0.76 (N) and 0.73 (S).

### Step 3 — toggle runs (T21 Earth AMIP 1 + 1 years, year 2; diagnostics only; 5 runs)

Runs:
- **a:** reference (r3, corrected freeze-dry).
- **b:** `rh_ref` 0.8.
- **c:** `keep_lowest = 2`. A new diagnostic switch, since no existing one does this:
  convection does not modify σ 0.9925 or 0.9705.
- **d:** `ri_crit` 0.25, one change. This is the edge of the logged range 0.25–1. It gives a
  shallower boundary layer with weaker mixing.
- **e (free; reason stated before running):** `deep_requires_pt = true`, the paper's deep
  criterion (Frierson 2007 §2a–b; existing switch). Reason: step 2 showed deep convection in
  53–61 % of winter samples at 40–60°, and a convective share of 0.66–0.68 against NCEP's
  0.16–0.29. The code fires for P_q > 0 regardless of P_T.

Main table:

| Run | Albedo | OLR | Net TOA | SW / LW CRE | Cover | Crit. 1–2 | 20°S–20°N net CRE | Hadley N / S [10¹⁰ kg/s] | P map corr (P5) | P global | RH low, ocean | Low cover 32–57°N / 45–60°S (GOCCP 0.34 / 0.60) | Cover − CERES 32–57°N / 45–60°S |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| a | 0.303 | 233.8 | +3.24 | −53.1 / +27.2 | 0.550 | met | −39.8 | 5.47 / 10.88 | 0.704 | 2.92 | 0.723 | 0.255 / 0.407 | −0.159 / −0.271 |
| b `rh_ref` 0.8 | 0.325 | 232.2 | −2.57 | **−60.9** / +28.8 | 0.593 | **2 fails** (SW by 0.9) | −48.6 | 4.78 / 7.34 | 0.669 | 2.85 | 0.757 | 0.331 / 0.506 | **−0.091** / −0.198 |
| c keep lowest 2 | 0.305 | 233.7 | +2.66 | −53.9 / +27.5 | 0.557 | met | −40.2 | 5.28 / 11.23 | 0.703 | 2.97 | 0.730 | 0.266 / 0.431 | −0.156 / −0.263 |
| d `ri_crit` 0.25 | 0.292 | 235.8 | +5.18 | −49.1 / +26.1 | 0.528 | met | −36.8 | 5.28 / 12.07 | 0.623 | 2.76 | **0.781** | 0.225 / 0.359 | −0.191 / −0.306 |
| e P_T > 0 | 0.308 | 233.0 | +2.46 | −54.9 / +28.3 | 0.561 | met | −40.6 | 5.45 / 10.85 | 0.706 | 2.93 | 0.725 | 0.275 / 0.472 | −0.149 / −0.247 |

Near-surface RH by band against ICOADS, with winter for 40–60°:

| Band | ICOADS | a | b | c | d | e |
|---|---|---|---|---|---|---|
| 10°S–10°N | 0.807 | 0.721 | 0.760 | 0.726 | 0.760 | 0.714 |
| 10–30°N | 0.786 | 0.704 | 0.743 | 0.708 | 0.757 | 0.708 |
| 40–60°N DJF | 0.802 | 0.700 | 0.739 | 0.682 | 0.758 | 0.696 |
| 40–60°S JJA | 0.819 | 0.673 | 0.696 | 0.659 | 0.761 | 0.678 |

Evaporation and precipitation, winter at 40–60°N / 40–60°S, in mm/day:

| Run | Evaporation (NCEP 2.73 / 1.85) | Precipitation (GPCP 4.09 / 3.10) |
|---|---|---|
| a | 3.64 / 2.99 | 4.67 / 3.89 |
| b | 3.01 / 2.95 | 4.38 / 3.94 |
| c | 3.64 / 2.97 | 4.70 / 3.96 |
| d | 3.70 / 2.44 | 4.41 / 3.53 |
| e | 3.63 / 2.94 | 4.60 / 3.90 |

Per-run findings:
- **b** is the only run that raises both the RH and the mid-latitude low cloud. At 32–57°N,
  low cover rises +0.076 and cover − CERES reaches −0.091, against the 0.08 target. It also:
  - lowers the 40–60°N winter evaporation by 17 %;
  - drops the convective share there from 0.68 to 0.49;
  - fails criterion 2 at T21, makes the tropical net CRE 8.8 W m⁻² more negative, weakens the
    Hadley cells (N −13 %, S −33 %) and lowers the P5 pattern correlation by 0.035;
  - leaves the RH still 0.04–0.12 below ICOADS.
- **c:** keeping convection off the two lowest levels changes nothing (32–57°N +0.003), and
  the winter RH is even lower (−0.02).
- **d:** raises the lowest-level RH most (+0.06; 40–60°S JJA 0.761) but lowers the low cloud
  (−0.03 at 32–57°N, −0.05 at 45–60°S) and the precipitation pattern (0.62). Moisture stays in
  the lowest levels, where no RH cloud forms (the lowest level has none by design) and below
  the σ 0.8–0.9 levels where the low cloud sits.
- **e:** cuts the winter convection frequency from 0.61 to 0.51, with small gains (+0.010 at
  32–57°N, +0.024 at 45–60°S). Its RH is unchanged.

### Which hypothesis holds

- **Owner's hypothesis**, that convection fires often and relaxes the boundary layer toward 0.7,
  drying it: **half right.**
  - Right: convection is active in 53–61 % of mid-latitude winter samples. Its precipitation
    share is two to three times NCEP's. The near-surface RH is held at `rh_ref` (median 0.69
    at 0.7, 0.76 at 0.8), and `rh_ref` is the one lever that moves both RH and low cloud.
  - Wrong: the drying does not come from convection acting on the boundary layer directly. In
    winter its tendency there is moistening, and run c, with no convection in the lowest two
    levels, is unchanged.
  - The path is indirect. Convection holds the levels above about σ 0.87 at `rh_ref` and rains
    out what arrives there. The K-profile mixing (boundary layer 1.9–2.1 km deep in winter)
    exports 3.7–3.9 mm/day into those levels. The surface replaces it with too much
    evaporation (+33 % / +62 %).
- **The alternative** (mixing too deep or too strong) controls how the moisture is shared
  between the lowest levels and the cloud levels, not how much there is. A shallower boundary
  layer (d) moistens the lowest level but dries the cloud levels and lowers cover.
- **Conclusion:** the controlling quantity is the convective reference humidity, together with
  how often deep convection fires in the mid-latitude winter.

### Step 4 — sources (compare only; no source copied)

Read:
- **Frierson (2007)**, JAS 64, 1959–1976. Local copy fetched on 2026-11-21, reread.
  - §2d: q_ref = RH_SBM × q_sat(T_ref). Adjustment runs "from the surface to the level of
    zero buoyancy of a parcel lifted from the lowest model level". So the boundary layer is
    always inside the adjusted layer.
  - §3a: the model is an aquaplanet, slab, gray radiation, with no clouds and a K-profile
    boundary layer up to the critical-Ri depth (Frierson et al. 2006). The control is
    τ = 2 h, RH_SBM = 0.7, with "shallower" shallow convection.
  - §4b, Table 4: RH_SBM 0.5–0.8 changes evaporation only modestly ("only 20 %" at the
    equator from 0.8 to 0.5). The Hadley maximum stays within 8 %. The ITCZ humidity sits
    slightly above RH_SBM (eq. 21, q = q_ref + τ P).
  - §2a–b: deep convection needs P_T > 0 and P_q > 0.
  - Nothing in the paper concerns mid-latitudes or tuning against observed humidity. Its 0.7
    was chosen for a cloud-free aquaplanet.
- **SPEEDY v41 description** (Molteni & Kucharski, ICTP, appendix A–B; PDF fetched and read).
  - Convection is triggered only where conditional instability exists *and* the humidity in
    the boundary layer and the level above exceeds RH_cnv q_sat (eq. 8; RH_cnv 0.9 in the
    boundary layer, 0.7 in the intermediate layers).
  - The closure relaxes the boundary-layer humidity toward that threshold, not toward 0.7
    (eqs. 12–13, τ_cnv 6 h).
  - A separate shallow-convection and diffusion step moves moisture between the two lowest
    layers (§B.8).
- **Isca** (Vallis et al. 2018, GMD 11, 843; read). It names the Betts–Miller scheme of
  Frierson 2007 and a "simple Monin–Obukhov boundary layer, as in … Frierson et al. (2006)",
  with no RH_SBM value or Earth evaluation of humidity. It gives nothing to compare.
- **Not read:** Janjić (1994), the BMJ treatment of the subcloud layer. The AMS site refused the
  automated fetch (HTTP 403).

Candidate fixes:

| # | Fix | Source read | Evidence here | Cost in reruns of accepted milestones |
|---|---|---|---|---|
| 1 | Raise `rh_ref` (logged knob "P5"; 0.8 is inside the paper's 0.5–0.8) | Frierson 2007 §4b, Table 4 | b: the only lever that moves RH and mid-latitude cloud. 32–57°N −0.091. Fails criterion 2 at T21 (needs cloud knobs re-set afterwards); Hadley S −33 % (paper: ±8 % on an aquaplanet); P corr −0.035 | Default change in the SBM: P1 criterion 5 (APE, T42 1 + 3 years); P2 criteria 2, 3, 5, 6 (T21 land world 3 years, T42 test world slab to equilibrium, bistability pair); P3 criterion 4 and the T21 Earth record; regress references for the moist cases (testworld, gray aquaplanet); then the P4 gate (T42 1 + 5, slab pair ≥ 30 years) |
| 2 | SPEEDY-type boundary-layer trigger and closure: deep convection only where boundary-layer RH exceeds a threshold, relaxing the boundary layer toward it | SPEEDY v41 description, eqs. 8, 12–13 | Not tested. Expected to cut the 53–61 % winter frequency and lift the boundary layer toward 0.9; would change the tropics too | New scheme variant: P1 criterion 1 SCM conservation tests and criteria 2–3, then everything in row 1 |
| 3 | `deep_requires_pt = true` (the paper's criterion; existing switch) | Frierson 2007 §2a–b | e: small (+0.010 / +0.024); criteria 1–2 met; RH unchanged. Not enough alone; could go with 1 | As row 1 (default change) |
| 4 | Shallower boundary layer (`ri_crit` 0.25) | Frierson et al. 2006 (logged range) | d: wrong direction for cloud and precipitation pattern | Not proposed |
| — | `keep_lowest` | none (diagnostic) | c: no effect | Not proposed; switch kept for diagnosis |

Checks:
- `scripts/check` passes: fmt, clippy, 130 tests plus 7 ignored.
- `scripts/regress --against 71aa0a8 --jobs 2`: **PASS**, all six cases
  byte-identical at 1 and 8 threads (`out/p4/q13bl/regress_against_71aa0a8.log`).
- 10-day Earth AMIP byte check with the cloud scheme (r3), old binary (`toyclim_q13fd`)
  against new, without the probes: **400 of 400 `.npy` files identical**
  (`out/p4/q13bl/bytecheck/`).
- Within the 16-thread limit: 3 lanes × 5 threads, then 2 lanes × 8 threads.

**Stopped here as instructed.** No fix implemented and no default changed. Options are under
question 13.

## 2026-11-26 (evening) — Question 13, owner decision after the humidity diagnosis: trigger T and screen s1–s6; **no variant meets the bar, stopped at step 4** (branch `p4-clouds`)

The owner decision (follows 819e9de) is recorded under question 13 and in the history file.
- The defect is deep convection that fires too often in the mid-latitude winter.
- The choice between "what convection relaxes to" (1) and "when it fires" (2) is made by a
  T21 screen against a fixed bar.

No default was changed.

### Step 1 — trigger T (SPEEDY) and the Betts–Miller sources

**Rule.** Source: SPEEDY v41 description, Molteni & Kucharski, ICTP, appendix B.2; PDF read.
- Deep convection is allowed only where eq. 8 holds:
  Q_N > RH_cnv·Q_sat,N and Q_{N−1} > RH_cnv·Q_sat,N−1, with RH_cnv = 0.9 (its parameter table).
- SPEEDY's layer N spans σ 0.9–1 and layer N−1 spans σ 0.77–0.9 (half levels
  0, 0.05, 0.14, 0.26, 0.42, 0.60, 0.77, 0.9, 1).

**Mapping onto our L26.**
- N: the full levels with σ ≥ 0.9 (0.9295, 0.9705, 0.9925).
- N−1: the full levels with 0.77 ≤ σ < 0.9 (0.7873, 0.8669).
- The humidity of each layer is its layer mean, Σ q Δp / Σ q_sat Δp.
- Not taken from SPEEDY: the alternative path (eq. 7) and its mass-flux closure (eqs. 12–13).
  Where the trigger allows convection, our SBM relaxation is unchanged.

**Implementation.**
- Key `[convection] bl_trigger_rh`: 0 = off (the default), must be < 1. Knob range 0.80–0.90
  when on, from SPEEDY and ICOADS; registered in `docs/tuning.md`.
- An untriggered column is not changed and is counted in `conv_skipped`.
- SCM test `bl_trigger_gates_deep_convection_and_conserves`:
  - below the threshold (0.85 in either layer), the column is unchanged and counted;
  - at 0.95 in both layers, the result is identical to the trigger off, with water and enthalpy
    conserved.
- Nothing was copied from SPEEDY source code.

**Betts–Miller shallow vs deep.** I read the primary sources from scans:
- Betts (1986), QJRMS 112, 677–691 (Part I);
- Betts & Miller (1986), QJRMS 112, 693–709 (Part II).

What they say:
- **Part I §3b:** "Cloud top height initially distinguishes shallow from deep convection
  (currently level 11 in the global model; about 780 mb)".
- **Shallow convection (eq. 13a)** has zero integrated heating and moistening from p_B to p_T,
  so it gives no precipitation. Its reference is a mixing line with β = 1 (§2, eqs. 1–2).
- **Deep convection (eq. 13b)** conserves enthalpy.
  - p_B is σ 0.98 in the tests. In later global tests, "the deep convective adjustment is made
    down to within a few millibars of the surface (one level above the model surface)".
  - The deep adjustment is suppressed if the precipitation is negative.
- **Reference profiles (eqs. 15–18):** α = 1.5, and subsaturations P_B / P_M / P_T.
- **Part II, Table 1:** τ 2 h, α 1.5, P_B −25, P_M −50, P_T −38 mb.

Our mid-latitude winter tops (650–680 hPa in s1) would still count as deep under the 780 mb
rule. Not built, as instructed.

**Not read:**
- Janjić (1994): the AMS site refuses the automated fetch (HTTP 403).
- Isca (Vallis et al. 2018) gives no SBM parameters.

### Step 2 — Hadley reference (NCEP/NCAR v, 1991–2020 long-term monthly means)

Method (`tools/hadley_ref.py`):
- ψ is integrated from 10 hPa downward and evaluated at 700 hPa, the level of the model's
  σ 0.7 metric.
- Each cell is found by sign: the northern cell is the largest positive ψ at −20° to 35°, the
  southern cell the largest negative ψ at −35° to 20°.

Results (10¹⁰ kg s⁻¹):

| Season | Northern cell | Southern cell |
|---|---|---|
| Annual | 6.24 at 17.5° | 8.16 at −15° |
| DJF | 15.63 | 4.30 |
| JJA | 2.30 | 16.42 |

The per-hemisphere maximum |ψ| gives the same annual values. Output:
`out/p4/q13scr/hadley_ncep.json`.

**Change to how the bar is measured.** The bar now uses the model's cells by sign, the same
definition as the reference.
- The per-hemisphere maximum |ψ| picks up the southern cell where it reaches north of the
  equator. For s3 it gives a "northern cell" of 8.61, but the positive cell is 3.98.
- Both values are reported in the table below.
- No outcome changes: every run fails the correlation condition under either definition.

### Step 3 — screen (T21 Earth AMIP 1 + 1 years, year 2; cloud knobs at r3)

Setup:
- Cases `cases/bench/p4_q13scr/`, runs `out/p4/q13scr/`, analysis
  `tools/bl_humidity.py --screen` (`screen.txt`, `screen.json`).
- Lanes: 4 × 4 threads, then 2 × 8 threads.
- All six runs completed both years with rc 0. Maximum Courant 2.4 (s1) to 3.8 (s4).

The runs:
- s1: reference.
- s2: trigger 0.90.
- s3: trigger 0.80.
- s4: `rh_ref` 0.8 with `deep_requires_pt`.
- s5: trigger 0.90 with `deep_requires_pt`. s2 was the better of s2 and s3: higher northern RH,
  correlation 0.664 against 0.644, and Hadley closer.
- s6, the free run: trigger 0.90 with `ri_crit` 0.25 (logged range 0.25–1). Reason, stated
  before running: earlier run d (`ri_crit` 0.25) was the only change that met the northern RH
  bar (0.758). s6 tests whether the boundary-layer side combined with the trigger passes.

| run | RH 40–60°N DJF | RH 40–60°S JJA | P global | P corr | Hadley N / S cell | max \|ψ\| NH / SH | conv. active 40–60°N DJF / S JJA / 10°S–10°N | conv. share same | E 40–60°N DJF / S JJA | albedo | OLR | net TOA | SW / LW CRE | trop. net CRE | low 32–57°N / 45–60°S | clt − CERES 32–57°N / 45–60°S |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| obs | 0.802 | 0.819 | 2.69 | — | 6.24 / 8.16 | — | — | 0.29 / — / 0.99 (NCEP) | 2.73 / 1.85 | | | | | −14.6 | 0.344 / 0.599 | 0 / 0 |
| s1 | 0.700 | 0.673 | 2.92 | 0.704 | 5.47 / 10.88 | 5.47 / 10.88 | 0.61 / 0.53 / 0.33 | 0.68 / 0.66 / 0.82 | 3.64 / 2.99 | 0.303 | 233.8 | +3.24 | −53.1 / 27.2 | −39.8 | 0.255 / 0.407 | −0.159 / −0.271 |
| s2 | 0.709 | 0.662 | 2.85 | 0.664 | 4.35 / 7.34 | 6.38 / 7.34 | 0.01 / 0.00 / 0.01 | 0.06 / 0.03 / 0.12 | 3.14 / 2.84 | 0.336 | 229.4 | −3.45 | −64.7 / 30.7 | −51.5 | 0.405 / 0.600 | −0.050 / −0.146 |
| s3 | 0.683 | 0.671 | 2.74 | 0.644 | 3.98 / 11.27 | 8.61 / 11.27 | 0.04 / 0.02 / 0.03 | 0.14 / 0.13 / 0.54 | 3.31 / 2.67 | 0.327 | 228.6 | +0.38 | −61.5 / 31.3 | −45.1 | 0.366 / 0.559 | −0.080 / −0.174 |
| s4 | 0.725 | 0.724 | 2.84 | 0.630 | 4.90 / 8.74 | 7.04 / 8.74 | 0.36 / 0.31 / 0.13 | 0.37 / 0.33 / 0.47 | 3.24 / 2.48 | 0.327 | 231.9 | −2.96 | −61.3 / 29.1 | −48.7 | 0.350 / 0.509 | −0.098 / −0.214 |
| s5 | 0.686 | 0.659 | 2.82 | 0.622 | 5.08 / 9.03 | 7.72 / 9.03 | 0.01 / 0.00 / 0.01 | 0.04 / 0.03 / 0.12 | 3.52 / 2.90 | 0.338 | 229.0 | −3.79 | −65.2 / 31.1 | −51.8 | 0.397 / 0.584 | −0.072 / −0.155 |
| s6 | **0.782** | **0.797** | 2.51 | 0.627 | 4.58 / 10.58 | 7.21 / 10.58 | 0.04 / 0.03 / 0.01 | 0.21 / 0.27 / 0.25 | 3.27 / 2.08 | 0.317 | 230.4 | +2.15 | −57.6 / 29.1 | −44.8 | 0.313 / 0.448 | −0.124 / −0.249 |

Criteria 1–2 (reported, not required):

| Run | Criterion 1 | Criterion 2 |
|---|---|---|
| s1 | met | met |
| s2 | albedo 0.336 | SW CRE −64.7 |
| s3 | met | SW CRE −61.5 |
| s4 | met | SW CRE −61.3 |
| s5 | albedo 0.338 | SW CRE −65.2 |
| s6 | met | met |

The full tables are in `out/p4/q13scr/screen.txt`: RH, E, P and convective share for every
band and season, the seasonal Hadley cells, and the moisture budget of σ ≥ 0.87.

Seasonal cells by sign (10¹⁰ kg s⁻¹, northern / southern):

| Run | DJF | JJA |
|---|---|---|
| NCEP | 15.63 / 4.30 | 2.30 / 16.42 |
| s1 | 14.37 / 4.04 | 4.44 / 20.72 |
| s2–s6 | 13.3–15.5 / −0.02–3.19 | — / 26.2–30.0 |

Every variant strengthens the JJA southern cell by 27–45 % over s1.

Figure `docs/figures/P4/q13_screen_bands.png` has three panels: near-surface ocean RH,
evaporation and precipitation, each by band against the observations.
- **RH panel.** s6 lies on ICOADS within 0.01 in the tropics and northern subtropics, and is
  about 0.02 low in the winter 40–60° bands.
  - s2, s3 and s5 are 0.02–0.04 above s1 in the tropics, but at or below s1 in the winter
    40–60° bands.
  - s4 lies between them.
- **Evaporation panel.** The runs are close to each other; s6 is lowest everywhere, with
  40–60°S winter at 2.08 against NCEP 1.85.
- **Precipitation panel.** s2–s5 put 4.05–4.19 mm/day into 10–30°N (s6 3.65, s1 3.38) against GPCP 2.51
  (JJA 5.2–6.6 against 3.44).
  - All variants take precipitation out of 10°S–10°N in JJA (3.4–4.0 against GPCP 4.3; s1 4.7).
  - This northward shift of the summer rain is where the pattern correlation is lost.

### What the screen shows

- **How often convection fires is not what dries the winter boundary layer.**
  - The trigger at 0.90 cuts mid-latitude winter convection from 53–61 % of samples to
    0.4–1.3 %.
  - Winter RH stays at 0.709 / 0.662 (s2) and 0.686 / 0.659 (s5), against 0.700 / 0.673 in s1.
  - In σ ≥ 0.87, convection changes from a source (+0.34 / +0.92 mm/day) to a small sink, but
    the boundary-layer mixing still exports 2.7–3.1 mm/day.
- **SPEEDY's 0.9 does not carry over to our levels.**
  - Our tropical boundary layer has a layer RH of about 0.76, so the trigger also switches
    off tropical convection: convective share 0.12 against NCEP 0.99, active fraction 0.006.
  - At 0.80 (s3) the tropical share is still only 0.54.
  - Most precipitation becomes large-scale, and the low cloud rises over the GOCCP values
    (s2: 32–57°N 0.405 against 0.344).
- **`rh_ref` 0.8 (s4)** gives the largest RH gain among the convection-only variants:
  +0.025 / +0.051 over s1.
- **What moves the winter RH is the depth of the boundary layer.**
  - s6, with the shallower boundary layer, meets the RH condition (0.782 / 0.797, both
    within 0.05).
  - It also meets the P condition (2.51), criteria 1–2 and RH within 0.01 of ICOADS in the
    tropics.
  - Its precipitation correlation is 0.627 against the required ≥ 0.694.
  - Its northern cell falls to 4.58: farther from 6.24 than s1's 5.47, and −16 % of s1.
- **Every variant loses 0.04–0.08 of precipitation correlation** against s1 (0.704).

### Step 4 — the bar (applied unchanged)

| run | RH | P | corr ≥ 0.694 | Hadley | stable | all |
|---|---|---|---|---|---|---|
| s1 | no | yes | yes | yes | yes | **no** |
| s2 | no | yes | no | no | yes | **no** |
| s3 | no | yes | no | no | yes | **no** |
| s4 | no | yes | no | no | yes | **no** |
| s5 | no | yes | no | yes | yes | **no** |
| s6 | yes | yes | no | no | yes | **no** |

Notes on the bar:
- "Stable" means both years completed with no failure. The analysis is of year 2; year 1 was
  not compared separately.
- **No variant meets the bar. Stopped at step 4 as instructed.** Step 5 was not started.
- No default was changed; the trigger stays off.

### Checks

- `scripts/check` passes: fmt, clippy, 141 tests plus 7 ignored.
- `scripts/regress --against 819e9de --jobs 2`: **PASS**, all six cases byte-identical at 1 and 8 threads
  (`out/p4/q13scr/regress_against_819e9de.log`).
- Threads: at most 16 throughout.

Options for the owner are under question 13.

## 2026-11-26 (late evening) — Question 13, owner decision after the convection screen: step 1 analysis and resolution / mixing-depth runs v1–v5; **stopped and reported as instructed** (branch `p4-clouds`)

The owner decision (follows 8b0c923) is recorded under question 13 and in the history file.
- The trigger stays off, `rh_ref` stays 0.7 and `deep_requires_pt` stays off.
- Step 1 uses existing output only. Step 2 is at most 5 diagnostic runs. Then stop.

No physics code changed and no default changed. New: `tools/q13_step1.py` (the step 1 analyses
and per-run profiles). `tools/cloud_split.py` now draws each run's own level set in its profile
figure.

### Step 1a — the precipitation shift (s1–s6, d; T21 year 2)

Figures:
- `docs/figures/P4/q13_step1_precip_minus_s1.png`: JJA and DJF, each run minus s1.
- `docs/figures/P4/q13_step1_precip_minus_gpcp.png`: each run minus GPCP v2.3.
- `docs/figures/P4/q13_res_precip_minus_s1.png`: the same for d, s6 and v1–v5.

What the maps show:
- In JJA every variant adds more than 4 mm/day over the north-west Pacific (100–180°E,
  10–30°N). It dries a strip at 0–10°N in the Indian Ocean and West Pacific: the summer
  monsoon trough moves north.
- s1 already overestimates the same region against GPCP. The variants make it worse.
- DJF changes are patchy (South Pacific convergence zone), with no common sign.

10–30°N in JJA (band means, mm/day; GPCP P 3.50, land 3.89, ocean 3.33):

| run | P | P land | P ocean | P conv | P ls | E | P − E |
|---|---|---|---|---|---|---|---|
| s1 | 4.50 | 5.01 | 4.28 | 3.80 | 0.70 | 3.85 | 0.65 |
| s2 | 5.70 | 4.56 | 6.20 | 0.52 | 5.17 | 3.86 | 1.84 |
| s3 | 5.56 | 4.05 | 6.22 | 2.40 | 3.16 | 3.35 | 2.21 |
| s4 | 5.78 | 3.88 | 6.62 | 4.40 | 1.38 | 3.56 | 2.22 |
| s5 | 5.37 | 5.68 | 5.24 | 0.66 | 4.71 | 3.50 | 1.88 |
| s6 | 4.95 | 3.90 | 5.41 | 1.71 | 3.24 | 3.04 | 1.91 |
| d | 5.25 | 4.94 | 5.39 | 4.50 | 0.75 | 4.00 | 1.25 |

Change against s1, as contributions to the band mean (mm/day):

| run | ΔP | land | ocean | ΔE | Δ(P − E) | West Pacific 100–160°E | South Asia 60–100°E |
|---|---|---|---|---|---|---|---|
| s2 | +1.19 | −0.14 | +1.33 | +0.01 | +1.18 | +1.76 | −0.61 |
| s3 | +1.05 | −0.29 | +1.35 | −0.50 | +1.56 | +1.02 | +0.26 |
| s4 | +1.28 | −0.35 | +1.63 | −0.29 | +1.57 | +1.36 | −0.40 |
| s5 | +0.87 | +0.20 | +0.67 | −0.35 | +1.22 | +0.81 | +0.26 |
| s6 | +0.45 | −0.34 | +0.79 | −0.81 | +1.26 | +0.69 | +0.05 |
| d | +0.75 | −0.02 | +0.77 | +0.16 | +0.60 | +0.76 | +0.13 |
| v1 | +0.60 | −0.08 | +0.68 | +0.27 | +0.34 | +0.64 | −0.04 |
| v2 | +0.32 | −0.07 | +0.39 | +0.06 | +0.26 | +0.59 | −0.27 |
| v3 | +1.20 | +0.25 | +0.95 | +0.55 | +0.65 | +1.05 | +0.21 |
| v4 | −0.41 | −0.02 | −0.39 | +0.26 | −0.67 | −0.33 | −0.37 |
| v5 | +0.07 | +0.05 | +0.02 | +0.32 | −0.25 | +0.29 | −0.40 |

What feeds the extra rain:
- It falls over the **ocean**; over land the change is near zero or negative.
- It is fed by **moisture convergence** (Δ(P − E) ≈ ΔP), not by local evaporation: ΔE is
  between −0.81 and +0.55.
- It comes as large-scale rain where convection is suppressed (s2, s3, s5, s6) and as
  convective rain otherwise (s4, d, v1, v3).
- It falls in the **same region in every variant** except the L32 runs: the West Pacific
  sector carries most of it.
- Pattern correlation of the JJA change maps over 0–40°N:
  - among s2–s6: 0.58–0.82;
  - d against s2–s6: 0.21–0.46;
  - d with v1 and v3 (the same `ri_crit` lever): 0.78.
  - v4 against the others: −0.05 to 0.33, and 0.47 with v5.

**Where the annual correlation is lost.** For s2 and s5 the seasonal DJF and JJA correlations
against GPCP are not lower than s1:
- DJF: 0.617 (s1) against 0.636 / 0.616;
- JJA: 0.579 (s1) against 0.580 / 0.591.

The loss sits:
- in the annual map at 0–20°N (s1 0.56; variants 0.43–0.51);
- in SON (s1 0.653; variants 0.443–0.572).

**Year-to-year spread.** The only multi-year AMIP run with per-year output is r3 T42 1 + 5
(`out/p4/lts/gate_r3/earth_amip_t42`). Its `precip` field is 10-day instantaneous snapshots,
36–37 per year.
- **Raw single-year correlations** (years 2–6): 0.588, 0.632, 0.608, 0.574, 0.600; std 0.022,
  range 0.057. The 5-year climatology sampled every 6 h gives 0.722.
- **Sampling noise dominates.** Odd against even halves of a year differ by 0.017 on average.
  The deseasonalised within-year variance makes up 92 % of the cell-wise year-to-year variance.
- **Corrected estimate.** Removing that noise gives, for a 6-hourly-sampled single year,
  0.681–0.708: std 0.011, range 0.027.
- **What this means for the screen.** Losses of 0.04–0.08 after one year are 4–7 times that
  std, so they are not interannual noise.
- **Caveats:** this is T42, not T21; the cell-wise shrink is a simple correction; and there
  is only one run, so no ensemble.

### Step 1b — humidity profiles against NCEP/NCAR reanalysis 1

Data: NCEP/NCAR R1 RH and q on pressure levels, long-term monthly means 1991–2020, fetched
2026-11-26 (`reference/raw/ncep_{rhum,shum,air}_p/`).
- The model is sampled from its 5-day snapshots in year 2, over ocean cells.
- NCEP is interpolated in ln p to each level's band-mean pressure.
- Caveat: at 40–60°S in JJA the surface pressure is about 990 hPa, so NCEP's 1000 hPa value
  there is below ground (extrapolated).

Full tables: `out/p4/q13res/profiles.txt`. RH, NCEP / s1 / d / s6:

| level (hPa) | 40–60°N DJF | 40–60°S JJA | trades 10–25°N | trades 10–25°S |
|---|---|---|---|---|
| ~593 | 0.57 / 0.71 / 0.68 / 0.70 | 0.57 / 0.73 / 0.73 / 0.75 | 0.36 / 0.32 / 0.31 / 0.41 | 0.30 / 0.22 / 0.21 / 0.28 |
| ~690 | 0.58 / 0.73 / 0.68 / 0.74 | 0.62 / 0.76 / 0.72 / 0.78 | 0.40 / 0.36 / 0.32 / 0.43 | 0.35 / 0.26 / 0.23 / 0.30 |
| ~780 | 0.63 / 0.77 / 0.71 / 0.82 | 0.66 / 0.80 / 0.73 / 0.82 | 0.52 / 0.57 / 0.45 / 0.56 | 0.49 / 0.45 / 0.38 / 0.44 |
| ~860 | 0.69 / 0.79 / 0.79 / 0.88 | 0.72 / 0.80 / 0.80 / 0.89 | 0.62 / 0.78 / 0.73 / 0.80 | 0.62 / 0.75 / 0.70 / 0.76 |
| ~920 | 0.82 / 0.74 / 0.79 / 0.85 | 0.87 / 0.73 / 0.82 / 0.89 | 0.74 / 0.79 / 0.83 / 0.89 | 0.78 / 0.82 / 0.85 / 0.89 |
| ~960 | 0.84 / 0.69 / 0.75 / 0.79 | 0.87 / 0.68 / 0.77 / 0.82 | 0.77 / 0.72 / 0.78 / 0.82 | 0.80 / 0.75 / 0.80 / 0.84 |
| ~985 | 0.84 / 0.70 / 0.76 / 0.79 | 0.87 / 0.68 / 0.76 / 0.80 | 0.79 / 0.70 / 0.75 / 0.79 | 0.81 / 0.73 / 0.78 / 0.81 |

Specific humidity (g/kg), NCEP / s1 at about 857 hPa: 2.59 / 2.85 (40–60°N DJF), 8.87 / 10.52
(trades N). At about 981 hPa: 4.36 / 4.11 and 15.51 / 14.77.

**Where the model leaves the observations:**
- In the **mid-latitude winter**, at and below 925 hPa: s1 is 0.09–0.20 too dry from 920 hPa
  to the surface. From about 870 hPa up it is too moist: +0.10 at 860, +0.14 at 780,
  +0.14–0.15 at 600–690.
  - The model RH peaks at about 860 hPa; NCEP's peaks at 960–1000 hPa.
- In the **trades** the model RH peaks at 860–930 hPa: 0.78–0.82 against NCEP 0.62–0.74 at
  those levels. There is no inversion-capped moist layer, and the surface is 0.06–0.09 too dry.
- **The shallower mixing** (d, s6) moistens the levels at and below 920 hPa by 0.06–0.17 but
  leaves 860 hPa as moist or moister. It moves moisture down; it does not reshape the profile.

### Step 1c — boundary-layer depth

The model's depth is the K-profile depth: the critical bulk Richardson number Ri_c from the
lowest level.

Means and spread over the ocean (m):
- **s1** (Ri_c 1), from the monthly-mean cells; p10 / p50 / p90 in brackets:
  - 40–60°N DJF 1857 (1139 / 1867 / 2499);
  - 40–60°S JJA 2081 (1288 / 1975 / 2897);
  - trades 1159–1279; tropics 1193.
- **d** (Ri_c 0.25): 1142, 994, 959–1000 and 934 in the same regions.
- **Instantaneous distributions** from the `pblh` snapshots of v1–v5 (`out/p4/q13res/pblh.txt`),
  40–60°N DJF:
  - v2 (L30, Ri_c 1): p10 / p50 / p90 872 / 1805 / 3131;
  - v3 (L30, 0.25): 379 / 1018 / 1549.

Published climatology read:
- **Seidel et al. (2012)**, JGR 117, D17106 (read from the NOAA GML copy). It uses the bulk Ri
  with Ri_c = 0.25 after Vogelezang & Holtslag (1996), and compares 0.30 (14 % higher in their
  example).
  - It covers the continental US and Europe from radiosondes: daytime < 1 km, night < 0.5 km.
    ERA-Interim and the CAM5 and AM3 models are deeper than the radiosondes "due to
    difficulty simulating stable conditions".
  - Over ocean they give only "ERA-Interim and the climate models simulate z(Ri0.25)
    generally < 1.5 km (Figure S2)". The supplement was not read.
- **ROM SAF VS21 report** (Xie 2014; read). It gives an ERA-Interim seasonal RH-gradient
  climatology for 2008, as maps only: about 1 km near the eastern-ocean coasts and about 2 km
  in the trade-wind regions. It cites von Engeln & Teixeira (2013) as consistent.
  - That is a different definition (the inversion), so no band numbers are taken from it.
- **Not read:** von Engeln & Teixeira (2013), J. Climate (AMS, not fetched).
- **No readable published climatology gives ocean band means with a Ri definition.**
  - The only usable statement is Seidel's "< 1.5 km" over ocean, near the US and Europe.
  - s1's winter means (1.9–2.1 km) are above it; d's (1.0–1.1 km) are below it.
  - Ri_c 1 (ours) against 0.25 (theirs) accounts for much of the difference.

### Step 1d — `ri_crit`

- **Default:** 1, after Frierson, Held & Zurita-Gotor (2006), JAS 63, 2548, Table 1: "Ri_c,
  Critical Richardson number for stable mixing cutoff, 1". Their §2d sets the depth where the
  bulk Ri of their eq. 16 exceeds Ri_c (GFDL copy re-read).
  - In our scheme the same constant sets the depth and the stability function of K
    (physics/boundary_layer.rs).
- **Published values (sources read):**
  - Holtslag & Boville (1993), J. Climate 6, 1825, §3b, CCM2 non-local scheme: "The value of
    the critical bulk Richardson number … depends generally on the vertical resolution of the
    model. For our (still rather crude) vertical model resolution, we use Ri_cr = 0.5".
  - Seidel et al. (2012): 0.25 (Vogelezang & Holtslag 1996), with 0.30 compared.
  - Frierson et al. (2006): 1.
- **Not read:**
  - Troen & Mahrt (1986): the University at Albany copy returned 404; Springer was not tried.
  - Vogelezang & Holtslag (1996): Springer; cited here only through Seidel.
- **The intermediate published value** between 1 and 0.25 is 0.5 (Holtslag & Boville), used
  for v1.

### Step 2 — runs (T21 Earth AMIP 1 + 1 years, year 2; cloud knobs r3, convection defaults)

Cases `cases/bench/p4_q13res/`, runs `out/p4/q13res/`. All runs used dt 20 min; all completed
with rc 0.

The runs:
- **v1:** `ri_crit` 0.5.
- **v2:** L30. The four L26 layers below σ 0.831 are split in two at their midpoints:
  - half levels 0.8310, 0.8672, 0.9033, 0.9296, 0.9560, 0.9706, 0.9851, 0.9926, 1;
  - the lowest full level is σ 0.9963, about 30 m;
  - the levels above are unchanged.
  - No code change was needed: `[model] sigma_half` was already configurable.
- **v3:** L30 with `ri_crit` 0.25.
- **v4 (free).** Reason, stated before running: step 1b shows the model leaving NCEP from about
  925 hPa up, with the moist excess at 780–870 hPa, and the low cloud sits at 780–860 hPa
  (cloud by level, below). v2 leaves the 744–831 hPa layer (about 90 hPa) unrefined, and the
  trade inversion would sit there.
  - So v4 is L32: the six layers below σ 0.649 split in two.
- **v5 (free):** L32 with `ri_crit` 0.25. It pairs v4 as v3 pairs v2, so resolution and mixing
  depth are separated in the cloud layer too.

**Cost (v2–v5).** 30-day runs alone on 8 threads: L26 36.6 s, L30 40.9 s (+12 %), L32 43.0 s
(+17 %).
- 2-year wall times depend on the load and are not comparable with s1:
  - v2 1950 s and v3 2461 s (3 lanes × 4 threads);
  - v4 1671 s and v5 1678 s (2 × 8);
  - s1 2656 s (4 × 4).
- Maximum Courant: v2 3.6, v3 3.6, v4 3.5, v5 4.1 (s1 2.4, s4 3.8). It is not a stability
  limit here; all ran to day 730.5.

| run | RH 40–60°N DJF | RH 40–60°S JJA | P global | P corr | Hadley N / S cell | albedo | OLR | net TOA | SW / LW CRE | cover | trop. net CRE | low 32–57°N / 45–60°S (GOCCP 0.344 / 0.599) | clt − CERES 32–57°N / 45–60°S |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| obs | 0.802 | 0.819 | 2.69 | — | 6.24 / 8.16 | | | | | | −14.6 | 0.344 / 0.599 | 0 / 0 |
| s1 (L26, Ri_c 1) | 0.700 | 0.673 | 2.92 | 0.704 | 5.47 / 10.88 | 0.303 | 233.8 | +3.24 | −53.1 / 27.2 | 0.550 | −39.8 | 0.255 / 0.407 | −0.159 / −0.271 |
| v1 (L26, 0.5) | 0.734 | 0.737 | 2.85 | 0.679 | 4.41 / 9.25 | 0.301 | 234.9 | +3.02 | −52.2 / 27.0 | 0.542 | −38.2 | 0.236 / 0.400 | −0.171 / −0.274 |
| d (L26, 0.25) | 0.758 | 0.761 | 2.76 | 0.623 | 5.28 / 12.07 | 0.292 | 235.8 | +5.18 | −49.1 / 26.1 | 0.528 | −36.8 | 0.225 / 0.359 | −0.191 / −0.306 |
| v2 (L30, 1) | 0.694 | 0.663 | 3.12 | 0.726 | 4.95 / 10.81 | 0.306 | 232.9 | +3.18 | −54.1 / 28.3 | 0.571 | −40.6 | 0.268 / 0.430 | −0.140 / −0.250 |
| v3 (L30, 0.25) | 0.762 | 0.748 | 2.92 | 0.681 | 4.45 / 10.30 | 0.293 | 234.4 | +6.13 | −49.5 / 26.8 | 0.545 | −36.5 | 0.236 / 0.367 | −0.163 / −0.296 |
| v4 (L32, 1) | 0.692 | 0.667 | 3.14 | 0.722 | 5.66 / 11.00 | 0.280 | 233.9 | **+11.01** | −45.5 / 27.6 | 0.543 | **−26.8** | 0.244 / 0.418 | −0.154 / −0.261 |
| v5 (L32, 0.25) | 0.764 | 0.749 | 2.96 | 0.665 | 5.44 / 10.11 | 0.270 | 236.0 | **+12.33** | −41.8 / 25.2 | 0.513 | **−26.0** | 0.212 / 0.335 | −0.189 / −0.316 |
| s6 (L26, 0.25 + trigger) | 0.782 | 0.797 | 2.51 | 0.627 | 4.58 / 10.58 | 0.317 | 230.4 | +2.15 | −57.6 / 29.1 | 0.579 | −44.8 | 0.313 / 0.448 | −0.124 / −0.249 |

The screen bar is applied for information only; this step has no bar.
- Every run fails RH except s6.
- v2 and v4 fail P (3.12 / 3.14), and pass the correlation and Hadley conditions.
- v1, v3, v5 and d fail the correlation condition.

Other measures:
- **Convection frequency** (40–60°N DJF / S JJA / tropics): v1 0.57 / 0.42 / 0.27; v2 0.52 /
  0.46 / 0.37; v3 0.61 / 0.51 / 0.33; v4 0.56 / 0.45 / 0.37; v5 0.58 / 0.49 / 0.32.
- **Convective share** (same order): 0.56–0.70 / 0.54–0.68 / 0.68–0.79.
- **Criteria 1–2** (reported, not required):
  - all runs meet criterion 2;
  - v4 and v5 fail criterion 1 on net TOA (+11.0 / +12.3; limit ±8);
  - the other runs meet it.

**Low cover by band, annual, raw against GOCCP** (`out/p4/q13res/cloud_split.txt`;
`docs/figures/P4/q13_res_cloud_bands.png`):

| band | GOCCP | model runs |
|---|---|---|
| 30–45°N | 0.306 | 0.15–0.24 |
| 45–60°N | 0.393 | 0.28–0.39 (s6 0.386 highest) |
| 30–45°S | 0.478 | 0.13–0.32 |
| 45–60°S | 0.599 | 0.34–0.45 |
| 60–77°S | 0.512 | 0.31–0.36 |

- Across the eight runs, every band moves by no more than 0.11, except 30–45°S, where s6 gives
  0.317 against 0.13–0.25.
- In the figure all runs sit together, well below GOCCP south of 30°S, while middle and high
  cloud lie above GOCCP poleward of 45°.

**Cloud fraction by level, 40–60°N DJF, ocean** (`out/p4/q13res/cloudlev.txt`; figure
`docs/figures/P4/q13_res_cloud_profiles.png`):

| run | ~690 hPa | ~780 hPa | ~860 hPa | ~920 hPa | ~960 hPa | ~985 hPa |
|---|---|---|---|---|---|---|
| s1 | 0.31 | 0.27 | 0.17 | 0.12 | 0.11 | 0.01 |
| d | 0.23 | 0.22 | 0.18 | 0.15 | 0.12 | 0.01 |
| v2 | 0.34 | 0.30 | 0.14–0.18 | 0.10–0.11 | 0.10 | 0.03 |
| v3 | 0.25 | 0.21 | 0.17 | 0.13–0.16 | 0.12 | 0.02 |
| v4 | 0.34–0.36 | 0.18–0.30 | 0.13–0.18 | 0.11 | 0.11 | 0.02 |
| v5 | 0.21–0.22 | 0.15–0.19 | 0.17 | 0.13–0.15 | 0.11 | 0.01 |

40–60°S JJA is alike (same file). In the figure:
- the shallower-mixing runs (d, v3, v5) take 0.05–0.15 of cloud off 690–780 hPa and add
  0.01–0.04 at 840–920 hPa;
- refinement alone (v2, v4) changes little below 850 hPa.

Profiles for v1–v5, from `out/p4/q13res/profiles.txt`:
- **Near-surface RH depends on Ri_c, not on the levels:**

  | 40–60°N DJF lowest level | v2 | s1 | v4 | v3 | d | v5 |
  |---|---|---|---|---|---|---|
  | RH | 0.694 | 0.698 | 0.695 | 0.762 | 0.759 | 0.759 |

- **The 840–880 hPa excess remains at every resolution:** at 843 hPa, v2 0.807, v3 0.772,
  v4 0.806, v5 0.756, against NCEP 0.671.
- **The trades** keep RH 0.82 at 885 hPa against NCEP 0.66–0.67 in all four refined runs, so
  L30 and L32 produce no inversion.
- **The surface stays too dry** at 0.68–0.73 against NCEP 0.80–0.81.

### What explains the precipitation shift

- The extra summer rain at 10–30°N comes from moisture convergence over the north-west
  Pacific. It shows up whenever the near-surface layer gets moister or convection is
  delayed: by `rh_ref`, the trigger or `ri_crit`.
  - It grows with the near-surface moistening: v1 +0.64, d +0.76, v3 +1.05 in that sector.
  - Evaporation hardly changes, so this is a circulation response (the monsoon trough moves
    north), not local recycling.
- **What reverses it:** refining the 650–830 hPa layers (v4: −0.33; v5 +0.29 against v3's
  +1.05). Those are the layers where the convective tops and the moist excess sit.
- **Not established:** the mechanism behind the L32 effect (heating profile, cloud-radiative
  heating, or both). This step did not diagnose it.
- **Interannual noise does not explain the shift:** the estimated single-year spread is
  std 0.011.

### Whether resolution or mixing depth controls the low cloud

**Neither does, within the tested range.**
- **Mixing depth controls the near-surface humidity:** +0.06–0.09 RH at Ri_c 0.25 in both L26
  and L30/L32.
- **But low cover falls with it** (32–57°N 0.255 → 0.225 at L26, 0.268 → 0.236 at L30,
  0.244 → 0.212 at L32): the shallower mixing removes cloud at 690–780 hPa, which is where
  most of the "low" (> 680 hPa) cover sits.
- **Vertical resolution below σ 0.83 changes low cover by +0.013**, and the near-surface RH not
  at all.
- **Resolution below σ 0.65** cuts the tropical net CRE from −39.8 to −26.8 (CERES −14.6), the
  largest tropical change seen in question 13. But net TOA goes to +11, and mid-latitude low
  cover does not rise.
- **Why the moisture does not become low cloud** (cloud_split, s1 / d / s6 at 40–60°N DJF):
  - The share of points above the scheme's RH_c rises at σ 0.93 from 0.14 to 0.26 and 0.43,
    and at σ 0.97 from 0.05 to 0.17 and 0.21.
  - But RH_c there is 0.936–0.949, and the Sundqvist fraction just above RH_c is small.
  - Half the winter points have q < 3 g/kg, where the corrected freeze-dry reduces cloud.
  - The stratiform cloud at σ 0.93 rises only from 0.017 to 0.054 and 0.073.
- **So the low cloud is limited by the cloud scheme's near-surface thresholds and the
  freeze-dry**, not by the depth or resolution of the moist layer. Testing that would be cloud
  work, which this step excludes.

### Candidate routes and the accepted criteria each would reopen

**Run costs** (from the logs): T42 Earth AMIP 1 + 5 took 5.9 h at 6 threads (r3 gate); P2
test world T42 slab 15 years took 8.8 h; the slab pair took 5.1 h (400 ppm, 15 years) and
6.5 h (800 ppm, 30 years). L30 / L32 add 12 / 17 %.

**A. `ri_crit` 0.5 or 0.25 as default** (a boundary-layer change):
- P1: 1 (SCM conservation), 2 (SCM stability), 3 (diurnal cycle, nocturnal boundary layer),
  5 (APE T42 1 + 3, about 4 h), 7 (viewer).
  - P1 4 (neutral drag) and 6 (HS / TJ16) are not affected.
- P2: 1 (SCM), 2 and 3 (T21 land world 3 years, about 1 h), 4 (restart), 5 (T42 test world
  slab, about 9 h), 6 (bistability pair, T21 slab).
- P3: 3 (SCM RCE) and 4 (T21 slab 10 years).
- New regress references for the moist cases.
- Cloud rebalance at T21 (≤ 6 × 0.75 h), then the P4 gate: T42 1 + 5 at 6 h, plus the slab
  pair at 5–7 h in parallel.
- About 30 h of runs; about 15 h wall in 2–3 lanes within 16 threads.
- On this step's evidence it moves RH toward ICOADS but costs 0.02–0.08 of P correlation and
  0.02–0.03 of low cover.

**B. More levels (L30 or L32 for the world cases):**
- **Every Part II criterion run at L26:**
  - P1 1–5 and 7;
  - P2 1–7;
  - P3 3–5 (1–2 are offline profile tests, rerun cheaply);
  - P4 1–5 with a cloud rebalance.
- **SPEC wording:** P1 criterion 5 and the P4 gate say "T42 L26", so the owner would have to
  amend SPEC.md.
- **Unaffected:** Part I (M4 JW L26 and M5 HS L20) keeps its own levels unless the default level
  set changes there too.
- **Machine time:** the route A list plus P1 1–2 and P2 1, all at +12–17 %. About 35 h of runs;
  about 18–20 h wall within 16 threads.
- On this step's evidence it lifts the P correlation (+0.02) and, at L32, the tropical CRE.
  It moves neither the near-surface RH nor the mid-latitude low cover, and pushes global P to
  3.12–3.14 (fails the 10 % window) and net TOA to +11.
- These machine times are estimates from the logged runs, not measured for the new
  configurations.

### Checks

- `scripts/check` passes. No Rust code changed, so the binary is the 8b0c923 build and regress
  was not rerun.
- Threads: 3 lanes × 4 threads, then 2 lanes × 8; at most 16 throughout.

**Stopped here as instructed.** No fix implemented and no default changed. Options are under
question 13.

## Open questions for the owner

Latest ruling each; history in [open-questions.md](progress/open-questions.md), closed ones in
[closed-questions.md](progress/closed-questions.md).

2. **Station time series and a procedural-terrain milestone** (scoping answers in
   `docs/design-decisions.md`; both extend the SPEC). Owner (2026-11-23): no decision now.
4. **Earth-gate thresholds of P5/P6** (2026-11-12). Owner (2026-11-23): unchanged until a P5
   scorecard exists with the final physics.
13. **P4: tropical cloud response and the Earth low-cloud excess.** Latest rulings (2026-11-23):
    low cloud by option (vi)/(A), `low_cloud = "slingo_lts"` (Slingo eqs. 5–7 gate + the Klein &
    Hartmann stability stratus without CAM's trigger), then T21 variants (a)–(d), diagnostic
    factorial cases, one T42 pass, freeze, gate, closing entry, stop. P4 closes with criterion 4
    open when criteria 1, 2, 3 and 5 pass at T42, the 400 ppm slab aquaplanet is within 283–293 K
    and the warming is measured once and recorded with its attribution; nothing is tuned to move
    the warming. Stop and report if: no T21 variant brings the tropical net CRE within 15 W m⁻² of
    CERES while keeping criteria 1–2 and cover ≥ 0.5; the T42 pass cannot meet criteria 1, 2, 3
    and 5; the 400 ppm slab aquaplanet leaves 283–293 K; `scripts/regress` regresses. Convection
    intermittency is the first work item of P5. Known limitations for the closing entry are in
    question 14 of the history file.
    **Result after step 4b (2026-11-24 evening): stopped at step 7** (entry above). r6 met
    criteria 1–2 with T21 cover 0.567 and went through steps 5–7 unchanged. T42 gate: criteria
    1, 2, 5 and cover met, band error 8.72 (frozen 11.48), tropical net CRE −36.3 (CERES −14.4);
    **the shape row fails**: northern cover rises monotonically to the pole, with no storm-track
    maximum. **The 400 ppm slab ends at 297.1 K** (ice-free; frozen knobs 288.5 K with ice to
    58°), mainly from +7.1 W m⁻² of LW CRE, largest at the poles. 2×CO₂ +3.53 K, recorded only.
    `scripts/regress` shows no regression. Both failures sit at high latitude and come with the
    extra upper cloud of `rh_crit_upper` 0.75 (not separated by a run). **Owner, choose:**
    (1) one separating T21 slab + Earth pair, r6 with `rh_crit_upper` back at 0.90
    (diagnostic), then decide; (2) authorise the completion-plan step 4 physics change (a
    published treatment of cold, dry polar air, SCM tests) and rerun steps 6–7 with r6;
    (3) close P4 with the frozen 2026-11-21 knobs (cover 0.497) as decided before;
    (4) take a round variant with less upper cloud (r1/r3, T21 cover 0.527/0.543) to T42,
    which needs the ≥ 0.56 rule relaxed.
    **Owner decision after step 7 (2026-11-24, follows 923fb1c):** the stop was correct; r6
    not adopted; the low-cloud scheme and the lower water cap stay as the basis; the T21
    cover ≥ 0.56 rule is withdrawn; the +3.53 K is recorded only; Q7 closed. Option (4) as a
    screen: A polar profile (record); B r1, r3, r5 at T42 1 + 1 years and in the 400 ppm slab
    for years 46–60. Pass = T42 criteria 1, 2, cover ≥ 0.51 and the shape row, plus a slab that
    keeps ice, 283–293 K and no melting trend. C: gate with the passer that has the smallest
    band error. D: if none passes, stop and propose with fetched sources. No new variants.
    **Result (2026-11-25, entry above): none passes, stopped at D.**
    - **T42:** r3 alone passes (cover 0.548, shape met, band error 10.19). r1 and r5 fail the
      shape row.
    - **Slab:** r1, r3 and r5 all lose their sea ice and end at 296.0, 295.6 and 297.4 K.
    - **Step A:** the r6 polar increase is 60–69 % upper-tropospheric and 14–27 % middle;
      8–22 % is above the tropopause (170–190 hPa).
    - **Cause of the slab warming:** at fixed SST the new low cloud itself adds +14.8 W m⁻² of
      global net CRE, of which SW +10.0 is in the tropics and subtropics and the poles add only
      +3.5 to +4.5. The frozen aquaplanet's 288.5 K rested on a tropical net CRE of −64.
    **Owner, choose:**
    (1) EIS in place of LTS in the stability law (Wood & Bretherton 2006, fetched;
    CF = 0.14 + 0.060 EIS). It adds midlatitude and Southern Ocean low cloud below σ 0.8 only,
    with SCM tests and a T21 round.
    (2) A cloud-only liquid/ice ramp of −10 to −40 °C (CAM3, Collins et al. 2004 Eq. 4.132,
    fetched). It brightens cold low and middle cloud without adding cover.
    (3) Close P4 with the frozen 2026-11-21 knobs (cover 0.497) as decided before.
    (4) Rule on criterion 4's 283–293 K for this scheme: with Earth's tropical CRE moved toward
    CERES, every variant of this scheme tested so far puts the aquaplanet above 293 K.
    No published rule removing cloud above the tropopause was found (CAM3, ECHAM5).
    **Owner decision after the screen (2026-11-25, follows ad03af1):**
    - The Earth run is the anchor: the frozen aquaplanet's 288.5 K rested on a tropical net CRE
      of −64 against −14 on Earth. Nothing is tuned to cool the aquaplanet; criterion 4's
      283–293 K is unchanged and not marked met.
    - r3 is the candidate. The closing condition "400 ppm slab within 283–293 K" is withdrawn;
      row 4 stays open with the owner (revisit with P6). P4 closes when the T42 1 + 5 year gate
      with r3 passes criteria 1, 2, 3 and 5 and row 4 is measured once on an equilibrated pair.
    - EIS (option 1; Wood & Bretherton 2006) is a P5 candidate after the convection item
      (Southern Ocean cover 0.61 against 0.87); read the paper directly before then. Option 3
      cannot close P4.
    **Result (2026-11-25 afternoon, entry above): stopped at step 1, the T42 shape row fails.**
    - Criteria 1, 2, 5 and cover (0.545) are met.
    - The northern 40–70° maximum, 0.668, sits on the band's poleward row (68.4°), 0.005 above
      65.6°. Cover is flat at 0.65–0.67 from 65° to 80°N. The maximum is local in 2 of 5 single
      years, and the screen's pass rested on one of them.
    - Slab pair, both equilibrated and ice-free: 400 ppm **295.62 K**; 2×CO₂ **+2.96 K**; cloud
      feedback negative (SW CRE −1.11 W m⁻² K⁻¹, mostly 30–60°).
    - Regress: no regression. TRACMIP AquaControl: median 295.1 K (290.4–300.7).
    - Step 3: the cold start stays ice-covered; the APE T42 run and the sweep were not done.
      Step 4 was not done.
    **Owner, choose:**
    (1) Rule on the shape row for r3 as measured. The rule and the criterion are unchanged
    here; I made no variant.
    (2) Authorise a physics change for the northern high-latitude cloud (completion-plan step
    4, a published treatment of cold, dry polar air, with SCM tests), then rerun the gate with
    r3.
    (3) Another route of the owner's choosing.
    **Owner decision after the r3 gate (2026-11-25 evening, follows a526a57; full text in the
    history file):** the stop was correct, the shape row is not ruled met; row 4 with r3 is
    recorded (295.62 K, +2.96 K) and TRACMIP's 290.4–300.7 K goes into the 283–293 K question.
    Owner's reading: the mid-latitudes (0.47 at 40.5°N) and the Southern Ocean (0.62 against
    0.87) are too clear, not the northern cap. **Option 3, the EIS law (Wood & Bretherton
    2006), now in P4**; polar air is the fallback, not authorised. Steps: A offline check
    (cover vs observations 30–90°; EIS vs LTS law on r3 output), B `low_cloud = "slingo_eis"`
    with SCM tests, C T21 round ≤ 6 variants (`water_max`, `rh_crit_upper` only), D T42 1 + 3
    years with a stricter shape screen, E full gate and closing. Stops as listed there.
    Record for P5: tropical net CRE −41.4 (CERES −14.4); tropical liquid 4.1×, ice 0.17×
    Cloud_cci.
    **Result (2026-11-26, entry above): stopped at step A, (b) fails.**
    - Wood & Bretherton (2006) read in full; CF = 0.14 + 0.060 EIS verified (± 0.06,
      ± 0.010 K⁻¹, r = 0.92); RH0 = 0.8 is the paper's own assumption.
    - (a) passes. Against CERES, r3 is short from 30° to 77° in both hemispheres: by 0.14–0.17
      at 32–57°N and 0.24–0.30 over the Southern Ocean. The northern cap is below CERES too
      (−0.03 to −0.09 at 65–80°N). CERES peaks at 57°N (0.759) and dips at 65.6°N.
    - (b) fails. In the zonal mean, EIS raises 68–80°N by +0.013 and 45–65°N by +0.006 (law
      alone +0.053 and +0.039). This holds for the lower estimate and with EIS shifted by
      +1.2 K. Cause: Γ_m is small over cold surfaces, so the summer Arctic Ocean gets the
      largest EIS (15.6 K). The Southern Ocean law doubles (0.18 → 0.35), but its total cover
      rises only 0.02 against a deficit of 0.25.
    - Also seen: one low layer per column moves zonal cover by ≤ 0.02 anywhere, while the
      mid-latitude deficits are 0.15–0.30 of total cover.
    **Owner, choose:**
    (1) The polar-air fallback (completion-plan step 4). It would put the northern maximum
    inside the band, but (a) shows that the cap is already below CERES, so this moves away from
    the observations. A source would be fetched and read before anything is built.
    (2) Take EIS forward anyway, for the Southern Ocean and the trades/warm pool, with the shape
    row still expected to fail. This needs your waiver of the step A rule.
    (3) A diagnostic step on the mid-latitude deficit itself, from existing or 1-year T21
    output: which levels and which cloud types are missing at 40–60° against CERES, and whether
    freeze-dry (q < 3 g/kg at σ ≥ 0.75, which is common in mid-latitude winter) removes
    boundary-layer cloud there. This is a hypothesis, not tested, because r3 saved no
    humidity.
    (4) Rule on the shape row as measured, or close P4 another way.
    **Owner decision after step A (2026-11-26, follows ee43b8e; full text in the history file):**
    the stop was correct; EIS not adopted in P4 (P5 candidate, tool kept); polar air refused for
    now (the cap is below CERES). The shape row fails because the mid-latitudes are too clear, a
    deficit of the RH cloud. **Option 3, diagnosis:** step 1 observed low/middle/high split
    (CALIPSO-GOCCP or ISCCP) against r3; step 2 freeze-dry against CAM's `cloud_fraction.F90`;
    step 3 T21 toggle runs (r3; freeze-dry off; gate off; both; best with `rh_crit_surface`
    0.85; one free), diagnostics only; step 4 act only by correcting freeze-dry to its source or
    by knob values inside logged ranges; with 32–57°N cover within 0.08 of CERES at T21 and
    criteria 1–2, T42 1 + 3 screen, then gate and closing. Stops as listed there.
    **Result (2026-11-26 day, entry above): stopped at step 3, no toggle restores mid-latitude
    cover.**
    - Step 1: against CALIPSO-GOCCP (fetched), the missing cloud is low cloud. The model's raw
      low cover is below GOCCP in every band (32–57°N 0.25 against 0.34; 45–60°S 0.43 against
      0.60), while middle and high cloud poleward of 45° are at or above it.
    - Step 2: freeze-dry departed from CAM in two ways. It also reduced the stability stratus
      (CAM: RH cloud only), and its bound was σ 0.75 instead of p 750 hPa. **Corrected**, with
      SCM tests and a tuning-log entry.
    - Step 3, T21, 32–57°N cover minus CERES: a r3 −0.170, b freeze-dry off −0.166, c gate off
      −0.147, d both −0.136, e d + RH_s 0.85 −0.123 (fails criterion 1), f corrected freeze-dry +
      RH_s 0.75 (inside the limits) −0.152. Every switch adds more at 60–77°N than at 32–57°N.
    - Cause, as far as measured: the mid-latitude boundary layer is sub-saturated. In winter the
      median RH at σ 0.93–0.99 is 0.67–0.73 and only 1–11 % of points exceed RH_c. Freeze-dry
      acts mostly poleward of 68°. The gate halves the potential low cloud everywhere but costs
      16 W m⁻² of tropical net CRE.
    **Owner, choose:**
    (1) A diagnostic step on the boundary-layer humidity itself (no tuning). The model's
    lowest-level ocean RH, 0.70 ± 0.08, against an observed near-surface climatology (fetched;
    W&B quote COADS 0.80 ± 0.07), plus the evaporation and the K-profile mixing at 40–60°.
    (2) A published low-cloud scheme that forms cloud below RH_c from subgrid humidity
    variability, for the mid-latitude boundary layer (e.g. a PDF/statistical scheme; source
    fetched and read first). New scheme, needs your decision.
    (3) Restrict the subsidence gate to the tropics/subtropics or to the trade regime. This is a
    restriction by latitude or regime, which your limits exclude; c/d show it would add ≤ 0.034
    at 32–57°N.
    (4) Rule on the shape row as measured (r3, with the corrected freeze-dry recorded), or close
    P4 another way.
    **Owner decision after the diagnosis (2026-11-26, follows 71aa0a8; full text in the history
    file):** the stop was correct; the freeze-dry correction is accepted (frozen-knob gate numbers
    of 2026-11-21 predate it, marked, not rerun). The low-cloud deficit is a too-dry boundary
    layer, not the cloud scheme. Options 2 and 3 refused; option 4 not ruled (P4 does not close
    with the shape row open). Option 1 taken and widened; **plan change**: the P5 convection item
    is done inside P4, P5 starts after P4 closes (SPEC.md not edited). r3 with the corrected
    freeze-dry is the working configuration; no cloud tuning until the humidity question is
    answered. Steps: 1 observations (near-surface ocean RH, GPCP, evaporation, convective share)
    by band and season; 2 budget of σ ≥ 0.87 at 40–60° winter and the trades; 3 ≤ 5 T21 toggles
    (reference; `rh_ref` 0.8; convection off the lowest levels; weaker/shallower mixing; free);
    4 sources and candidate fixes with costs. Then stop; no fix, no default changed.
    **Result (2026-11-26 afternoon, entry above): diagnosis done, stopped as instructed.**
    - Observations (ICOADS, NCEP σ 0.995, NCEP LHF and cprat, GPCP; OAFlux not reachable):
      near-surface ocean RH is 0.05–0.15 below ICOADS in every band, worst in the winter
      mid-latitudes (0.700 / 0.673 against 0.802 / 0.819); winter evaporation +33 % (N) and
      +62 % (S) over NCEP; global P 2.92 (T21) and 3.20 (T42) against GPCP 2.69; convective
      share at 40–60° 0.57–0.68 against NCEP 0.16–0.29.
    - Budget (σ ≥ 0.87, 40–60° winter): E +3.6 / +3.0, mixing export −3.7 / −3.9, convection
      **+0.3 / +0.9 (moistens)**; deep convection active in 53–61 % of samples, tops 650–680
      hPa; boundary layer 1.9–2.1 km; parcels start at the lowest level. Trades: convection
      −0.7, dynamics −1.0.
    - Toggles (32–57°N cover − CERES; ocean lowest-level RH): a −0.159, 0.723; b `rh_ref` 0.8
      **−0.091**, 0.757, but SW CRE −60.9 (criterion 2 fails), Hadley S −33 %, P corr −0.035;
      c convection off the lowest two levels −0.156 (no effect); d `ri_crit` 0.25 −0.191, 0.781
      (moister lowest level, less cloud); e `deep_requires_pt` −0.149 (small).
    - Hypothesis: half right. The RH is held at `rh_ref`, and convection is far too frequent in
      the mid-latitude winter, but it acts through the levels above the boundary layer, which
      convection holds at `rh_ref` and the mixing drains into; not by drying the boundary layer
      directly (c). Mixing depth sets where the moisture sits, not how much there is.
    **Owner, choose** (each needs reruns of P1 criterion 5, P2 criteria 2, 3, 5, 6, P3
    criterion 4, new regress references, then the P4 gate; details in the entry):
    (1) Raise `rh_ref` (Frierson 2007 §4b tested 0.5–0.8; logged knob), alone or with (3),
    then reset the cloud knobs inside their ranges for criteria 1–2. Evidence: b.
    (2) A boundary-layer trigger and closure for deep convection as in SPEEDY (Molteni &
    Kucharski, v41 description, eqs. 8, 12–13: convect only where boundary-layer RH exceeds
    RH_cnv = 0.9, relax it toward that threshold). New scheme variant, SCM tests first;
    untested here.
    (3) `deep_requires_pt = true` (the paper's deep criterion), small on its own (e).
    (4) Another route of the owner's choosing.
    **Owner decision after the humidity diagnosis (2026-11-26, follows 819e9de; full text in the
    history file):** accepted as stated. The defect: deep convection fires too often in the
    mid-latitude winter (53–61 %, tops 650–680 hPa, 57–68 % of the precipitation), holds the air
    above σ 0.87 at `rh_ref` and rains out what the mixing brings up. Run b shows the direction,
    not the fix. Choose between what convection relaxes to (1) and when it fires (2) by a
    screen on Earth targets. Step 1: build T, a boundary-layer humidity trigger after SPEEDY
    (0.9; knob range 0.80–0.90; off by default, byte-identical, SCM tests first); report what a
    primary Betts–Miller source says on shallow vs deep by cloud depth (do not build). Step 2:
    Hadley reference from NCEP v (both cells, annual and by season). Step 3: T21 screen ≤ 6:
    s1 reference; s2 trigger 0.90; s3 trigger 0.80; s4 `rh_ref` 0.8 + `deep_requires_pt`; s5
    better trigger + `deep_requires_pt`; s6 free (reason first). Step 4 bar (unchanged, all at
    T21): winter ocean RH at 40–60° within 0.05 of ICOADS in both hemispheres; global P within
    10 % of GPCP; P pattern correlation ≥ s1 − 0.01; each Hadley cell closer to the reference
    than s1 or within 10 % of s1; stable. One passer: continue; several: highest correlation;
    none: stop. Step 5 with an adopted variant: a rerun P1 criterion 5, P2 criteria 2, 3, 5, 6,
    P3 criterion 4 (stop on failure); b cloud rebalance at T21 ≤ 6 variants in logged ranges;
    c T42 1 + 3 for the best one or two (2026-11-25 screen rule); d defaults, regress
    references, full gate and closing as of 2026-11-25. Stops: no variant meets the bar; a
    P1–P3 criterion fails; no rebalance meets 1–2, or the T42 screen or gate fails; a slab fails
    or does not equilibrate in 45 years. ≤ 16 threads.
    **Result (2026-11-26 evening): no variant meets the bar; stopped at step 4.**
    - Trigger T (SPEEDY eq. 8, mapped onto σ ≥ 0.9 and 0.77–0.9) is built, tested and off.
    - The NCEP Hadley reference is N 6.24, S 8.16 (annual cells by sign).
    - Winter RH 40–60°N DJF / 40–60°S JJA (ICOADS 0.802 / 0.819) and P correlation (≥ 0.694):
      - s1: 0.700 / 0.673, corr 0.704;
      - s2 (trigger 0.90): 0.709 / 0.662, 0.664;
      - s3 (0.80): 0.683 / 0.671, 0.644;
      - s4 (`rh_ref` 0.8 + pt): 0.725 / 0.724, 0.630;
      - s5 (0.90 + pt): 0.686 / 0.659, 0.622;
      - s6 (0.90 + `ri_crit` 0.25): 0.782 / 0.797, 0.627 (RH and P met; correlation and
        northern cell 4.58 fail).
    - How often convection fires does not set the winter RH: the trigger cuts it from 53–61 %
      to ≤ 1.3 % with no RH gain.
    - SPEEDY's 0.9 also switches off tropical convection on our levels (share 0.12 against
      NCEP 0.99).
    - Only the boundary-layer depth reaches the RH condition.
    - All variants lose 0.04–0.08 of correlation, with summer rain moved into 10–30°N.
    **Options:**
    (1) Diagnose the correlation loss common to all variants (JJA rain 5.2–6.6 against GPCP
        3.44 at 10–30°N), diagnostics only, before choosing a route.
    (2) The boundary-layer route (`ri_crit` toward 0.25, logged range), the only one meeting
        the RH condition; it would need the correlation and the northern cell recovered.
    (3) Accept the near-surface humidity deficit as a limitation and return to r3 (T42 shape
        row open).
    (4) Another route of the owner's choosing.
    Trigger T is not proposed in its SPEEDY form: thresholds below 0.80 are outside the
    logged range.
    **Owner decision after the convection screen (2026-11-26, follows 8b0c923; full text in the
    history file):**
    - The stop was correct, and so was the change to the Hadley measure; the bar stays.
    - Trigger T is not adopted and stays off. `rh_ref` stays 0.7 and `deep_requires_pt` off.
      No further convection variants for now.
    - Established: only the mixing depth moves the winter near-surface RH (d, s6), and with it
      global P comes within 10 % of GPCP.
    - Not explained: why low cloud does not follow the moisture, and why every variant moves
      summer rain into 10–30°N.
    - Vertical resolution has limited six results, so it is tested directly, as a diagnostic.
    - Step 1, existing output only (s1–s6, d, r3 T42):
      - a. P maps by season against s1 and GPCP; 10–30°N summer land/ocean,
        convective/large-scale, evaporation or convergence; the same regions in every variant?
        Year-to-year spread of the pattern correlation.
      - b. RH and q profiles against a named reanalysis (s1, d, s6; 40–60° winter, trades) at
        our levels up to 600 hPa.
      - c. Boundary-layer depth, mean and distribution, against a published climatology.
      - d. `ri_crit` default, its source and the published values (sources read).
    - Step 2, T21 Earth AMIP 1 + 1, at most 5 runs, cloud knobs r3, convection defaults:
      - v1 `ri_crit` at the intermediate published value;
      - v2 about twice the levels below σ 0.8 (skip v2 and v3 if code changes are needed);
      - v3 v2 with `ri_crit` 0.25;
      - v4 and v5 free, reason first.
      - Report: the screen table, low cover by band against GOCCP, cloud fraction by level at
        40–60° in winter, the step 1b profiles; time step and run time for v2 and v3.
    - Then stop and report: the precipitation shift, whether resolution or mixing depth
      controls low cloud, and for each route the accepted criteria to rerun (more levels: all,
      with machine time).
    - No fix, no default change, no T42, slab, cloud tuning or gate. ≤ 16 threads.
    **Result (2026-11-26 late evening): step 1 done, v1–v5 run; stopped as instructed.**
    - **Precipitation shift:** the extra 10–30°N summer rain is ocean rain over the north-west
      Pacific, fed by convergence (ΔE ≈ 0). It is the same region in every variant.
      - It grows with near-surface moistening: v1 +0.64, d +0.76, v3 +1.05 mm/day in that sector.
      - L32 (v4) reverses it.
      - Single-year noise in the P correlation is estimated at std 0.011 (r3 T42, corrected for
        sampling), so the 0.04–0.08 losses are not noise.
    - **Profiles (NCEP R1):** in the winter mid-latitudes the model is 0.09–0.20 too dry from
      920 hPa down and 0.10–0.15 too moist from 870 hPa up.
      - In the trades it shows no inversion: RH 0.78–0.82 at 860–930 hPa against 0.62–0.74.
      - A shallower mixing moistens the surface but leaves the excess.
    - **Boundary-layer depth:** s1 1.9–2.1 km in winter (Ri_c 1). The only readable climatology
      is Seidel et al. 2012: over ocean "generally < 1.5 km" at Ri 0.25.
    - **`ri_crit`:** default 1 (Frierson et al. 2006 Table 1); 0.5 (Holtslag & Boville 1993);
      0.25 (Seidel et al. 2012 after Vogelezang & Holtslag 1996).
    - **Runs**, winter RH N / S, P, P correlation, low 32–57°N, tropical net CRE:
      - v1 (0.5): 0.734 / 0.737, 2.85, 0.679, 0.236, −38.2;
      - v2 (L30): 0.694 / 0.663, 3.12, 0.726, 0.268, −40.6;
      - v3 (L30, 0.25): 0.762 / 0.748, 2.92, 0.681, 0.236, −36.5;
      - v4 (L32): 0.692 / 0.667, 3.14, 0.722, 0.244, −26.8 (net TOA +11.0);
      - v5 (L32, 0.25): 0.764 / 0.749, 2.96, 0.665, 0.212, −26.0 (net TOA +12.3).
    - **Near-surface RH is set by Ri_c, not by the levels.**
    - **Low cloud is controlled by neither:** the moisture does not cross the scheme's
      near-surface RH_c (0.94–0.95) and freeze-dry, and shallower mixing removes 690–780 hPa
      cloud.
    **Options:**
    (1) Diagnose the cloud scheme's near-surface thresholds (RH_c profile, Sundqvist form,
        freeze-dry) against the measured humidity distribution, offline. This is cloud work,
        so it needs the owner's go-ahead.
    (2) Route A, `ri_crit` 0.5 or 0.25 as default: RH toward ICOADS, but P correlation −0.02
        to −0.08. Reruns in the entry, about 30 h of runs.
    (3) Route B, more levels (L30 / L32): P correlation +0.02 and, at L32, tropical CRE −13 W m⁻²
        closer to CERES, but no RH or low-cloud gain, and global P and net TOA fail. It
        reopens every Part II criterion (about 35 h of runs) and SPEC.md's "T42 L26".
    (4) Accept the near-surface humidity and mid-latitude low-cloud deficits as a recorded
        limitation and return to r3.
    (5) Another route of the owner's choosing.

Closed: 1 License — MIT. 3 f32 HS asymmetry — accepted as recorded (2026-11-18). 5 Courant
trigger — replaced (2026-11-18). 6 APE westerlies — known limitation (2026-11-23). 7 P2 bistability — both branches hold at 1361 W m⁻² from a proper cold start (2026-11-24); sweep (record) once the knobs are frozen. 8 P2 G, sea ice
— restated, met (2026-11-18). 9 P3 ranges — vs RRTMG, met (2026-11-18). 10 P3 drift — closed
(2026-11-23). 11 CO₂ term — forcing error accepted, band off (2026-11-22). 12 Equilibrium test —
10-year version, ±0.3 W m⁻² (2026-11-21). 14 Decisions of 2026-11-23 — record, history file.
