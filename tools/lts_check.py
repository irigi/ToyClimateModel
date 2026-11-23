"""Offline check of the lower-tropospheric-stability stratus law on existing output (P4 question
13, owner decision (vi) of 2026-11-23: "before 3-D runs, evaluate the law offline on the existing
T42 Earth output"; stop if the decks come out under 0.3).

    python tools/lts_check.py <run> [--json out.json] [--fig out.png]

The law as transcribed from the CAM implementation (ESCOMP/CAM, tag cam_cesm2_2_rel,
src/physics/cam/cloud_fraction.F90, subroutine cldfrc, fetched 2026-11-23; it cites Klein &
Hartmann 1993, which was NOT read; the CAM3 technical note could not be fetched):

    strat = min(1, max(0, f_ocn · ((θ(k700) − θ_s) · 0.057 − 0.5573)))

- θ_s = SST (1000 hPa / p_s)^κ (ocean surface, lapse-rate correction for φ_s = 0 at sea), only
  where the ocean fraction (excluding sea ice) exceeds 0.01; here: non-land points with
  T_s ≥ 271.35 K (no sea-ice field in the snapshots), f_ocn = 1;
- k700 = the model level nearest 700 hPa (CAM: reference pressure; here σ nearest 0.7);
- trigger and placement: over the pairs of adjacent levels (k−1 above, k below) with p_k ≥
  `premib` (750 hPa for CAM4 physics), the most negative Δθ/Δp = 100 (θ_k − θ_{k−1}) /
  (p_k − p_{k−1}) [K/hPa]; it must be below −0.125 K/hPa ("nothing triggers unless a stability
  greater than this minimum threshold is found"); if no pair qualifies, the surface pair
  100 (θ_s − θ_L)/(p_s − p_L) is tried with the same threshold; the cloud goes into layer k
  (the lower level of that pair; the lowest layer for the surface pair) as
  min(strat, max(RH_k, RH_{k+1})) and combines with the RH cloud by maximum.

Printed per region (boxes of tools/inversion_check.py): mean strat from LTS alone; the share of
snapshots where the trigger fires; the cloud with the trigger and RH limit (as CAM); the cloud
without the trigger (law + placement in the most stable layer + RH limit); LTS.
"""

import argparse
import json
import pathlib

import numpy as np

from inversion_check import KAPPA, REGIONS, snapshots

SLOPE, OFFSET, TRIGGER, PREMIB = 0.057, 0.5573, -0.125, 75000.0


