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
    ap.add_argument("--by-month", action="store_true", help="statistics per calendar month (needs an [orbit])")
    ap.add_argument("-o", "--output", default=None)
    a = ap.parse_args()
    if a.by_month:
        return by_month(a)

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


def by_month(a):
    """Per-month distributions using the calendar in meta.json."""
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    cal = meta.get("calendar")
    if not cal:
        raise SystemExit("meta.json has no calendar (run without an [orbit] block)")
    d = run / "stations"
    smeta = json.loads((d / "stations.json").read_text())
    t = np.load(d / "time.npy")
    x = np.load(d / f"{a.field}.npy")
    if x.ndim == 3:
        x = x[:, :, -1 if a.level is None else a.level]
    x = x * a.scale
    months = cal["months"]
    mi = ((t / cal["year"]) % 1.0 * len(months)).astype(int).clip(0, len(months) - 1)
    names = [s_["name"] for s_ in smeta["stations"]]
    idx = range(len(names)) if a.station is None else [names.index(a.station)]
    ths = [float(v) for v in a.thresholds.split(",")] if a.thresholds else []
    fig, axs = plt.subplots(len(idx), 1, figsize=(10, 2.6 * len(idx)), constrained_layout=True, squeeze=False)
    for row, i in enumerate(idx):
        print(f"== {names[i]} ({a.field} {a.unit})")
        header = f"{'month':<6}{'n':>6}{'mean':>10}{'p50':>10}{'p90':>10}{'p99':>10}" + "".join(f"{'P>=' + format(th, 'g'):>10}" for th in ths)
        print(header)
        data = []
        for m, name in enumerate(months):
            v = x[mi == m, i]
            if v.size == 0:
                data.append(np.array([np.nan]))
                continue
            data.append(v)
            line = f"{name:<6}{v.size:>6}{v.mean():>10.4g}{np.percentile(v, 50):>10.4g}{np.percentile(v, 90):>10.4g}{np.percentile(v, 99):>10.4g}"
            line += "".join(f"{np.mean(v >= th):>10.3f}" for th in ths)
            print(line)
        ax = axs[row, 0]
        ax.boxplot([dd for dd in data], tick_labels=months, whis=(5, 95), showfliers=False)
        ax.plot(range(1, len(months) + 1), [np.nanmean(dd) for dd in data], "o-", ms=3, label="mean")
        ax.set_ylabel(f"{a.field} {a.unit}")
        ax.set_title(names[i], fontsize=10)
        ax.legend(fontsize=8)
    out = a.output or str(run / f"stations_{a.field}_by_month.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
