# Instructions for Claude sessions

- Read `SPEC.md` in full, then `docs/progress.md` and `docs/design-decisions.md`, before
  changing anything.
- Continue at the lowest milestone whose acceptance criteria are not all met. Do not start
  M7 or later without the owner's go-ahead.
- Follow the session protocol in SPEC.md §10:
  - never weaken an acceptance criterion; record blockers under "Open questions for the
    owner" and move on to the next independent item;
  - long runs go in the background in `--release`;
  - every session ends with a `docs/progress.md` entry containing the measured numbers.
- Build: `cargo build --release`. Tests: `cargo test` and, for long ones,
  `cargo test --release -- --ignored`. Lint: `cargo fmt --check`,
  `cargo clippy -- -D warnings`.
- `toyclim-core` must not depend on Godot. No `unsafe` outside `toyclim-godot`.
- Before implementing a published test case, fetch the paper and transcribe its constants
  into `cases/<name>.toml` with the citation; the paper wins over SPEC.md if they disagree.
