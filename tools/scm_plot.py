#!/usr/bin/env python3
"""Plot single-column runs (`toyclim scm`): T_s, 2 m temperature, sensible and latent heat
flux and boundary-layer height against time, one column of panels per run.

    python tools/scm_plot.py out/scm/dry_land_diurnal out/scm/ocean_diurnal -o fig.png
    [--days 3]   (last N days)
"""
import argparse
import csv
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def load(run):
    with open(os.path.join(run, "scm.csv")) as f:
        rows = list(csv.DictReader(f))
    return {k: [float(r[k]) for r in rows] for k in rows[0]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("-o", "--output", required=True)
    ap.add_argument("--days", type=float, default=3.0)
    a = ap.parse_args()
    panels = [("ts", "tas", "K"), ("shf", "lhf", "W m$^{-2}$"), ("pblh", None, "m")]
    fig, axes = plt.subplots(len(panels), len(a.runs), figsize=(4.2 * len(a.runs), 7), squeeze=False, sharex="col")
    for j, run in enumerate(a.runs):
        d = load(run)
        t = [x / 86400 for x in d["time_s"]]
        t0 = t[-1] - a.days
        idx = [i for i, x in enumerate(t) if x >= t0]
        tt = [t[i] for i in idx]
        for i, (k1, k2, unit) in enumerate(panels):
            ax = axes[i][j]
            for k in (k1, k2):
                if k and k in d:
                    ax.plot(tt, [d[k][n] for n in idx], label=k)
            ax.set_ylabel(unit)
            ax.legend(fontsize=8)
            ax.grid(alpha=0.3)
        axes[0][j].set_title(os.path.basename(run.rstrip("/")))
        axes[-1][j].set_xlabel("day")
    fig.tight_layout()
    fig.savefig(a.output, dpi=110)
    print(a.output)


if __name__ == "__main__":
    main()
