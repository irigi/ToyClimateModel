# Published comparators

One folder per paper, `cases/bench/<paper>/`, with the configuration transcribed from the
paper (constants and citation in the case file) and the numbers it is compared against
(SPEC.md §9.6).

| Folder | Paper | Gate |
|---|---|---|
| `neale_hoskins2000/` | Neale & Hoskins (2000), Aqua-Planet Experiment | P1 aquaplanet (`ape_control`, T42; `_t21` for development and `scripts/regress`) |
| `earth_amip/` | AMIP configuration with observed data (GPCP, NCEP/NCAR, OISST, Rutgers snow, ETOPO5, Beck et al. 2018; `tools/fetch_reference.py`) | P2 item 7 record, P5 Earth gate (`_t42`; `_t21` smoke) |