def analyse(run):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"])) % 360
    nlat, nlon = len(lat), len(lon)
    sig = np.asarray(meta["sigma"], float)
    l = len(sig)
    dt = float(cfg["dt"])
    year = 365.25 * 86400.0
    start = float(cfg.get("climatology", {}).get("start", 0.0))
    fs = {n: snapshots(run, n) for n in ("t", "rh", "ts", "ps")}
    land = np.load(next(iter(snapshots(run, "land").values()))).reshape(nlat, nlon) > 0.5
    steps = sorted(s for s in fs["t"] if all(s in fs[n] for n in fs) and s * dt >= start)
    k700 = int(np.argmin(np.abs(sig - 0.7)))
    out = {k: [] for k in ("strat", "fire", "cam", "notrig", "lts", "month", "dthdp")}
    for s in steps:
        t = np.load(fs["t"][s]).astype(float).reshape(l, nlat, nlon)
        rh = np.load(fs["rh"][s]).astype(float).reshape(l, nlat, nlon)
        ts = np.load(fs["ts"][s]).astype(float).reshape(nlat, nlon)
        ps = np.load(fs["ps"][s]).astype(float).reshape(nlat, nlon)
        ocean = ~land & (ts >= 271.35)
        p = sig[:, None, None] * ps[None]
        th = t * (1.0e5 / p) ** KAPPA
        ths = ts * (1.0e5 / ps) ** KAPPA
        lts = th[k700] - ths
        strat = np.where(ocean, np.clip(SLOPE * lts - OFFSET, 0.0, 1.0), 0.0)
        best = np.full((nlat, nlon), np.inf)
        kd = np.full((nlat, nlon), -1)
        for k in range(1, l):
            sl = 100.0 * (th[k] - th[k - 1]) / (p[k] - p[k - 1])
            better = (p[k] >= PREMIB) & (sl < best)
            best = np.where(better, sl, best)
            kd = np.where(better, k, kd)
        fire = best < TRIGGER
        sl_s = 100.0 * (ths - th[l - 1]) / (ps - p[l - 1])
        surf = ~fire & (sl_s < TRIGGER)
        kc = np.where(surf, l - 1, kd)
        fire_any = (fire | surf) & ocean
        kc = np.clip(kc, 0, l - 1)
        rhk = np.take_along_axis(rh, kc[None], 0)[0]
        rhk1 = np.take_along_axis(rh, np.minimum(kc + 1, l - 1)[None], 0)[0]
        lim = np.minimum(strat, np.maximum(rhk, rhk1))
        out["strat"].append(strat)
        out["fire"].append(fire_any.astype(float))
        out["cam"].append(np.where(fire_any, lim, 0.0))
        out["notrig"].append(np.where(ocean, lim, 0.0))
        out["lts"].append(np.where(ocean, lts, np.nan))
        out["dthdp"].append(np.where(ocean, best, np.nan))
        out["month"].append(int(((s * dt) % year) / (year / 12.0)) + 1)
    arr = {k: np.asarray(v) for k, v in out.items()}
    w2 = np.cos(np.radians(lat))[:, None] * np.ones((nlat, nlon))
    res = {"n_snapshots": len(steps), "k700_sigma": float(sig[k700]), "regions": {}}
    for name, la0, la1, lo0, lo1, months in REGIONS:
        m = (lat[:, None] >= la0) & (lat[:, None] <= la1) & (lon[None] >= lo0) & (lon[None] <= lo1) & ~land
        if not m.any():
            continue
        r = {}
        for tag, sel in (("all", np.ones(len(steps), bool)),
                         ("peak", np.isin(arr["month"], months) if months else None)):
            if sel is None or not sel.any():
                continue

            def mean(x):
                x = x[sel][:, m]
                ww = np.broadcast_to(w2[m], x.shape)
                g = np.isfinite(x)
                return float((x[g] * ww[g]).sum() / ww[g].sum()) if g.any() else float("nan")

            r[tag] = {k: mean(arr[k]) for k in ("strat", "fire", "cam", "notrig", "lts", "dthdp")}
        res["regions"][name] = r
    res["maps"] = {k: np.nanmean(arr[k], axis=0) for k in ("strat", "cam", "notrig", "fire")}
    res["lat"], res["lon"], res["land"] = lat, lon, land
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--json")
    ap.add_argument("--fig")
    a = ap.parse_args()
    r = analyse(a.run)
    print(f"{a.run}: {r['n_snapshots']} snapshots, k700 at σ {r['k700_sigma']:.3f}\n")
    print("| region | season | LTS [K] | strat (LTS law) | trigger fires (Δθ/Δp < −0.125) "
          "| cloud, CAM (trigger + RH limit) | cloud, no trigger (+ RH limit) | mean min Δθ/Δp |")
    print("|---|---|---|---|---|---|---|---|")
    for name, rr in r["regions"].items():
        for tag in ("all", "peak"):
            if tag in rr:
                x = rr[tag]
                print(f"| {name} | {tag} | {x['lts']:.1f} | {x['strat']:.2f} | {x['fire']:.2f} | {x['cam']:.2f} "
                      f"| {x['notrig']:.2f} | {x['dthdp']:+.3f} |")
    if a.json:
        js = {k: v for k, v in r.items() if k not in ("maps", "lat", "lon", "land")}
        pathlib.Path(a.json).write_text(json.dumps(js, indent=1))
    if a.fig:
        figure(r, a.fig, a.run)


def figure(r, path, title):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    lat, lon = r["lat"], r["lon"]
    order = np.argsort(np.where(lon > 180, lon - 360, lon))
    lo = np.where(lon > 180, lon - 360, lon)[order]
    fig, axs = plt.subplots(2, 2, figsize=(13, 7))
    for ax, (key, lab) in zip(axs.flat, (
            ("strat", "LTS law alone: clamp(0.057 LTS − 0.5573, 0, 1)"),
            ("fire", "share of snapshots with Δθ/Δp < −0.125 K/hPa (CAM trigger)"),
            ("cam", "stratus as CAM (trigger, RH limit)"),
            ("notrig", "stratus without the trigger (RH limit)"))):
        im = ax.pcolormesh(lo, lat, r["maps"][key][:, order], vmin=0, vmax=1, cmap="Blues", shading="auto")
        ax.contour(lo, lat, r["land"][:, order].astype(float), [0.5], colors="k", linewidths=0.5)
        for name, la0, la1, lo0, lo1, months in REGIONS[:5]:
            x0 = lo0 - 360 if lo0 > 180 else lo0
            ax.add_patch(plt.Rectangle((x0, la0), lo1 - lo0, la1 - la0, fill=False, ec="r", lw=1))
        ax.set_title(lab, fontsize=9)
        ax.set_ylim(-60, 60)
        fig.colorbar(im, ax=ax, shrink=0.8)
    fig.suptitle(f"{title}: time means over the climatology window", fontsize=10)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


if __name__ == "__main__":
    main()
