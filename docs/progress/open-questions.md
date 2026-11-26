# Progress archive: Full history of the owner questions still open (2, 4, 7, 13) and the 2026-11-23 decision record (14); the current ruling of each is summarised in docs/progress.md

Moved unchanged from `docs/progress.md` on 2026-11-23 (old line ranges 2591–2592, 2594–2596, 2618–2621, 2703–2768, 2967–3135, 3136–3278); newest at the bottom. Summary in `docs/progress.md`.

## Open questions for the owner

2. Owner scoping answers are in `docs/design-decisions.md`. Two of them extend the SPEC and
   need a go-ahead before implementation: station time-series output (proposed for M2,
   SPEC §6 addition) and a procedural terrain milestone (after M9).
4. **Earth-gate thresholds** (2026-11-12): the numbers in SPEC.md §9.6 P5/P6 were set from
   judgement of what intermediate-complexity models achieve, before any Earth run exists.
   They are binding for sessions; the owner may want to revise them after seeing the first
   `earth_amip` scorecard (recorded without thresholds at the end of P2).
7. **P2 bistability: one climate, with ice to ~45°** (2026-11-16; P2 acceptance 6).
   - **What fails:** the T21 aquaplanet (slab 20 m, sea ice, seasons, 1361 W/m², 12 years)
     reaches the same climate from both starts (`cases/bench/p2_bistability/`):
     - global T_s 279.3 K and global sea-ice cover 0.29 in both;
     - annual-mean sea ice reaches 42°N (concentration 0.2) and 47°S (0.55), full cover
       poleward of ~52°.
     - So the warm start does not stay ice-free equatorward of 50°, and the ice start does
       not stay ice-covered (it deglaciates in ~4 years).
     - T_s trend over the last 5 years is −0.13 / −0.07 K/yr (warm / ice start).
   - **Figures:** `docs/figures/P2/bistability_1361.png`, `bistability_1361_ts.png`.
   - **Likely candidates** (none tested yet):
     - the uniform TOA reflectance 0.23 (cloud stand-in) applies over sea ice too, on top of
       the ice albedo, so high latitudes absorb too little. Real clouds over ice add little
       to the albedo;
     - no ocean heat transport: the q-flux is zero in this case;
     - gray radiation with the Earth-shaped τ(φ) and no water-vapour feedback; P3 replaces
       it.
   - **Options:**
     - (a) keep the criterion and retest after P3/P4 (radiation, clouds), as for question 6;
     - (b) diagnostic runs, not adopted as a setting: the q-flux on, and the reflectance
       only over open ocean and land. This would be a physics change of the cloud
       stand-in, not tuning;
     - (c) the owner revises the criterion for the gray stand-in physics.
   - **Recommendation:** (b) as data, then (a).
   - **Owner decision (2026-11-18): (a), retest after P4 clouds.** The q-flux pair (below)
     ruled out the ocean heat transport.
   - **Instellation sweep** (acceptance 6 *record*): from the warm state (year 12) at 98, 96,
     94 and 92 % (luminosity scaled), 6 years each, climatology over years 15–18
     (`out/p2/sweep/`, binary f59dd7e). Ice edge = most equatorward latitude with
     annual-mean concentration ≥ 0.15:

     | Instellation | Global T_s | Global ice cover | Ice edge N / S | T_s trend, last 5 yr |
     |---|---|---|---|---|
     | 100 % (bistab_warm) | 279.3 K | 0.29 | 41.5° / 41.5° | −0.19 K/yr |
     | 98 % | 277.5 K | 0.30 | 41.5° / 41.5° | −0.01 K/yr |
     | 96 % | 275.4 K | 0.33 | 41.5° / 41.5° | −0.24 K/yr |
     | 94 % | 272.8 K | 0.37 | 36.0° / 36.0° | −0.44 K/yr |
     | 92 % | 270.3 K | 0.40 | 36.0° / 36.0° | −0.70 K/yr |

     - No run reaches a snowball within 6 years, and the tropics stay ice-free in all of
       them. The 94 % and 92 % runs are still cooling fast, so the freeze point is not
       found. It lies at or above 92 % if those runs keep going, and is undetermined
       otherwise.
     - Recording the transition would need longer runs (≥ 15 years) at 94–90 %. These are
       not started: question 7 is open and its physics may change.
   - **Update 2026-11-18, ocean heat transport on** (option (b), first half; 12 years,
     `out/p2/q7/`):
     - still one climate: warm / ice start 278.8 / 278.6 K, ice cover 0.283 / 0.287, edge
       41.5° in both;
     - the ice start deglaciates in ~4 years, as before;
     - the heat transport is ruled out as the cause.
     - Under semigray radiation without a top reflectance, the P3 aquaplanet has the same
       41.5° edge but then slides colder (question 10).
   - **P4 note (2026-11-18):** with clouds, the 400 ppm slab aquaplanet keeps its annual ice
     edge near 58°, which is what the warm branch needs. The ice-covered start has not been
     run with clouds, so acceptance 6 stays open until step 6 of the P4 completion plan.
   - **Retest after P4 (2026-11-21):** the ice-covered start with the frozen P4 physics
     (`cases/bench/p4_retest/bistab_ice.toml`, 12 years) deglaciates again.
     - The 3 m tropical ice melts in year 1; global T_s goes 248 K (month 7) → 287.3 K
       (year 12), still rising 0.3 K/yr.
     - Years 8–12: ice cover 0.153, edge at 58°N / 53°S.
     - Figure `docs/figures/P4/bistab_ice_start.png`.
     - Candidate cause: bare sea-ice albedo of about 0.5 (CAM6 band values) at low latitudes.
       No code defect found. Still open.
   - **Owner decision (2026-11-23):** the setup is suspect; rerun from a properly cold state
     before P6 (question 14). Not a P4 criterion.
