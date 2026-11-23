"""Prescribed-ocean boundary files from a slab run's monthly climatology (P4 question 13, review
round of 2026-11-23, item 2: fixed-SST factorial on the slab pair's own SST and sea ice).

    python tools/slab_bc.py <slab run> <out prefix>

Needs `[climatology] monthly = true` with maps `tos`, `sic` and `sit`. Writes
`<prefix>_tos.npy` and `<prefix>_sic.npy`, shape (12, nlat, nlon), the layout of
`[surface.ocean] model = "prescribed"` (calendar months of the run, equal length, t = 0 at the
start of the year). Prints the global and 20°S–20°N means and, per hemisphere, the mean sea-ice
thickness where ice exists (sic-weighted mean of `sit` over points and months with sic > 0.01),
the value to give `ice_thickness` (the slab ice has a prognostic thickness; prescribed ice has
a fixed floe thickness per hemisphere).
"""

import json
import pathlib
import sys

import numpy as np


def main():
    run, prefix = pathlib.Path(sys.argv[1]), sys.argv[2]
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlat, nlon = len(lat), len(meta["grid"]["lon"])
    m = {n: np.load(run / "climatology" / f"monthly_{n}.npy").astype(float).reshape(12, nlat, nlon)
         for n in ("tos", "sic", "sit")}
    counts = json.loads((run / "climatology" / "sums.json").read_text()).get("monthly_count")
    assert m["tos"].min() > 200.0, "tos has empty months"
    np.save(f"{prefix}_tos.npy", m["tos"])
    np.save(f"{prefix}_sic.npy", np.clip(m["sic"], 0.0, 1.0))
    w = np.cos(np.radians(lat))[None, :, None] * np.ones((12, nlat, nlon))
    trop = (np.abs(lat) <= 20)[None, :, None] & np.ones((12, nlat, nlon), bool)
    print(f"monthly samples: {counts}")
    print(f"tos global {np.average(m['tos'], weights=w):.2f} K, 20S-20N "
          f"{np.average(m['tos'][trop], weights=w[trop]):.2f} K; sic global {np.average(m['sic'], weights=w):.4f}")
    for name, hemi in (("north", lat > 0), ("south", lat < 0)):
        s = hemi[None, :, None] & (m["sic"] > 0.01)
        if s.any():
            h = np.average(m["sit"][s], weights=(w * m["sic"])[s])
            print(f"{name}: mean ice thickness {h:.2f} m (sic-weighted), max sic {m['sic'][s].max():.2f}")
        else:
            print(f"{name}: no sea ice")


if __name__ == "__main__":
    main()
