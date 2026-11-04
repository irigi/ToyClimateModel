#!/usr/bin/env python3
"""Point-scale rain statistics from grid-cell station series (stochastic sub-grid
generator; design decision "stochastic downscaling", first, UNCALIBRATED version).

A model grid cell (hundreds of km) reports an area-mean rain rate R. At a point, rain is
intermittent and more intense. For each hourly sample:
  - the raining fraction of the cell is f = 1 − exp(−R / R_c);
  - a point is wet with probability f, and its intensity is Gamma-distributed with mean
    R / f and shape k (k < 1 → heavy tail).
Exceedance probabilities are estimated by Monte Carlo (N draws per sample) and printed next
to the raw grid-cell values. R_c and k are placeholders until calibrated against station
data (M11); treat the point numbers as illustrative.

Usage:
    python tools/point_rain.py out/<run> [--station NAME] [--thresholds 0.1,1,2,5,10,20]
        [--rc 1.0] [--shape 0.7] [--draws 200] [--by-month]
"""
import argparse
import json
import pathlib

import numpy as np


def point_samples(R, rc, shape, draws, rng):
    """R: grid-cell rates (mm/h), 1D. Returns point rates, shape (len(R), draws)."""
    R = np.maximum(R, 0.0)
    f = 1.0 - np.exp(-R / rc)
    wet = rng.random((R.size, draws)) < f[:, None]
    mean = np.where(f > 0, R / np.maximum(f, 1e-12), 0.0)
    inten = rng.gamma(shape, 1.0, (R.size, draws)) * (mean / shape)[:, None]
    return np.where(wet, inten, 0.0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--station", default=None)
    ap.add_argument("--field", default="precip")
    ap.add_argument("--thresholds", default="0.1,1,2,5,10,20")
    ap.add_argument("--rc", type=float, default=1.0, help="cell-mean rate [mm/h] at which 63%% of the cell rains")
    ap.add_argument("--shape", type=float, default=0.7, help="gamma shape of point intensities")
    ap.add_argument("--draws", type=int, default=200)
    ap.add_argument("--by-month", action="store_true")
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    d = run / "stations"
    smeta = json.loads((d / "stations.json").read_text())
    t = np.load(d / "time.npy")
    x = np.load(d / f"{a.field}.npy") * 3600.0  # kg m-2 s-1 -> mm/h
    names = [s["name"] for s in smeta["stations"]]
    idx = range(len(names)) if a.station is None else [names.index(a.station)]
    ths = [float(v) for v in a.thresholds.split(",")]
    rng = np.random.default_rng(a.seed)
    meta = json.loads((run / "meta.json").read_text())
    cal = meta.get("calendar")
    groups = [("all", np.ones(t.size, bool))]
    if a.by_month and cal:
        nm = len(cal["months"])
        mi = ((t / cal["year"]) % 1.0 * nm).astype(int).clip(0, nm - 1)
        groups = [(m, mi == k) for k, m in enumerate(cal["months"])]
    print(f"R_c = {a.rc} mm/h, gamma shape = {a.shape} (uncalibrated); hourly samples")
    for i in idx:
        print(f"== {names[i]}")
        print(f"{'period':<6}{'kind':<6}" + "".join(f"{'P>=' + format(th, 'g'):>10}" for th in ths) + f"{'max':>10}")
        for gname, sel in groups:
            R = x[sel, i]
            if R.size == 0:
                continue
            pts = point_samples(R, a.rc, a.shape, a.draws, rng)
            print(f"{gname:<6}{'cell':<6}" + "".join(f"{np.mean(R >= th):>10.4f}" for th in ths) + f"{R.max():>10.2f}")
            print(f"{'':<6}{'point':<6}" + "".join(f"{np.mean(pts >= th):>10.4f}" for th in ths) + f"{np.percentile(pts, 99.99):>10.2f}")


if __name__ == "__main__":
    main()