13. **P4: the convective cloud diagnosis gives a strong positive tropical cloud feedback**
    (2026-11-21; P4 acceptance 3 and 4).
    - **Evidence:** entry "P4 completion plan, steps 1–5", diagnosis section, and figures
      `tropical_rh_cloud_vs_T.png` and `tropical_cl_cwp_vs_T.png`.
      - Under 2×CO₂ the tropical layer cloud is unchanged against temperature.
      - The convective cloud, 0.125 ln(1 + P) of each column's instantaneous precipitation,
        falls by half because convection becomes intermittent: median tropical
        precipitation 1.93 → 0.08 mm/day, 99th percentile 36 → 87, CAPE 508 → 1015 J kg⁻¹.
      - Tropical cover falls from 0.70 to 0.57, and the tropical SW CRE weakens by
        24 W m⁻².
    - **Options:**
      - (a) Base convective cloud on a time-averaged convective mass flux or precipitation,
        e.g. a running mean over a few hours kept in the cloud memory, so that it measures
        how often a column convects rather than single steps.
      - (b) Tie anvil and detrained cloud to convective detrainment (the convective top), and
        widen the RH window aloft (RH_c nearer 0.8), with the polar excess handled
        separately (freeze-dry, or a polar-specific term).
      - (c) Look first at why the SBM convection becomes intermittent: CAPE doubles, so the
        trigger or the relaxation time may be the deeper cause.
      - Whichever is chosen: single-column tests, screening with the fixed-SST +4 K pair
        (`cases/bench/p4_feedback/`, about 1 h at T21), then one tuning pass at T42 aiming
        for cover ≥ 0.55, then the gate with longer slab runs (≥ 25 years after a change).
    - **Recommendation:** after the LW refit (question 11): (c) as a diagnostic, then (a),
      which is the smallest change. (b) only if (a) leaves the cover/shape conflict at T42.
      Not choosing anything by its effect on the warming; screening uses the fixed-SST
      feedback as a diagnostic, and the tuning targets stay Earth-only.
    - **Owner decision (2026-11-21): (c) first, with a named suspect.**
      - SBM skips columns with CAPE > 0 where the adjustment would moisten (P_q ≤ 0).
        SPEC.md §9.4.4 asks for Frierson's shallow-convection treatment, which was never
        implemented. Such columns may build CAPE and then fire in bursts.
      - Test: the share of tropical column-steps with CAPE > 0 and P_q ≤ 0 at 400 and
        800 ppm, and the convective vs large-scale share of tropical rain.
      - If confirmed: implement the paper's shallow scheme (fetched, transcribed,
        single-column tests). That closes a spec gap; it is not tuning.
      - If not confirmed: (a), with the averaging time fixed on physical grounds (a few
        hours, anvil lifetime), not tuned.
      - Also compare the mean convective cloud between T21 and T42 Earth runs: a concave
        function of instantaneous rain depends on resolution and time step, which may
        explain part of the T42 cover loss (0.497).
      - (b) on hold; no tuning at T42 until this is settled.
      - Sequence: check that the fixed-SST +4 K pair reproduces the tropical SW cloud term
        (otherwise it cannot serve as the screen); refit CO₂ (question 11); intermittency
        test and the convection fix; screen with the +4 K pair; tune once at T42; gate with
        slab pairs of ≥ 25 years.
    - **Result (2026-11-21/22):** confirmed (P_q ≤ 0 skips 0.66 → 0.81 of tropical
      column-steps, CAPE doubles); Frierson (2007) "shallower" implemented with an SCM
      test, default on (commit 7a78d63). Untuned, it degrades accepted behaviour: APE
      Hadley maximum 16.3 → 4.9 ×10¹⁰ kg s⁻¹ and the equatorial rain peak flattened; Earth
      T42 zonal precipitation correlation 0.820 → 0.663, storm index G 1.90 → 2.04 (fail),
      cover 0.497 → 0.450, net TOA +11.3 W m⁻². It also leaves 17 % of warm-state tropical
      column-steps skipped and tropical cover still falls 0.12 under 2×CO₂.
    - **Owner, choose:** (i) keep shallow on and re-tune (the shallow scheme has no free
      parameters in the paper beyond τ, so the re-tune would be the cloud knobs only, and
      the Hadley/ITCZ change would remain); (ii) shallow off by default and go to (a), the
      running-mean convective cloud with a physical averaging time; (iii) both, judged by
      the +4 K screen rebuilt at about +10 K (the review's suggestion, to reach 300–310 K).
    - **Recommendation:** (ii) now, with (a) and the +10 K screen; keep the shallow scheme
      as an option. Not chosen by its effect on the warming: the reason is the APE and Earth
      precipitation regression.
    - **Owner decision (2026-11-22):** shallow off by default now; test (a) offline first;
      try the reference-humidity treatment on APE; check against the paper; if both fail,
      stop and report.
    - **Result (2026-11-22 afternoon):** shallow "none" by default, regress identical to the
      pre-shallow code. (a) fails offline: a 3–6 h running mean keeps 66–79 % of the
      convective-cloud fall (0–15°: −34 % instantaneous, −21 to −27 % with 3–6 h, −13 to
      −19 % with 24 h). "qref" on APE T21: Hadley 5.5 against 15.6 (shallower 4.7). In the
      paper's slab setup our shallower/qref reproduce Table 1 (Hadley ×0.73 against ×0.74,
      equatorial P ×0.78 against ×0.75), also at T21: the APE collapse comes with prescribed
      SST, not from the implementation, resolution or the boundary layer.
    - **Owner, choose:** (b) anvil/detrained cloud tied to the convective top (with shallow
      off), or tuning the convection knobs (τ, RH_SBM) with shallow on. **Recommendation:**
      (b). The shallow scheme is correct but its prescribed-SST effect on the Hadley cell
      (−70 %) would have to be tuned away through the very knobs that set the ITCZ, and the
      offline test shows the cloud fall is a property of the instantaneous closure on
      intermittent convection, which (b) replaces with a detrainment-based measure. Not
      chosen by its effect on the warming.
    - **Owner decision (2026-11-22 evening):** (b) after a diagnosis; shallow off.
    - **Result (2026-11-22/23):**
      - *Diagnosis held:*
        - tropical net CRE −43.1 against CERES EBAF −14.4 W m⁻²;
        - the convective part carried 85 % of the tropical 2×CO₂ net CRE change;
        - convective in-cloud path 1.3–2.2 kg m⁻².
      - *Scheme:* the Slingo (1987) core + anvil geometry with a capped convective water
        path, implemented as `convective = "anvil"`, off by default.
      - *T21 Earth tuning:* tropical net CRE at best −41.0 (from −51.6). The remaining
        excess is in the RH-based tropical cloud, and the shape row still fails.
      - *+10 K fixed-SST pair:* does not reproduce the tropical cloud loss.
      - *Slab pair with the best variant (25 years, not equilibrated by the trend test):*
        - ΔT 9.61 K against 11.06;
        - but tropical SW cloud term +1.08 against +0.81;
        - the smaller ΔT comes from the 400 ppm state being 6.6 K warmer and ice-free.
      - **The redesign did not reduce the tropical cloud feedback.**
    - **Owner, choose:**
      - (i) measure the convective/stratiform split of the anvil pair (1 year
        `cre_split` from the year-70 restarts, about 20 min) before deciding anything;
      - (ii) treat the RH-based tropical cloud: its 2×CO₂ loss and its too-thick
        present-day state (tropical SW CRE −70 against −44) are now the larger part;
      - (iii) accept the tropical cloud feedback as a limitation of the diagnostic cloud
        scheme, keep "column", and go to the T42 re-tune and the gate with the frozen knobs.
    - **Recommendation:** (i) first, because it is cheap and tells whether (ii) is the right
      target. Then (iii) unless (i) points at a single, published fix: a diagnostic RH
      cloud scheme cannot be expected to produce a well-constrained cloud feedback. Not
      chosen by its effect on the warming.
    - **Owner decision (2026-11-23, review adopted):** (i) plus a diagnostic round (fixed-SST
      factorial on the slab SSTs, Earth low-cloud split, slab pair with ocean heat
      transport); stop and propose.
    - **Result (2026-11-23, entry "review round"):**
      - anvil pair: the stratiform cloud carries 58 % of the tropical net CRE change;
      - frozen pair against its equilibrated 800 ppm state: 51 % stratiform; step 1.3's 85 %
        convective came from an unequilibrated 800 ppm state (rule (ii) would have failed);
      - fixed-SST factorial reproduces 86 % of the slab loss; SST change 79 % (convective
        cloud), CO₂ alone 16–21 % (boundary-layer stratiform cloud);
      - Earth T42: the boundary-layer stratiform cloud (σ ≥ 0.7, mostly σ 0.87, peak at
        5–20°) has a tropical net CRE of −27.7 W m⁻², equal to the excess against CERES (27.9).
      - Heat-transport pair: 400 ppm 306.5 K, not equilibrated; 800 ppm stopped at the humidity envelope (next entry).
    - **Owner, choose:**
      - (iv) implement the Slingo (1987) low cloud (eqs. 6–9: RH cloud suppressed under
        subsidence, inversion-strength boundary-layer cloud), which needs ω in the column
        context; SCM tests; screen with the factorial S400 C400 / S800 C800 and Earth T21
        against the CERES bands; T42 re-tune with the CERES bands as targets;
      - (iii) accept as a limitation, with this attribution, and go to the T42 re-tune and gate
        with the frozen knobs (CERES bands added as targets);
      - optional first, either way: the SST warming/pattern separation run (S400 + uniform
        12.2 K, 400 ppm, ≈ 1.3 h), and the subsidence share of the 5–20° low cloud on saved
        states.
    - **Recommendation:** (iv). It is a published treatment, it targets the cloud layer that
      carries the whole Earth excess, and the paper names this subtropical excess as what
      its ω dependence removes. It is a physics change justified by the Earth CERES bands,
      not by the warming. The convective cloud's SST response stays as it is, and (iv) will not
      remove it: the 2×CO₂ row may stay outside 1.5–6 K. Wait for item 4 before deciding.
    - **Owner decision (2026-11-23, closing the review session): (iv), Slingo (1987) low cloud**
      (eqs. 6–9, option off by default, ω passed to the column physics, SCM tests; L26
      inversion check before tuning; T21 exploratory tuning on the CERES bands and criteria
      1–3, revisiting `rh_crit_surface`; T42 pass aiming at cover ≥ 0.55; then freeze and gate).
      Shallow convection stays off by default. `deep_requires_pt` stays off, recorded as a
      deviation with negligible effect. "anvil" stays off and enters the T42 re-tune only if
      the tropical LW CRE is still short of CERES after the low-cloud change; it is not
      adopted (its 400 ppm aquaplanet, 295.1 K, is outside 283–293 K). The separating run is
      done first, record only. Interpretation adopted: the Earth tropical error is the
      boundary-layer stratiform cloud (an Earth-target defect, hidden in P4's global means by
      too little cloud elsewhere); the tropical 2×CO₂ cloud loss is an SST response; the low
      cloud fix is justified by CERES, not by the warming.
    - **Result (2026-11-23 afternoon, entry "Owner steps 1–3"):** steps 1–2 done (separating
      run: the slab SST pattern carries 85 % of the SST-driven loss, through the stratiform
      cloud; Slingo low cloud implemented, off by default, byte-identical). **Step 3 fails, so
      stopped:** L26 does not resolve the trade inversion (5–20° most stable layer −0.06 K/hPa,
      as over the warm pool); in the stratocumulus decks the ≈ 14 K inversion is spread over
      two layers (σ 0.93 → 0.79), each just past Slingo's zero point, so eq. (8) gives a deck
      cover of 0.09–0.19 at T42 (0.04–0.09 with eq. 9) and none in the trades. The eq. (7)
      subsidence gate is resolved (ω ≥ 0 in 55–91 % of snapshots over the decks).
    - **Owner, choose:**
      - (v) keep Slingo's form and re-derive the eq. (8) constants for L26 — the paper says
        they depend on vertical resolution (p. 923) and gives a second set for another model
        (Slingo 1980, not fetched) — by an Earth target (CERES bands and deck cover); this
        rescales a stability the model barely resolves, so the deck cloud would rest on a
        fitted threshold;
      - (vi) replace eq. (8) by a bulk-stability stratus law (Klein & Hartmann 1993, stratus
        amount linear in θ700 − θ_s; as I recall, NCAR CCM3/CAM3 use it for marine stratus — not verified): LTS is resolved at
        L26 (20–22 K in the decks against 10.5–12 K elsewhere). Needs the paper (HTTP 403 from
        AMS; the owner may have a copy) or the CAM source for the constants; eqs. (6)–(7) stay;
      - (vii) add boundary-layer levels (e.g. 3–4 more between σ 0.75 and 0.95) so that the
        inversion falls in one layer: a dynamical-core and regression change, all accepted
        gates rerun;
      - (viii) eqs. (6)–(7) only (ω-gated RH low cloud, no inversion cloud) as a screen of how
        much of the tropical excess the subsidence gate removes; the subtropical cover, already
        0.33 against about 0.6, would fall further.
    - **Recommendation:** (vi) with eqs. (6)–(7): it uses the stability measure L26 does
      resolve and is the published successor used at similar resolutions; (v) as the fallback
      if the paper cannot be obtained. Not (vii) in P4. Nothing implemented.
