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
| P4 Clouds, Earth-like energy balance | 2026-11-22 (PR #8, draft): gate with frozen knobs: 1, 2 and 5 met; 3: shape met, cover 0.497; 4: 2×CO₂ +11.1 K. Question 11 settled: CO₂ band refits failed the adoption conditions, the semi-gray CO₂ forcing error (+8 %/+29 %) and excess stratospheric cooling are an accepted limitation. Question 13: shallow convection off by default (regress identical); running-mean cloud (a) failed offline; both Frierson shallow treatments reproduce the paper in its slab setup but collapse the APE Hadley cell (−70 %) with prescribed SST. Question 13 (b), 2026-11-22/23: Slingo (1987) anvil option implemented (off by default); T21 Earth tropical net CRE −41 at best against CERES −14.6; slab pair with it ΔT 9.6 K but tropical SW cloud feedback larger (+1.08 against +0.81); review round 2026-11-23: Earth tropical excess = boundary-layer stratiform cloud (−27.7 of 27.9 W m⁻²); fixed-SST factorial reproduces 86 % of the slab loss (SST 79 %, CO₂ 16–21 %); owner chose Slingo (1987) low cloud (2026-11-23): implemented, off by default; separating run: the slab SST pattern carries 85 % of the SST-driven loss; **stopped at owner step 3**: L26 does not resolve the trade inversion that Slingo's eq. (8) needs (question 13). Heat-transport pair: 400 ppm 306.5 K ice-free, not equilibrated (trend −0.037 K/yr); 800 ppm failed the humidity envelope at 311 K; no warming estimate. Question 6 closed as a limitation; 7 to be rerun before P6. 2026-11-23/24: owner chose (vi)/(A), "slingo_lts" implemented (off by default); **stopped at step 4**: best tropical net CRE with criteria 1–2 and cover ≥ 0.5 is −39.8 to −42.2 against CERES −14.6 (question 13) |
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

## P4 state (as of 2026-11-23 evening)

**Frozen knobs** (2026-11-21, `docs/tuning.md`, gate binary `out/p4/toyclim_frozen`): freeze-dry on,
RH_c 0.95 / 0.90, β 0.03; shallow convection, "anvil", `deep_requires_pt` and the CO₂ band off.
"slingo_lts" low cloud (decision (vi)/(A), 2026-11-23): implemented, off; in step 4 (question 13).

| # | Criterion (T42 Earth AMIP 1 + 5 years; 4: T21 slab aquaplanet) | Measured, frozen knobs | State |
|---|---|---|---|
| 1 | Albedo 0.27–0.33; OLR 228–250; net TOA ±8 | 0.280; 242.5 W m⁻²; +2.53 W m⁻² | met |
| 2 | SW CRE −30 to −60; LW CRE +15 to +35 | −44.6; +19.9 W m⁻² | met |
| 3 | Cover 0.5–0.75; ITCZ and storm-track maxima, subtropical minima | **0.497**; shape met | **not met** (cover, by 0.003) |
| 4 | 400 ppm T_s 283–293 K; 2×CO₂ warming 1.5–6 K | 288.376 … 288.516 K (**equilibrated**); **2×CO₂ warming 11.06 K** | warming open; P4 closes with it open (owner, 2026-11-23) |
| 5 | Energy closure ≤ 0.5 W m⁻² | 0.31 | met |

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

## Open questions for the owner

Latest ruling each; history in [open-questions.md](progress/open-questions.md), closed ones in
[closed-questions.md](progress/closed-questions.md).

2. **Station time series and a procedural-terrain milestone** (scoping answers in
   `docs/design-decisions.md`; both extend the SPEC). Owner (2026-11-23): no decision now.
4. **Earth-gate thresholds of P5/P6** (2026-11-12). Owner (2026-11-23): unchanged until a P5
   scorecard exists with the final physics.
7. **P2 bistability: the ice-covered start melts** (P2 acceptance 6). Owner (2026-11-23): the test
   setup is suspect; rerun 12 years from a properly cold state (mixed layer at freezing, cold
   atmosphere, snow-covered ice) when the machine has room. Not a P4 criterion; does not block P4
   or P5; must be settled before P6.
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
    **Result (2026-11-24): stopped at step 4** (entry above). The low-cloud change works where
    it acts (gate: tropical net CRE −51.6 → −29.0; law: deck stratus 0.32–0.38) but the rest of
    the excess is bright cloud in the ascending tropics. **Owner, choose:** (1) carry the best
    criteria-1–2 variant ((c) + `water_max` 2e-4 + `rh_crit_upper` 0.85: cover 0.529, tropical
    −42.2) to the T42 pass and record the tropical excess as a limitation for the P5 convection
    item (recommended); (2) allow `water_max` below its range (2e-4) or a separate cap for
    convective cloud, then rerun step 4; (3) restate the 15 W m⁻² condition; (4) keep the frozen
    knobs and close P4 as decided without the low-cloud change.

Closed: 1 License — MIT. 3 f32 HS asymmetry — accepted as recorded (2026-11-18). 5 Courant
trigger — replaced (2026-11-18). 6 APE westerlies — known limitation (2026-11-23). 8 P2 G, sea ice
— restated, met (2026-11-18). 9 P3 ranges — vs RRTMG, met (2026-11-18). 10 P3 drift — closed
(2026-11-23). 11 CO₂ term — forcing error accepted, band off (2026-11-22). 12 Equilibrium test —
10-year version, ±0.3 W m⁻² (2026-11-21). 14 Decisions of 2026-11-23 — record, history file.
