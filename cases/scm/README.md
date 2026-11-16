# Single-column cases (SPEC.md §9.5.1)

Run with `toyclim scm --config cases/scm/<case>.toml --out out/scm/<case>`. They are also the
inputs of the SCM unit tests (`crates/toyclim-core/src/scm/tests.rs`), which check the P1
acceptance criteria 1–3:

| Case | Purpose |
|---|---|
| `dry_land_diurnal` | dry land at 20°, equinox, clear sky: 2 m diurnal range, stable nocturnal boundary layer |
| `ocean_diurnal` | same over a 50 m mixed layer, moist with SBM convection: diurnal range < 1 K; water budget |
| `stiff_land` | land with heat capacity 1e3 J m⁻² K⁻¹, 30 m/s wind, Δt_p = 1 h: no 2Δt oscillation |
