# Design decisions (owner answers, 2026-11-04)

Answers to the scoping questions asked in the M0 session. They do not change the
M0–M6 acceptance criteria in SPEC.md; they say what those milestones must leave room for
and how M7+ should be shaped once the owner gives the go-ahead.

## Purpose

**Worldbuilding sandbox.** Design a planet, run it, get believable climate. No goals,
scoring or progression layer. Science fidelity first.

## Deliverables (all wanted)

1. **Local weather statistics** — the headline output. For a place on the planet: how
   often it does not rain, rains 2 mm/h, 20 mm/h; distributions of wind speed and
   temperature; how these distributions change through the year.
2. Climate/biome maps (Köppen, Holdridge; annual and seasonal means).
3. Live weather view in Godot.
4. Exported map layers for other tools.
5. Numbers and plots via the Python tools.

## Run time budget

**Overnight is fine.** T85 and long spin-ups (decades for a slab ocean) are acceptable for
final runs. Restarts and robust long runs (M6) matter more than raw speed; a fast
low-resolution preview is still useful but not required.

## Worlds to support (eventually all)

- Earth-like with any rotation (fast, slow, zero, retrograde), obliquity, eccentricity.
- Tidally locked.
- Thin (Mars-like) to thick (Venus-like) atmospheres.
- Exotic/fictional: non-water condensables (e.g. methane), moons, binary stars.

**Generality: generic interfaces now, water first.** Code from M7 on takes condensable
properties (latent heat, Clausius–Clapeyron constants, molar mass) and the gas mix as
data; only water on an N₂/O₂-like atmosphere is tested until the owner asks for more.
Consequence for M0–M6: `ColumnIn`/`ColumnOut` must allow a list of tracers rather than a
single hard-coded `q`; the planet's thermodynamic constants stay data, not code.

## Weather statistics: stations only

Statistics are collected from **stations**: user-placed points (name, lat, lon) at which the
model writes full time series at high frequency (every step or hourly) for the whole run.
Any distribution can be computed afterwards in Python. No per-cell online histograms.

- Proposed config shape (to add to SPEC §6 when implemented):
  `stations = [{ name = "Port Ash", lat = 41.2, lon = 12.0 }]`,
  `station_every = "1h"`, `station_fields = ["precip", "u10", "v10", "t2m", "ps"]`.
- Output: `out/<run>/stations/<name>.npy` (`[time][field]`) plus names in `meta.json`.
- Interpolation from the Gaussian grid: bilinear (cheap, deterministic).
- Can be implemented as soon as there is a model to sample (M2), with dynamical
  fields; precipitation arrives with M7.

## Point-scale extremes: stochastic downscaling

A T42 cell (~300 km) gives area-mean rain; point rain rates are far more intermittent.
Report both:

- grid-cell statistics (raw model output), and
- point-like statistics from a **stochastic sub-grid generator** (rain intermittency and
  intensity, gustiness), calibrated against Earth station data, clearly labelled as
  statistical. Belongs with M11 (downscaling), together with the lapse-rate and
  Smith–Barstad orographic corrections.

## Calendar

**Custom calendar.** Config defines year length, day length and named months/seasons;
statistics and climatologies are binned by them. Default: Earth months. Tidally locked
planets with no seasons get a trivial calendar.

## Terrain

**Full procedural generation** (plate-tectonics-like) inside toyclim. This is a large,
separate project; proposed as its own future milestone (after M9 orography support), not
before. Until then, orography comes from a heightmap file (M9).

## Interaction

**Configure, then watch.** Set up a planet (TOML or viewer), run, pause, change speed, save
restarts. Changing physics means a new run or a restart with new parameters. No live
parameter tweaking or terrain painting.

## Platform and licence

- Viewer mainly on **Linux**; no Windows build path needed for now.
- Licence: **MIT**.
