# Instructions for Claude sessions

- Read `SPEC.md` in full, then `docs/progress.md`, `docs/design-decisions.md` and
  `docs/tuning.md` (once it exists), before changing anything. **SPEC.md is the plan.**
  Part I (§1–§8, milestones M0–M6) is accepted; Part II (§9, milestones P0–P11) is the
  current work.
- Continue at the lowest milestone whose acceptance criteria are not all met (SPEC.md
  §9.6). P0–P9 are authorised, in order. Do not start P10 or P11 without the owner's
  go-ahead.
- The physics, surface and tools written under the old labels "M7–M11" are a prototype,
  not accepted work (SPEC.md §9.1). Do not build on them beyond what §9 says.
- Follow the session protocol in SPEC.md §10:
  - never weaken an acceptance criterion; record blockers under "Open questions for the
    owner" and move on to the next independent item;
  - tune only on Earth targets and log every change in `docs/tuning.md`; never tune on an
    exoplanet benchmark;
  - new physics schemes get single-column conservation tests before they run in 3-D;
  - long runs go in the background in `--release`;
  - open and describe every figure you produce;
  - every session ends with a `docs/progress.md` entry containing the measured numbers.
- Build: `cargo build --release`. Tests: `cargo test` and, for long ones,
  `cargo test --release -- --ignored`. Lint: `cargo fmt --check`,
  `cargo clippy -- -D warnings`. `scripts/check` runs all of these.
- `toyclim-core` must not depend on Godot. No `unsafe` outside `toyclim-godot`.
- Before implementing a published scheme or test case, fetch the paper and transcribe its
  constants into the case file (or the scheme's doc comment) with the citation; the paper
  wins over SPEC.md if they disagree. Formulas in SPEC.md §9 were written from memory.
- PlaSim, ExoPlaSim and Isca are GPL: compare against them, never copy their source.