14. **Owner decisions of 2026-11-23 closing the review session** (recorded before starting).
    - **Q6** (APE westerlies end at 51°): closed as a known limitation; criterion unchanged.
      It fails the same way with gray, semi-gray, clouds and T63. The Earth jet and
      sea-level pressure rows of P5 are the binding circulation test. No more retests in P4.
    - **Q7** (ice-covered start melts): the test setup is suspect. Verify the initial state
      of `cases/bench/p4_retest/bistab_ice.toml` (3 m bare ice; ocean and atmosphere appear
      to start warm), restart from a properly cold state (mixed layer at freezing, cold
      atmosphere, snow-covered ice) and rerun 12 years when the machine has room. Not a P4
      criterion; does not block P4 or P5; must be settled before P6.
    - **Q10** (aquaplanet drift): closed; the slab aquaplanet with clouds was steady for 30
      years.
    - **Q11** (CO₂ forcing): stays closed as an accepted limitation; the band stays off.
    - **Q12** (equilibrium test): as restated; slab pairs run ≥ 30 years after any scheme
      change.
    - **Q13**: (iv), above.
    - **Criterion 4** (2×CO₂ warming): P4 closes with this row open; the criterion is not
      weakened and its configuration is not changed. P4 is closed when (a) criteria 1, 2, 3
      and 5 pass at T42 after the re-tune, with the CERES zonal bands as added targets;
      (b) the 400 ppm slab aquaplanet is within 283–293 K; (c) the warming is measured once
      with the final settings and recorded with its attribution (convective cloud loss on
      warm SST from the idle-column intermittency of the convection scheme; complete sea-ice
      melt on a slab without ocean heat transport; the semi-gray CO₂ forcing, +8 % / +29 %).
      Nothing is tuned to move the warming. P5 may then start.
    - **Heat-transport slab pair:** record only, into the attribution. It does not change the
      configuration of criterion 4, which is revisited with the P6 ocean together with a
      recheck of the warming.
    - **Convection intermittency and the Hadley collapse with shallow convection:** first
      work item of P5 (its precipitation row already fails, 0.82 vs 0.85); must be resolved
      before P6.
    - **Q4** (P5/P6 thresholds): unchanged until a P5 scorecard exists with the final physics.
    - **Q2** (station series, procedural terrain): no decision now.
    - **Stop and report (do not work around) if:** the L26 inversion check fails; no T21
      variant brings the tropical net CRE within 15 W m⁻² of CERES while keeping P4 criteria
      1–2 and cover ≥ 0.5; the T42 pass cannot meet criteria 1, 2, 3 and 5 together; the 400
      ppm slab aquaplanet leaves 283–293 K with the frozen T42 knobs; anything accepted
      regresses in `scripts/regress`. Machine budget: ≤ 16 threads in total.
    - **Owner decision on the low cloud (2026-11-23 evening, follows 446131e):**
      - Heat-transport pair closed, record only (400 ppm 306.5 K, ice-free, not equilibrated;
        800 ppm failed): no warming estimate, no further heat-transport runs in P4; criterion
        4 keeps its configuration and stays open as decided.
      - Separating run recorded (85 % of the cloud loss follows the slab SST pattern, uniform
        warming +2.2 W m⁻²); it goes to the P5 convection item and the P6 ocean; nothing in
        P4 acts on it.
      - Stopping at the inversion check was correct; eq. (8) is not used.
      - **Option (vi) chosen:** keep the eq. (7) subsidence gate on the RH low cloud; replace
        eq. (8) by the lower-tropospheric-stability stratus law of Klein & Hartmann (1993, J.
        Climate 6, 1587–1606) as a new option value (`low_cloud = "slingo_lts"`), off by
        default, byte-identical when off; the eq. (8) variant stays in the code, off, with a
        doc note that L26 does not resolve it. Rejected: (v); (vii) noted as a candidate for a
        later milestone; (viii) is run as one diagnostic variant.
      - Source: the CAM3 description (Collins et al. 2004, NCAR/TN-464+STR), Rasch &
        Kristjánsson (1998) if needed, and one more try at the 1993 paper. The owner's
        remembered form, min(1, max(0, 0.057·LTS − 0.5573)), is not to be used unverified.
        Transcribe constants, surface θ, ocean-only restriction, vertical placement, humidity
        factor; state which document they come from. If neither source can be fetched, stop.
      - Implementation: maximum with the RH cloud in the same layer; in-cloud water as other
        stratiform cloud; freeze-dry and sigma_min unchanged; SCM tests; knobs in
        docs/tuning.md (published values as defaults, moved only inside a stated range and
        only against CERES). Offline check on the T42 Earth output first; **stop if the decks
        come out under 0.3**.
      - Recorded fact, not a criterion: this law makes low cloud rise with stability, which
        generally rises under warming; its effect on the 2×CO₂ warming is measured once at
        the gate and plays no role in tuning.
      - Then steps 4–8 of the earlier plan: step 4 T21 variants (a) frozen, (b) gate only
        (diagnostic), (c) gate + LTS law with published constants, (d) as (c) with
        `rh_crit_surface` lowered from 0.95 in steps; step 5 the two matching factorial cases
        (diagnostic); step 6 one T42 pass, cover aiming ≥ 0.55, then freeze; step 7 gate;
        step 8 closing entry, then stop. Q7 rerun when the machine has room. Stop conditions as
        above (with the offline deck check replacing the inversion check).
      - **Readings adopted (mine, flagged to the owner):** in (b)–(d) the gated RH low cloud
        uses the existing Sundqvist RH_c profile (`low_rh = "sundqvist"`), so that
        `rh_crit_surface` in (d) acts on the low cloud (with `low_rh = "slingo"` the σ ≥ 0.8
        cloud uses `low_rh_crit` instead); (b) is the new option with the LTS cloud switched
        off by a flag; "the two matching factorial cases" are S400/C400 and S800/C800.
    - **Result of the offline check (2026-11-23 evening): stop condition reached, owner
      choice needed.** Sources: the CAM3 technical note could not be fetched (cesm.ucar.edu
      redirects to www2, HTTP 401 / TLS error; opensky.ucar.edu serves a bot check); the AMS
      PDF of Klein & Hartmann (1993) returned 403 again. **Fetched and read:** the CAM
      implementation, ESCOMP/CAM tag `cam_cesm2_2_rel`, `src/physics/cam/cloud_fraction.F90`
      (`cldfrc`) and `bld/namelist_files/namelist_defaults_cam.xml` (no licence file found
      in the repository; no code copied, constants transcribed). It states
      strat = min(1, max(0, ocnfrac·((θ(k700) − θ_s)·0.057 − 0.5573))) — the owner's
      remembered constants are confirmed — with θ_s = (SST − Γφ_s/g)(p₀/p_s)^κ, only where
      ocnfrac > 0.01 (ocean without sea ice), k700 the level nearest 700 hPa by reference
      pressure; **and a trigger**: the cloud exists only where the most stable adjacent pair
      with the lower level at p ≥ `premib` (750 hPa for CAM4 physics, 700 for CAM5/6) has
      Δθ/Δp < −0.125 K/hPa ("nothing triggers unless a stability greater than this minimum
      threshold is found"; surface pair tried if no level pair qualifies); it is placed in
      the lower layer of that pair, limited by max(RH_k, RH_{k+1}), and combined with the RH
      cloud by maximum. The 1993 paper itself was not read.
    - Offline on the T42 Earth output (`tools/lts_check.py`, `out/p4/slingo/earth_inv_t42`,
      74 snapshots; figure `docs/figures/P4/lts_check_inv_t42.png`, T21 in `…_base_t21.png`):

      | region (T42) | LTS [K] all / peak | law alone all / peak | trigger fires all / peak | CAM form (trigger + RH) all / peak | without trigger (+ RH) all / peak |
      |---|---|---|---|---|---|
      | Peru | 18.6 / 21.7 | 0.50 / 0.68 | 0.29 / 0.54 | 0.17 / 0.37 | 0.50 / 0.67 |
      | Namibia | 18.3 / 19.6 | 0.49 / 0.56 | 0.38 / 0.60 | 0.22 / 0.36 | 0.47 / 0.53 |
      | California | 17.7 / 20.8 | 0.45 / 0.63 | 0.23 / 0.32 | 0.12 / 0.22 | 0.45 / 0.62 |
      | Canary | 16.1 / 19.7 | 0.36 / 0.57 | 0.17 / 0.32 | 0.09 / 0.20 | 0.36 / 0.56 |
      | Australia | 13.3 / 15.9 | 0.22 / 0.35 | 0.14 / 0.12 | 0.03 / 0.05 | 0.22 / 0.35 |
      | trades 5–20N / 5–20S | 12.1 / 12.6 | 0.14 / 0.16 | 0.15 / 0.11 | 0.03 / 0.03 | 0.14 / 0.16 |
      | warm pool | 10.9 | 0.07 | 0.04 | 0.01 | 0.07 |

      T21 is weaker (law alone at peak 0.52–0.58 in the four main decks; CAM form 0.04–0.23).
      Figures (opened): the law alone has clear maxima along the west coasts of the Americas
      and Africa, sharper at T42, and a weaker band at 40–60°N over the oceans; the trigger
      fires mostly over the Southern Ocean (shares 0.6–0.9 at 45–60°S) and in thin coastal
      strips, rarely inside the deck boxes; so the CAM form puts its stratus there and leaves
      the decks at 0.1–0.4. The RH limit barely binds (without trigger ≈ law alone).
    - **Reading:** the law itself gives the expected deck cover (0.5–0.68 at peak, owner's
      expectation about 0.6; trades 0.14–0.16, slightly above the expected 0.05–0.13), but the
      published implementation gates it with a Δθ/Δp threshold of the same kind as Slingo's
      eq. (8), which L26 does not reach in the decks (mean most stable layer −0.11 to −0.126
      at peak). As transcribed, the decks come out under 0.3 (all seasons 0.09–0.22; peak
      0.20–0.37): **stop condition, nothing implemented in the model.**
    - **Owner, choose:** (A) the law without CAM's trigger (cloud placed in the most stable
      layer below 750 hPa as transcribed, RH limit, ocean only) — a stated deviation from the
      CAM implementation, justified by the same resolution argument that set eq. (8) aside;
      recommended. (B) keep a trigger but at a value L26 reaches (e.g. −0.07 K/hPa) — a fitted
      threshold, rejected for the same reason as (v). (C) the CAM form as published — decks
      0.1–0.4; would not reach the owner's expected deck cover.
    - **Owner decision on the form of the law (2026-11-23 evening, follows 6789a02): (A).**
      The stratus law without the stability trigger, everything else as in the CAM source:
      θ_s from the SST, open ocean only (no land, no sea ice), placed in the most stable layer
      below 750 hPa, capped by the humidity there, combined with the RH cloud by maximum; the
      Slingo subsidence gate on the RH low cloud stays. Rejected: (B) fitted threshold, as
      (v); (C) decks at 0.1–0.4. **Stated deviation:** the −0.125 K/hPa trigger is omitted
      because L26 does not resolve layer inversions (same argument as for eq. 8). The
      constants were read from the CAM source and a forum quotation; neither the 1993 paper
      nor the CAM3 technical note was read. CAM is a reference for the formula and constants
      only: code written independently, no source copied. Defaults slope 0.057, intercept
      −0.5573; no trigger knob. Before 3-D: extend the offline check (law alone) to the
      Southern Ocean 45–60°S, North Pacific and North Atlantic 40–60°N, open ocean poleward
      of 60°, and the zonal low cloud against CERES cover by band; stop only if the law gives
      more than 0.8 year-round over a whole band. Then steps 4–8 as before; the deck stop
      condition refers to the law alone and is satisfied. Known limitations for the P4
      closing entry: (1) L26 does not resolve boundary-layer inversions, low cloud rests on a
      bulk-stability regression fitted to Earth's oceans, more boundary-layer levels a
      candidate for later (owner decides); (2) for non-Earth cases the law is an
      extrapolation: from the first exoplanet benchmark on, each benchmark is run with
      low_cloud on and off, both reported, nothing tuned on them; (3) the law is tied to SST
      and open ocean, recheck when the P6 ocean replaces prescribed SST.

    - **Owner decision after step 4 (2026-11-24, follows 852ca60)**, recorded under 13/14:
      (1) the stop was correct; the low-cloud change is adopted for the remaining work: the
      subsidence gate and the stability law with the published constants (slope 0.057,
      intercept −0.5573); the intercept range set by Claude has no source, the −0.65 variant
      is not carried forward. (2) The remaining error is brightness, not amount: clouds less
      bright per unit cover, and more cover. (3) Carry forward only variants with T21 cover
      ≥ 0.56 (frozen knobs gave 0.534 at T21 and 0.50 at T42). Step 4b: a diagnostic on
      variant (c) — (a) the 10°S–10°N SW and LW CRE split by layer group and convective /
      stratiform, LW CRE against CERES; (b) liquid and ice water path by zonal band against
      a fetched observed climatology (tropics, subtropics, Southern Ocean); (c) the cloud
      optics' effective radii and any sub-grid inhomogeneity reduction. Lever by rule: water
      path above about 1.5× observed → extend the `water_max` range downward or add a separate
      cap for convective cloud, the observed water path the stated target; within about 1.5×
      → the plane-parallel inhomogeneity reduction (Cahalan et al. 1994), fetched, off by
      default, byte-identical when off, SCM test; tropical LW CRE short of CERES by more than
      8 W m⁻² → the anvil option may enter. Then one T21 round (Earth AMIP 1 + 1 years, at
      most 8 variants, all logged) from gate + law with the lever, restoring cover with
      `rh_crit_surface`, `rh_crit_upper` downward and a weaker gate within Slingo's values;
      targets CERES bands, criteria 1–2, T21 cover ≥ 0.56. The 15 W m⁻² line is retired:
      after the round, the variant meeting criteria 1–2 with cover ≥ 0.56 and the smallest
      CERES band error goes on to steps 5–8; if none has cover ≥ 0.56, the best-cover
      variant meeting criteria 1–2 goes to T42 and the T42 stop condition decides. Remaining
      tropical excess goes into the closing entry as a known limitation for the P5
      convection item. Unchanged stops: T42 cannot meet 1, 2, 3 and 5 together; 400 ppm slab
      aquaplanet outside 283–293 K; any regression in `scripts/regress`; a source that cannot
      be fetched. Q7: run now in the background (cold start, 12 years). progress.md at 556
      lines is fine (500 was a cleanup target). Rules unchanged; nothing is chosen by its
      effect on the 2×CO₂ warming.

    - **Result of step 4b and steps 5–7 (2026-11-24 evening)** (progress.md entry "Question 13
      after step 4"): diagnostic → water path 2.5× Cloud_cci in 10°S–10°N, rule case 1
      (`water_max` range extended to 1e-4); LW CRE short by 7.5 (< 8, no anvil). T21 round of 8:
      r6 (`water_max` 1e-4, `rh_crit_upper` 0.75) qualified (cover 0.567, band error 9.99).
      Step 5 factorial: tropical net CRE change −14.4 (frozen +18.5), recorded. Step 6 T42 pass:
      cover 0.566, shape row fails. Step 7 gate: T42 criteria 1, 2, 5 and cover met, **shape row
      fails** (no northern storm-track maximum); 400 ppm slab **297.1 K**, ice-free, outside
      283–293 K; 2×CO₂ +3.53 K (800 ppm not equilibrated), recorded with its attribution
      (net CRE −3.73, no sea ice in either state). `scripts/regress`: no regression (APE
      westerlies row −6.85 as before). **Stopped at step 7 on two stop conditions;** options
      in progress.md question 13. Q7 rerun: both branches hold at 1361 W m⁻².
    - **Owner decision after step 7 (2026-11-24, follows 923fb1c):**
      - The stop was correct; r6 is not adopted.
      - "slingo_lts" with the published constants, and the lower water cap, stay as the basis.
      - The T21 cover ≥ 0.56 rule is withdrawn; the +3.53 K is recorded only; Q7 is closed.
      - Option 4 as a screen: A, the polar profile; B, r1, r3 and r5 at T42 1 + 1 years and in
        the 400 ppm slab for years 46–60 from the frozen gate restart; C, the gate with the best
        passer; D, otherwise stop and propose with fetched sources.
      **Result (2026-11-25): none passes, stopped at D.**
      - T42: r3 passes (cover 0.548, shape met, band error 10.19); r1 and r5 fail the shape row.
      - Slab: all three lose their ice and end at 295.6–297.4 K.
      - Step A: the r6 polar increase is upper and middle tropospheric; 8–22 % is above the
        tropopause.
      - At fixed SST the new low cloud adds +14.8 W m⁻² of global net CRE, mostly as tropical
        and subtropical SW.
      - Proposals: EIS (Wood & Bretherton 2006) and a cloud-only −10 to −40 °C phase ramp
        (CAM3); or close with the frozen knobs; or an owner ruling on 283–293 K for this scheme.
    - **Owner decision after the screen (2026-11-25, follows ad03af1), recorded under 13:**
      - Interpretation: the frozen aquaplanet's 288.5 K rested on a tropical net cloud effect of
        −64 W m⁻² against −14 on Earth; the Earth run is the anchor. Nothing is tuned to cool
        the aquaplanet. Criterion 4's 283–293 K is unchanged and not marked met.
      - r3 (`slingo_lts`, `water_max` 1.5e-4, `rh_crit_upper` 0.80) is the candidate. The
        closing condition "400 ppm slab within 283–293 K" is withdrawn; row 4 stays open with
        the owner and is revisited with P6. P4 closes when the T42 Earth AMIP 1 + 5 year gate
        with r3 passes criteria 1, 2, 3 and 5 and row 4 has been measured once on an
        equilibrated pair and recorded.
      - Option 1 (EIS, Wood & Bretherton 2006) is a P5 candidate after the convection item,
        justified by the Southern Ocean cover (0.61 against 0.87); read the paper directly
        before then. Option 3 cannot close P4.
      - Steps: 1 gate with r3 (T42 1 + 5 years; 400 ppm slab continued to the equilibrium
        test, ≥ 30 years after the change; 800 ppm from the same restart; `tools/feedbacks.py`;
        `scripts/regress`); 2 TRACMIP (Voigt et al. 2016) aquaplanet-control T_s across models;
        3 completion-plan step 6 with r3, record only (APE T42, bistability pair, instellation
        sweep); 4 closing evaluation if criteria 1, 2, 3 and 5 hold (r3 as default, closing
        entry, new open question on 283–293 K). Stop and report if the T42 run fails 1, 2, 3 or
        5 (shape row: profile and margin, no further variants), a slab fails or does not
        equilibrate within 45 years, or regress regresses.
      **Result (2026-11-25 afternoon): stopped at step 1 — the T42 shape row fails.**
      - T42 1 + 5 years: criteria 1, 2, 5 and cover (0.545) met. Northern 40–70° maximum
        0.668 at 68.4°, the band's poleward row (0.005 above 65.6°); cover rises monotonically
        from 40°N and stays at 0.65–0.67 to 80°N. In single years the maximum is local at 65.6°
        in 2 of 5 years. The screen's pass (1 + 1 years) rested on year 2 alone.
      - Slab pair (both equilibrated, ice-free): 400 ppm 295.62 K, 800 ppm 298.57 K, 2×CO₂
        +2.96 K (implied 2.84 K); cloud terms negative (SW CRE −1.11 W m⁻² K⁻¹, mostly 30–60°).
      - `scripts/regress`: no regression. TRACMIP AquaControl 290.4–300.7 K, median 295.1 K.
      - Step 3: cold start stays ice-covered (record); APE T42 not run; sweep stopped after
        about one model year. Step 4 not done; code defaults unchanged.
    - **Owner decision after the r3 gate (2026-11-25 evening, follows a526a57), recorded under
      13 on 2026-11-26:**
      - Interpretation: the stop was correct; the shape row fails as measured and is not ruled
        met (northern maximum on the edge row, local in 2 of 5 years). Row 4 with r3 is
        recorded, not marked met: 400 ppm 295.62 K (ice-free, equilibrated), 2×CO₂ +2.96 K;
        TRACMIP's 290.4–300.7 K (median 295.1 K) goes into the open question on 283–293 K with
        the setup differences; nothing is chosen by these numbers.
      - Owner's reading of the shape failure: the northern polar cap (0.65–0.67) is not the
        outlier; the mid-latitudes are too clear (0.47 at 40.5°N), as is the Southern Ocean
        (0.62 against 0.87). A polar-air treatment would lower the cap and leave both deficits.
      - **Decision: option 3, the EIS law (Wood & Bretherton 2006), brought into P4** (deferred
        to P5 on 2026-11-25). Completion-plan step 4 (polar air) not authorised now; fallback.
      - Step A (existing output only): (a) zonal-mean cover of the r3 T42 run against an
        observed climatology, 30–90° both hemispheres: where short, and whether the northern
        cap is above, at or below the observations; (b) EIS law offline next to the LTS law on
        the r3 T42 output, seasonal and annual, for the five decks, trades, warm pool, N
        Pacific and N Atlantic 40–60°N, Southern Ocean 45–65°S, open ocean poleward of 65°,
        then the implied zonal-mean cover change at 45–65°N against 68–80°N. Stop if (a) shows
        the cap, not the mid-latitudes, is the error, or if (b) does not raise 45–65°N
        relative to the cap.
      - Step B: new option value (e.g. `low_cloud = "slingo_eis"`), off by default,
        byte-identical when off; EIS definition, fit and constants transcribed from the paper
        (verify 0.14 + 0.060 EIS); otherwise as "slingo_lts"; SCM tests as for LTS; knobs with
        source and range in docs/tuning.md, published constants as defaults.
      - Step C: T21 Earth AMIP 1 + 1 years, ≤ 6 variants, from r3 with EIS; rebalance only
        with `water_max` and `rh_crit_upper` within range; targets criteria 1–2, cover, CERES
        bands; shape row not judged at T21.
      - Step D: T42 Earth AMIP 1 + 3 years for the best one or two; screen: both storm-track
        maxima interior local maxima on the 3-year mean and in ≥ 2 of 3 single years;
        criteria 1, 2, cover and closure hold.
      - Step E: if a variant passes, the full gate once (T42 1 + 5; slab 400/800 ppm from a
        common restart to the equilibrium test, ≥ 30 years; feedbacks; `scripts/regress`),
        then APE Control T42, bistability pair, instellation sweep (record), closing
        evaluation and entry as specified on 2026-11-25; row 4 re-measured once.
      - Stop and report if: step A fails as described; no T21 variant meets criteria 1–2 with
        cover ≥ 0.51; no T42 variant passes the screen, or the 5-year gate fails 1, 2, 3 or 5
        (then zonal profile, margin and a read polar-air source as the next option; no further
        variants); a slab fails or does not equilibrate in 45 years; regress regresses.
      - Record for P5 (no action): tropical net CRE −41.4 against −14.4 with r3; tropical cloud
        phase liquid 4.1× and ice 0.17× Cloud_cci with the right total (goes with the
        convection item, phase of convective condensate).
      **Result (2026-11-26): stopped at step A, (b) fails.** W&B (2006) read in full (author's
      copy); CF = 0.14 + 0.060 EIS verified; RH0 0.8 is the paper's assumption. (a): r3 short
      against CERES EBAF Ed4.1 from 30° to 77° in both hemispheres (−0.14 to −0.17 at 32–57°N,
      −0.24 to −0.30 over the Southern Ocean); northern cap also below CERES (−0.03 to −0.09);
      (a) passes. (b): EIS raises the 68–80°N cover by +0.013 and 45–65°N by +0.006 (law
      +0.053 / +0.039), robust to the estimate and to +1.2 K in EIS; cause: small Γ_m over the
      cold summer Arctic Ocean (EIS 15.6 K). Southern Ocean law 0.18 → 0.35 but cover +0.02
      against a 0.25 deficit. Options in docs/progress.md (polar air; EIS with a waiver;
      diagnose the mid-latitude deficit, incl. freeze-dry; ruling).
      **Owner decision after step A (2026-11-26, follows ee43b8e), recorded before starting:**
      - Interpretation: the stop was correct. EIS is not adopted in P4 (option 2 refused): it
        does not fix the shape row and moves zonal cover by ≤ 0.02; it stays a P5 candidate
        for the Southern Ocean and the trades; `tools/eis_check.py` is kept. The polar-air
        fallback (option 1) is refused for now (the cap is below CERES; lowering it would pass
        the row by adding an error). What matters: cover is short by 0.14–0.30 over 30–77° in
        both hemispheres, a deficit of the humidity-based cloud, not of the stratus law.
        Lowering `rh_crit_surface` 0.95 → 0.80 changed nothing (step 4, variants d), so
        something else removes the low cloud. Suspects, both untested: freeze-dry
        (mid-latitude winter air near 3 g/kg; CAM's level never verified) and the subsidence
        gate (removes RH low cloud wherever air sinks, including cold air behind fronts).
      - **Decision: option 3, diagnosis, with limited authority to act.**
        Step 1: fetch a low/middle/high cover climatology (CALIPSO-GOCCP or ISCCP; name exactly
        what was fetched or say nothing could be); compare with r3 by band over 30–77° using
        `cl_split`; if nothing can be fetched, continue. Step 2: verify freeze-dry against
        CAM's `cloud_fraction.F90` (threshold, floor, levels, surface types) next to our code;
        compare only. Step 3: toggle runs, T21 Earth AMIP 1 + 1 years from r3, saving humidity
        and cloud by level, ≤ 6 runs, diagnostics only: a r3; b freeze-dry off; c subsidence
        gate off (stability law kept); d both off; e as (c) or (d), whichever restores more,
        with `rh_crit_surface` 0.85; f free if steps 1–2 point elsewhere (reason stated first).
        Report per run: zonal cover vs CERES by band; cloud fraction by level at 40–60°N,
        40–60°S, 68–80°N, winter and summer; criteria 1–2; tropical net CRE. Step 4: act only
        within limits — correct freeze-dry to its source if it departs (a correction, SCM test,
        tuning-log entry); knob values inside logged ranges for the CERES cover and band
        targets. Anything else (switching freeze-dry or the gate off, restricting either by
        latitude or surface, new ranges, new schemes) needs the owner. With a variant inside
        the limits that brings 32–57°N cover within 0.08 of CERES at T21 and meets criteria
        1–2: T42 1 + 3 year screen (step D, same rule), then if it passes the full gate and
        closing (step E).
      - Stop and report if: no toggle restores mid-latitude cover (report level profiles and
        the RH distribution at 40–60° against RH_c); the lever that works is outside the step 4
        limits; the T42 screen or gate fails as before; a slab run fails; regress fails.
      - ≤ 16 threads. Nothing chosen by the 2×CO₂ warming or the aquaplanet temperature.
      **Result (2026-11-26 day): stopped at step 3, no toggle restores mid-latitude cover.**
      Step 1: CALIPSO-GOCCP v3.1.2 (`MapLowMidHigh330m_200606-201803_avg`, fetched): the
      deficit is low cloud (raw model low below GOCCP in every band, 32–57°N 0.25 vs 0.34,
      45–60°S 0.43 vs 0.60); middle and high cloud at or above GOCCP poleward of 45°. Step 2:
      freeze-dry departed from CAM `cldfrc` (also reduced the stratus; σ 0.75 instead of
      p ≥ `premib` 750 hPa); corrected, SCM tests, tuning log. Step 3 (T21, 32–57°N cover −
      CERES): a −0.170, b −0.166, c −0.147, d −0.136, e −0.123 (fails criterion 1), f
      (corrected freeze-dry + `rh_crit_surface` 0.75) −0.152. Winter boundary layer at 40–60°
      sub-saturated (median RH 0.67–0.73 at σ ≥ 0.93, 1–11 % above RH_c); freeze-dry acts
      mostly poleward of 68°; the gate costs 16 W m⁻² of tropical net CRE. Options in
      docs/progress.md (BL humidity diagnosis; subgrid-variability low-cloud scheme; gate
      restriction; ruling).
      **Owner decision after the diagnosis (2026-11-26, follows 71aa0a8), recorded before
      starting:**
      - Push the branch first (done).
      - Interpretation: the stop was correct; the freeze-dry correction to the source is
        accepted; the frozen-knob gate numbers of 2026-11-21 predate it, are marked so and not
        rerun. The mid-latitude deficit is low cloud (GOCCP) and its cause is not in the cloud
        scheme: the boundary layer is too dry (winter median RH 0.67–0.73 at σ 0.93–0.99;
        lowest-level ocean RH 0.70 ± 0.08 against 0.80 ± 0.07); no cloud knob or switch
        reaches it. Two facts point below the clouds: global precipitation 3.2 mm/day against
        about 2.7 (verify against GPCP), and the lowest-level RH equals `rh_ref` = 0.7.
        Hypothesis (untested): convection fires often in mid-latitude winter air over the
        ocean and relaxes the boundary layer toward 0.7, drying it and driving extra
        evaporation. Alternative: boundary-layer mixing too deep or strong.
      - Decisions: options 2 and 3 refused (a scheme making cloud at lower RH would cover a
        humidity error; the gate restriction gains ≤ 0.034). Option 4: the shape row is not
        ruled met and P4 does not close with it open. Option 1 taken and widened. **Plan
        change:** the convection work item that was to be the first item of P5 is done now,
        inside P4; P5 starts only after P4 closes with the final convection and clouds;
        recorded in the P4 state section and the status table; SPEC.md not edited, criteria
        unchanged. r3 with the corrected freeze-dry is the working cloud configuration; no more
        cloud tuning until the humidity question is answered.
      - Next step, diagnosis only, then report. Step 1: fetch and name a near-surface ocean RH
        climatology, GPCP and an ocean evaporation / latent-heat climatology; compare with r3
        by band (tropics, subtropics, 40–60° both hemispheres, by season): near-surface RH,
        evaporation, precipitation, convective share; say what was not fetched. Step 2:
        moisture budget of σ ≥ 0.87 at 40–60°N and 40–60°S in winter and in the trade belts
        from one T21 Earth AMIP year with the tendencies saved (evaporation, mixing,
        convection, large-scale condensation, advection), plus convection frequency,
        skipping, parcel start level and mean boundary-layer depth. Step 3: T21 Earth AMIP
        1 + 1 year toggles, ≤ 5, logged: a reference; b `rh_ref` 0.8; c convection not
        allowed to modify the lowest one or two levels (or the nearest switch; say which);
        d mixing weakened or depth capped (one change, stated); e free with the reason stated
        first. Report per run: near-surface RH by band against step 1, E and P, low cover
        against GOCCP and CERES, criteria 1–2, tropical net CRE, Hadley strength, P5
        precipitation pattern correlation. Step 4: reread Frierson (2007) on the reference
        humidity and the boundary layer; find how at least one other published simplified-
        physics model treats this (compare only); list candidate fixes with a read source and
        their rerun cost for accepted milestones (P1–P3). Then stop: implement no fix, change
        no default.
      - ≤ 16 threads; rules unchanged; nothing chosen by the 2×CO₂ warming or the aquaplanet
        temperature.
      **Result (2026-11-26 afternoon): diagnosis done, stopped as instructed.** Observations:
      ICOADS 2° `rhum` (1991–2020), NCEP σ 0.995 RH, NCEP LHF, `prate`, `cprat`, GPCP v2.3;
      OAFlux not reachable; no observed convective share. Global P: GPCP 2.69, model 2.92 (T21)
      / 3.20 (T42). Near-surface ocean RH 0.05–0.15 below ICOADS, worst in winter at 40–60°
      (0.700 / 0.673 vs 0.802 / 0.819); winter E +33 % / +62 % vs NCEP; convective share at
      40–60° 0.57–0.68 vs NCEP 0.16–0.29. Budget σ ≥ 0.87, 40–60° winter: E +3.6 / +3.0,
      mixing −3.7 / −3.9, convection +0.3 / +0.9 (moistens), active 53–61 %, tops 650–680 hPa,
      PBL 1.9–2.1 km. Toggles (32–57°N cover − CERES): a −0.159; b `rh_ref` 0.8 −0.091 (fails
      criterion 2, SW −60.9; Hadley S −33 %); c `keep_lowest` 2 −0.156; d `ri_crit` 0.25
      −0.191; e `deep_requires_pt` −0.149. Hypothesis half right: RH held at `rh_ref` through
      frequent convection above the boundary layer and the mixing, not by direct drying.
      Options in docs/progress.md (raise `rh_ref`; SPEEDY-type boundary-layer trigger and
      closure; P_T criterion; other).
      **Owner decision after the humidity diagnosis (2026-11-26, follows 819e9de), recorded
      before starting:**
      - Interpretation: accepted as stated; the hypothesis was half right. The defect: "deep"
        convection active in 53–61 % of mid-latitude winter samples with tops near 650–680 hPa,
        making 57–68 % of the precipitation there (NCEP 16–29 %, itself a model product),
        holding the air above σ 0.87 at `rh_ref` and raining out what the mixing brings up;
        evaporation 33–62 % too high, near-surface RH 0.10–0.15 too low. Run b shows the
        direction, not the fix (a third of the southern Hadley cell, lower precipitation
        correlation): convection fires too often, more than what it relaxes to. Choice between
        changing what convection relaxes to (option 1) and when it fires (option 2), decided
        by a screen on Earth targets.
      - Step 1: build, off by default, byte-identical when off, SCM tests first (energy and
        water conservation, no adjustment when the trigger is not met): T, a boundary-layer
        humidity trigger after SPEEDY's description (deep convection only where boundary-layer
        RH exceeds a threshold; transcribe the rule and value 0.9 with the citation; state the
        mapping onto our levels; knob range 0.80–0.90, sources SPEEDY and ICOADS; relaxation
        profile otherwise unchanged; compare only, copy no source). If a primary source for the
        Betts–Miller shallow/deep distinction by cloud depth or top can be read (Betts 1986,
        Betts & Miller 1986, Janjić 1994), report it; do not build it.
      - Step 2: Hadley strength reference (both cells, annual and by season) from NCEP v.
      - Step 3: screen, T21 Earth AMIP 1 + 1 years, cloud knobs at r3, ≤ 6 runs, logged as
        diagnostics: s1 reference; s2 trigger 0.90; s3 trigger 0.80; s4 `rh_ref` 0.8 with
        `deep_requires_pt`; s5 the better trigger with `deep_requires_pt`; s6 free, reason
        stated first. Report: near-surface ocean RH by band and season vs ICOADS; E vs NCEP;
        global P vs GPCP; convective share and convection frequency at 40–60° winter and in the
        tropics; Hadley vs step 2; P pattern correlation; low cover vs GOCCP, total vs CERES;
        tropical net CRE; criteria 1–2 reported, not required.
      - Step 4, the bar (not to be adjusted), at T21: winter ocean RH at 40–60° within 0.05 of
        ICOADS in both hemispheres; global P within 10 % of GPCP; P pattern correlation not
        more than 0.01 below s1; each Hadley cell closer to the reference than s1, or within
        10 % of its s1 value; stable through both years. One passer: continue with it; several:
        highest P correlation; none: stop and report.
      - Step 5, only with an adopted variant, in order: a rerun P1 criterion 5, P2 criteria 2,
        3, 5, 6, P3 criterion 4 (any failure: stop, no cloud work); b cloud rebalance at T21,
        ≤ 6 variants, knobs in logged ranges, targets criteria 1–2, CERES bands and cover,
        GOCCP low cover, Cloud_cci water path; c T42 1 + 3 for the best one or two with the
        2026-11-25 screen rule for the shape row; d if it passes: defaults, new regress
        references, full P4 gate and closing as specified on 2026-11-25.
      - Stop and report if: no variant meets the bar; an accepted P1–P3 criterion fails; no
        cloud rebalance meets criteria 1–2, or the T42 screen or gate fails; a slab run fails or
        does not equilibrate in 45 years. ≤ 16 threads; Earth targets only, every change
        logged; nothing chosen by the 2×CO₂ warming or the aquaplanet temperature; say what
        was not fetched or read.
      - Result (2026-11-26 evening): no variant meets the bar; stopped at step 4.
        - Winter RH 40–60°N DJF / 40–60°S JJA and P correlation:
          s1 0.700 / 0.673, 0.704; s2 0.709 / 0.662, 0.664; s3 0.683 / 0.671, 0.644;
          s4 0.725 / 0.724, 0.630; s5 0.686 / 0.659, 0.622; s6 (trigger 0.90 + `ri_crit`
          0.25) 0.782 / 0.797, 0.627.
        - The trigger cuts winter convection to ≤ 1.3 % with no RH gain, and switches off
          tropical convection (share 0.12).
        - Options in docs/progress.md question 13.
