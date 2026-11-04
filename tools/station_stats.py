#!/usr/bin/env python3
"""Station time series: plot series and distributions, print summary statistics.

Usage:
    python tools/station_stats.py out/<run> <field> [--station NAME] [--bins 30] [-o out.png]
        [--thresholds 2,20]   (print how often field >= each threshold)

Reads out/<run>/stations/{time,<field>}.npy and stations.json. For 3D fields the lowest
level (last index) is used unless --level is given.
"""
import argparse
import json
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("field")
    ap.add_argument("--station", default=None)
    ap.add_argument("--level", type=int, default=None)
    ap.add_argument("--bins", type=int, default=30)
    ap.add_argument("--thresholds", default=None, help="comma-separated thresholds")
    ap.add_argument("--scale", type=float, default=1.0, help="multiply data, e.g. 3600 for precip kg/m2/s -> mm/h")
    ap.add_argument("--unit", default="", help="unit label after scaling")
    ap.add_argument("-o", "--output", default=None)
    a = ap.parse_args()

    d = pathlib.Path(a.run) / "stations"
    meta = json.loads((d / "stations.json").read_text())
    t = np.load(d / "time.npy") / 86400.0
    x = np.load(d / f"{a.field}.npy")
    if x.ndim == 3:
        x = x[:, :, -1 if a.level is None else a.level]
    x = x * a.scale
    names = [s["name"] for s in meta["stations"]]
    idx = range(len(names)) if a.station is None else [names.index(a.station)]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11, 4), constrained_layout=True)
    for i in idx:
        ax1.plot(t, x[:, i], label=names[i], lw=0.9)
        ax2.hist(x[:, i], bins=a.bins, histtype="step", density=True, label=names[i])
        s = x[:, i]
        line = f"{names[i]:<16} n={s.size} mean={s.mean():.4g} sd={s.std():.4g} min={s.min():.4g} max={s.max():.4g}"
        if a.thresholds:
            for th in (float(v) for v in a.thresholds.split(",")):
                line += f"  P(>={th:g})={np.mean(s >= th):.3f}"
        print(line)
    ax1.set_xlabel("time [days]")
    ax1.set_ylabel(f"{a.field} {a.unit}")
    ax1.legend(fontsize=8)
    ax2.set_xlabel(f"{a.field} {a.unit}")
    ax2.set_ylabel("probability density")
    out = a.output or str(pathlib.Path(a.run) / f"stations_{a.field}.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
