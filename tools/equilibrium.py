#!/usr/bin/env python3
"""Equilibrium test of a run (SPEC.md §9.5.2).

Usage:
    python tools/equilibrium.py out/<run> [--years 5]

A run is equilibrated when, over the last `years` years of `diagnostics.csv`,
- the linear trend of the annual means of global-mean T_s is below 0.02 K/yr (the annual
  means are averages of the `diagnostics.csv` rows in each model year; the run needs at
  least 12 rows per year, set `diagnostics_every`, e.g. "1day"), and
- the mean net top-of-atmosphere flux is within ±1 W m⁻² of the energy-closure residual
  (whole system: net TOA + q-flux + ice-cap flux − d(atmosphere + surface energy)/dt, the
  energy the model loses without accounting for it).

Prints both numbers and EQUILIBRATED / NOT EQUILIBRATED; exit status 0 when equilibrated.
"""
import argparse
import json
import pathlib

import numpy as np


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--years", type=float, default=5.0)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    d = np.genfromtxt(run / "diagnostics.csv", delimiter=",", names=True)
    year = (meta.get("calendar") or {}).get("year") or 365.25 * 86400
    t = d["time_s"]
    w = a.years * year
    if t[-1] - t[0] < w * 0.999:
        raise SystemExit(f"run covers {(t[-1] - t[0]) / year:.2f} years, less than {a.years}")
    i0 = int(np.searchsorted(t, t[-1] - w))
    sel = slice(i0, None)
    ty = t[sel] / year
    # annual means: bin the rows after t[i0] into whole years counted back from the end
    tt, ts = t[i0 + 1:], d["ts"][i0 + 1:]
    nyr = int(round(a.years))
    k = np.clip(np.floor((tt - t[i0]) / year).astype(int), 0, nyr - 1)
    counts = np.bincount(k, minlength=nyr)
    if counts.min() < 12:
        raise SystemExit(
            f"only {counts.min()} diagnostics rows in some year: annual means need at least 12 "
            "(set diagnostics_every, e.g. \"1day\")"
        )
    annual = np.bincount(k, weights=ts, minlength=nyr) / counts
    trend = np.polyfit(np.arange(nyr) + 0.5, annual, 1)[0]
    radius = meta["planet"]["radius"]
    area = 4 * np.pi * radius * radius
    T = t[-1] - t[i0]

    def rate(k):
        return (d[k][-1] - d[k][i0]) / T / area

    toa = rate("cum_toa_net")
    e_atm = d["energy"] + (d["latent_energy"] if "latent_energy" in d.dtype.names else 0)
    e_sfc = d["surface_energy"] if "surface_energy" in d.dtype.names else 0 * t
    de = ((e_atm[-1] + e_sfc[-1]) - (e_atm[i0] + e_sfc[i0])) / T / area
    src = toa
    for k in ("cum_qflux", "cum_ice_cap"):
        if k in d.dtype.names:
            src += rate(k)
    # closure residual: energy entering through the boundaries that the system does not
    # store (positive = the model loses energy unaccounted)
    leak = src - de
    ok_trend = abs(trend) < 0.02
    ok_toa = abs(toa - leak) <= 1.0
    print(f"{run.name}: last {a.years:g} years ({ty[0]:.2f}–{ty[-1]:.2f})")
    print(f"  annual-mean T_s    {' '.join(f'{x:.3f}' for x in annual)} K ({counts.min()}+ rows/yr)")
    print(f"  T_s trend          {trend:+.4f} K/yr  (|·| < 0.02: {'yes' if ok_trend else 'no'})")
    print(f"  net TOA            {toa:+.3f} W/m²")
    print(f"  closure residual   {leak:+.3f} W/m²")
    print(f"  TOA − residual     {toa - leak:+.3f} W/m²  (|·| ≤ 1: {'yes' if ok_toa else 'no'})")
    eq = ok_trend and ok_toa
    print("EQUILIBRATED" if eq else "NOT EQUILIBRATED")
    raise SystemExit(0 if eq else 1)


if __name__ == "__main__":
    main()
