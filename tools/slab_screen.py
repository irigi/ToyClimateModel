#!/usr/bin/env python3
"""Slab-aquaplanet screen of P4 question 13 (owner decision after step 7 of 2026-11-24, step B):
T_s, sea ice and their trends, year by year, for runs continued from the same restart.

    .venv/bin/python -I tools/slab_screen.py <run> [...] --labels a b ... [--first 46 --last 60]
        [--fig out.png] [--json out.json]

Per model year y (time in (y − 1, y] years of 365.25 days):
- T_s: mean of the `ts` column of diagnostics.csv (global mean, daily rows);
- sea-ice fraction (area-weighted global mean of `sic`) and ice edge (per hemisphere the
  equatorward-most latitude row whose zonal-mean `sic` is ≥ 0.15, mean of both hemispheres;
  0 rows with ice = no edge) from the annual `sic` snapshot written at the end of year y
  (the same season every year, so a trend is meaningful; a single snapshot is not an annual
  mean).

Screen (owner decision, step B): the slab keeps its sea ice, T_s (mean of the last 5 years)
inside 283–293 K, and no trend toward melting in the last 5 years. "Trend toward melting" is
fixed here before the screen results: the linear T_s trend over the last 5 annual means
exceeds +0.02 K/yr (the equilibrium test's threshold), or the snapshot ice fraction falls by
more than 10 % of its value at the start of those 5 years (least-squares line over the 5
snapshots), or the year-end ice fraction is 0.
"""
import argparse
import csv
import glob
import json
import pathlib

import numpy as np

YEAR = 365.25 * 86400.0


def load(run, first, last):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    w = np.asarray(meta["grid"]["weights"], float)
    dt = float(meta["config"]["dt"])
    ts = {}
    with open(run / "diagnostics.csv") as f:
        for row in csv.DictReader(f):
            t = float(row["time_s"])
            if t <= 0:
                continue
            y = int(np.ceil(t / YEAR - 1e-9))
            if first <= y <= last:
                ts.setdefault(y, []).append(float(row["ts"]))
    years = sorted(y for y in ts if len(ts[y]) >= 300)
    out = {"years": years, "ts": [float(np.mean(ts[y])) for y in years], "ice": [], "edge": [],
           "ice_years": []}
    for fpath in sorted(glob.glob(str(run / "fields" / "sic_*.npy")),
                        key=lambda p: int(pathlib.Path(p).stem.split("_")[-1])):
        step = int(pathlib.Path(fpath).stem.split("_")[-1])
        y = int(round(step * dt / YEAR))
        if not first <= y <= last:
            continue
        s = np.load(fpath).astype(float)
        zm = s.mean(axis=-1)
        frac = float((zm * w).sum() / w.sum())
        edges = []
        for hem in (lat > 0, lat < 0):
            iced = np.abs(lat[hem])[zm[hem] >= 0.15]
            if iced.size:
                edges.append(float(iced.min()))
        out["ice_years"].append(y)
        out["ice"].append(frac)
        out["edge"].append(float(np.mean(edges)) if len(edges) == 2 else float("nan"))
    return out


def screen(d, n=5):
    ys, ts = np.array(d["years"][-n:], float), np.array(d["ts"][-n:])
    yi, ice = np.array(d["ice_years"][-n:], float), np.array(d["ice"][-n:])
    tr = float(np.polyfit(ys, ts, 1)[0])
    if len(ice) >= 2 and ice[0] > 0:
        line = np.polyval(np.polyfit(yi, ice, 1), [yi[0], yi[-1]])
        ice_drop = float((line[0] - line[1]) / line[0])
    else:
        ice_drop = float("nan")
    ts_mean = float(ts.mean())
    keeps_ice = bool(ice[-1] > 0)
    melting = tr > 0.02 or (np.isfinite(ice_drop) and ice_drop > 0.10) or not keeps_ice
    ok = keeps_ice and 283.0 <= ts_mean <= 293.0 and not melting
    return {"years": [int(ys[0]), int(ys[-1])], "ts_mean": ts_mean, "ts_trend": tr,
            "ice_last": float(ice[-1]), "ice_drop_frac": ice_drop,
            "edge_last": d["edge"][-1] if d["edge"] else float("nan"), "keeps_ice": keeps_ice,
            "melting_trend": bool(melting), "pass": bool(ok)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--first", type=int, default=46)
    ap.add_argument("--last", type=int, default=60)
    ap.add_argument("--fig")
    ap.add_argument("--json")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    res = {}
    for run, lab in zip(a.runs, labels):
        d = load(run, a.first, a.last)
        if len(d["years"]) < 2:
            print(f"{lab}: fewer than 2 complete years in {a.first}-{a.last}")
            continue
        s = screen(d)
        res[lab] = {"series": d, "screen": s}
        print(f"{lab}: years {s['years'][0]}-{s['years'][1]}: T_s {s['ts_mean']:.2f} K, trend "
              f"{s['ts_trend']:+.3f} K/yr; ice fraction (year-end snapshot) {s['ice_last']:.3f}, "
              f"fitted change over the 5 years {-s['ice_drop_frac']:+.0%} , edge {s['edge_last']:.1f} deg; "
              f"melting trend {s['melting_trend']}; screen {'PASS' if s['pass'] else 'FAIL'}")
        print("   T_s by year: " + " ".join(f"{y}:{t:.2f}" for y, t in zip(d["years"], d["ts"])))
        print("   ice by year: " + " ".join(f"{y}:{i:.3f}/{e:.0f}" for y, i, e in
                                           zip(d["ice_years"], d["ice"], d["edge"])))
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(res, indent=1))
    if a.fig:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        fig, ax = plt.subplots(1, 3, figsize=(15, 4))
        for lab in [x for x in labels if x in res]:
            d = res[lab]["series"]
            ax[0].plot(d["years"], d["ts"], marker=".", label=lab)
            ax[1].plot(d["ice_years"], d["ice"], marker=".", label=lab)
            ax[2].plot(d["ice_years"], d["edge"], marker=".", label=lab)
        ax[0].axhspan(283, 293, color="0.9", zorder=0, label="283-293 K")
        ax[0].set_title("global-mean T_s, annual mean")
        ax[0].set_ylabel("K")
        ax[1].set_title("sea-ice fraction (year-end snapshot)")
        ax[2].set_title("ice edge (sic ≥ 0.15, mean of hemispheres; year-end)")
        ax[2].set_ylabel("|latitude|")
        for x in ax:
            x.set_xlabel("model year")
            x.axvspan(a.last - 4.5, a.last + 0.5, color="tab:orange", alpha=0.08)
            x.grid(alpha=0.3)
        ax[0].legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(a.fig, dpi=100)


if __name__ == "__main__":
    main()
