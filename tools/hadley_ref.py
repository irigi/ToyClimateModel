"""Hadley-cell reference from NCEP/NCAR reanalysis meridional wind (P4 question 13, owner
decision after the humidity diagnosis, 2026-11-26, step 2).

    python tools/hadley_ref.py [--json out.json]

Data: `reference/raw/ncep_vwnd/vwnd.mon.ltm.1991-2020.nc` (Kalnay et al. 1996; long-term
monthly means 1991–2020 on 17 pressure levels, 2.5°; fetched 2026-11-26 from NOAA PSL).
Mass streamfunction ψ(p, φ) = 2π a cos φ / g ∫_0^p [v] dp (zonal mean, integrated from the
top level, 10 hPa, downward; below-ground values as in the reanalysis). Strength as the
model's metric (tools/scorecard.py `hadley`): max |ψ| within 35° of the equator in each
hemisphere, at 700 hPa (the model uses σ = 0.7); also the maximum over 300–850 hPa. Annual
(mean of the 12 months), DJF and JJA. In the solstice seasons the per-hemisphere maximum can
pick the cross-equatorial winter cell twice, so the cells are also given by sign (`cells`):
the northern cell = largest positive ψ at 700 hPa between −20° and 35°, the southern cell =
largest negative ψ between −35° and 20° (the signs of the annual cells), with their latitude.
"""

import argparse
import json
import pathlib

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
F = ROOT / "reference/raw/ncep_vwnd/vwnd.mon.ltm.1991-2020.nc"
A, G = 6.371e6, 9.80616
SEASONS = {"ann": list(range(1, 13)), "DJF": [12, 1, 2], "JJA": [6, 7, 8]}


def psi_of(vz, p, lat):
    """vz[lev, lat] on pressure levels p [Pa] (any order) → ψ at the levels [kg/s]."""
    o = np.argsort(p)
    p, vz = p[o], vz[o]
    edges = np.concatenate([[0.0], 0.5 * (p[1:] + p[:-1])])
    dp = np.diff(np.concatenate([edges, [p[-1] + 0.5 * (p[-1] - p[-2])]]))
    cum = np.cumsum(vz * dp[:, None], axis=0) - 0.5 * vz * dp[:, None]  # ∫ to the level
    return p, 2 * np.pi * A * np.cos(np.radians(lat))[None, :] / G * cum


def strength(psi, p, lat):
    out = {}
    k7 = int(np.argmin(np.abs(p - 70000.0)))
    band = (p >= 30000) & (p <= 85000)
    for h, sel in (("NH", (lat > 0) & (lat < 35)), ("SH", (lat < 0) & (lat > -35))):
        j = np.where(sel)[0]
        a7 = np.abs(psi[k7, j])
        ab = np.abs(psi[band][:, j])
        out[h] = {"psi700_1e10_kg_s": float(a7.max() / 1e10),
                  "lat_700": float(lat[j[np.argmax(a7)]]),
                  "psi_max_300_850_1e10_kg_s": float(ab.max() / 1e10)}
    out["cells"] = cells(psi[k7], lat)
    return out


def cells(row, lat):
    """Cells by sign at one level: the northern cell is the largest positive ψ at −20° to 35°,
    the southern cell the largest negative ψ at −35° to 20° (sign of the annual cells)."""
    n = np.where((lat > -20) & (lat < 35))[0]
    s = np.where((lat > -35) & (lat < 20))[0]
    i, j = n[np.argmax(row[n])], s[np.argmin(row[s])]
    return {"pos_1e10_kg_s": float(row[i] / 1e10), "pos_lat": float(lat[i]),
            "neg_1e10_kg_s": float(-row[j] / 1e10), "neg_lat": float(lat[j])}


def reference():
    import netCDF4

    d = netCDF4.Dataset(F)
    v = np.ma.filled(d["vwnd"][:].astype(float), np.nan)  # [12, lev, lat, lon]
    p = np.asarray(d["level"][:], float) * 100.0
    lat = np.asarray(d["lat"][:], float)
    res = {}
    for s, ms in SEASONS.items():
        vz = np.nanmean(np.nanmean(v[[m - 1 for m in ms]], axis=0), axis=-1)
        pp, psi = psi_of(vz, p, lat)
        res[s] = strength(psi, pp, lat)
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json")
    a = ap.parse_args()
    r = reference()
    for s, v in r.items():
        c = v["cells"]
        print(s, " ".join(f"{h}: {v[h]['psi700_1e10_kg_s']:.2f} at {v[h]['lat_700']:.1f} "
                          f"(max 300–850 {v[h]['psi_max_300_850_1e10_kg_s']:.2f})"
                          for h in ("NH", "SH")),
              f"| by sign: + {c['pos_1e10_kg_s']:.2f} at {c['pos_lat']:.1f}, "
              f"− {c['neg_1e10_kg_s']:.2f} at {c['neg_lat']:.1f}")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(r, indent=1))


if __name__ == "__main__":
    main()
