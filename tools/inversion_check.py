"""Does L26 resolve the trade inversion that the Slingo (1987) low cloud uses? (P4 question 13
option (iv), owner step 3 of 2026-11-23.)

    python tools/inversion_check.py <run> [--fig out.png] [--json out.json]

Needs 3-D snapshots `t`, `rh`, `omega` and 2-D `ts`, `ps`, `land` (one `land` file is enough).
For every snapshot and column, exactly as `physics/clouds/low.rs` does it: over the pairs of
adjacent full levels (k, k+1) with p_k ≥ 750 hPa and k+1 above the lowest level, the most
negative Δθ/Δp [K hPa⁻¹] (θ = T (1000 hPa/p)^κ); Slingo's eq. (8) C' = clamp(−6.67 Δθ/Δp −
0.667, 0, 1) and eq. (9) with the RH of the base level k+1. Also the lower-tropospheric
stability LTS = θ(700 hPa) − θ_s (θ at 700 hPa linear in ln p between levels; θ_s from `ts`
and `ps`), and the share of snapshots with ω ≥ 0 (subsidence, eq. (7) gives no RH low cloud) at
the levels σ ≥ 0.8 excluding the lowest.

Regions, ocean points only, all snapshots and the peak stratocumulus season:
- the five subtropical stratocumulus decks named in Slingo's Fig. 3 discussion ("off the
  western seaboards") with boxes and peak seasons as commonly used after Klein & Hartmann
  (1993) — the boxes are mine (that paper could not be fetched, 2026-11-23);
- the trade belts, 5–20° both hemispheres, all longitudes (ocean);
- the warm pool, 10°S–10°N, 130–170°E, as the convective contrast.

Prints a markdown table; `--fig` draws maps of the mean Δθ/Δp and C' and the mean θ profiles.
"""

import argparse
import json
import pathlib

import numpy as np

KAPPA = 287.04 / 1004.64
# name, lat0, lat1, lon0, lon1 (deg E, 0..360), peak months (1..12)
REGIONS = [
    ("Peru", -20, -10, 270, 280, (9, 10, 11)),
    ("Namibia", -20, -10, 0, 10, (6, 7, 8, 9, 10, 11)),
    ("California", 20, 30, 230, 240, (6, 7, 8)),
    ("Canary", 15, 25, 325, 335, (6, 7, 8)),
    ("Australia", -35, -25, 95, 105, (12, 1, 2)),
    ("trades 5-20N", 5, 20, 0, 360, None),
    ("trades 5-20S", -20, -5, 0, 360, None),
    ("warm pool", -10, 10, 130, 170, None),
]


def snapshots(run, name):
    fs = sorted((run / "fields").glob(f"{name}_*.npy"), key=lambda f: int(f.stem.split("_")[-1]))
    return {int(f.stem.split("_")[-1]): f for f in fs}


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
    fs = {n: snapshots(run, n) for n in ("t", "rh", "omega", "ts", "ps")}
    land = np.load(next(iter(snapshots(run, "land").values()))).reshape(nlat, nlon) > 0.5
    steps = sorted(s for s in fs["t"] if all(s in fs[n] for n in fs) and s * dt >= start)
    if not steps:
        raise SystemExit("no complete snapshots in the climatology window")
    out = {k: [] for k in ("dthdp", "c8", "cinv", "base", "lts", "sub", "month")}
    prof = []
    for s in steps:
        t = np.load(fs["t"][s]).astype(float).reshape(l, nlat, nlon)
        rh = np.load(fs["rh"][s]).astype(float).reshape(l, nlat, nlon)
        om = np.load(fs["omega"][s]).astype(float).reshape(l, nlat, nlon)
        ts = np.load(fs["ts"][s]).astype(float).reshape(nlat, nlon)
        ps = np.load(fs["ps"][s]).astype(float).reshape(nlat, nlon)
        p = sig[:, None, None] * ps[None]
        th = t * (1.0e5 / p) ** KAPPA
        best = np.full((nlat, nlon), np.inf)
        base = np.full((nlat, nlon), -1)
        for k in range(l - 2):
            ok = p[k] >= 75000.0
            sl = (th[k] - th[k + 1]) / ((p[k] - p[k + 1]) / 100.0)
            better = ok & (sl < best)
            best = np.where(better, sl, best)
            base = np.where(better, k + 1, base)
        has = np.isfinite(best)
        dthdp = np.where(has, best, np.nan)
        c8 = np.clip(-6.67 * dthdp - 0.667, 0.0, 1.0)
        bi = np.clip(base, 0, l - 1)
        rhb = np.take_along_axis(rh, bi[None], axis=0)[0]
        cinv = np.where(has, c8 * np.clip((rhb - 0.6) / 0.2, 0.0, 1.0), np.nan)
        # θ at 700 hPa, linear in ln p
        lp = np.log(p)
        target = np.log(70000.0)
        k7 = np.clip((lp < target).sum(axis=0) - 1, 0, l - 2)
        a = np.take_along_axis(lp, k7[None], 0)[0]
        b = np.take_along_axis(lp, (k7 + 1)[None], 0)[0]
        w = (target - a) / (b - a)
        th7 = (1 - w) * np.take_along_axis(th, k7[None], 0)[0] + w * np.take_along_axis(th, (k7 + 1)[None], 0)[0]
        lts = th7 - ts * (1.0e5 / ps) ** KAPPA
        low = (sig >= 0.8) & (np.arange(l) < l - 1)
        sub = (om[low] >= 0.0).mean(axis=0)
        for k_, v in (("dthdp", dthdp), ("c8", c8), ("cinv", cinv), ("base", np.where(has, sig[bi], np.nan)),
                      ("lts", lts), ("sub", sub)):
            out[k_].append(v)
        out["month"].append(int(((s * dt) % year) / (year / 12.0)) + 1)
        prof.append(th)
    arr = {k: np.asarray(v) for k, v in out.items()}
    prof = np.asarray(prof)
    w2 = np.cos(np.radians(lat))[:, None] * np.ones((nlat, nlon))
    res = {"n_snapshots": len(steps), "sigma": sig.tolist(), "regions": {}}
    for name, la0, la1, lo0, lo1, months in REGIONS:
        m = (lat[:, None] >= la0) & (lat[:, None] <= la1) & (lon[None] >= lo0) & (lon[None] <= lo1) & ~land
        if not m.any():
            continue
        r = {"points": int(m.sum())}
        for tag, sel in (("all", np.ones(len(steps), bool)),
                         ("peak", np.isin(arr["month"], months) if months else None)):
            if sel is None or not sel.any():
                continue
            ww = np.broadcast_to(w2, arr["dthdp"][sel].shape)[:, m]

            def mean(x):
                x = x[sel][:, m]
                g = np.isfinite(x)
                return float((x[g] * ww[g]).sum() / ww[g].sum()) if g.any() else float("nan")

            d = arr["dthdp"][sel][:, m]
            bs = arr["base"][sel][:, m]
            vals, cnt = np.unique(np.round(bs[np.isfinite(bs)], 3), return_counts=True)
            r[tag] = {
                "dthdp": mean(arr["dthdp"]), "frac_lt_0.1": float(np.nanmean(d < -0.1)),
                "frac_lt_0.25": float(np.nanmean(d < -0.25)), "c8": mean(arr["c8"]),
                "cinv": mean(arr["cinv"]), "lts": mean(arr["lts"]), "subsidence": mean(arr["sub"]),
                "base_sigma": {f"{v:.3f}": round(float(c) / cnt.sum(), 3) for v, c in zip(vals, cnt)},
                "theta_profile": [float((prof[sel][:, k][:, m] * ww).sum() / ww.sum()) for k in range(l)],
            }
        res["regions"][name] = r
    res["maps"] = {k: np.nanmean(arr[k], axis=0) for k in ("dthdp", "c8", "cinv", "lts", "sub")}
    res["lat"], res["lon"], res["land"] = lat, lon, land
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--fig")
    ap.add_argument("--json")
    a = ap.parse_args()
    r = analyse(a.run)
    print(f"{a.run}: {r['n_snapshots']} snapshots\n")
    print("| region | season | Δθ/Δp mean [K/hPa] | P(<−0.1) | P(<−0.25) | C' eq. 8 | C_inv eq. 9 | LTS [K] "
          "| ω≥0 share (σ≥0.8) | base σ (share) |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for name, rr in r["regions"].items():
        for tag in ("all", "peak"):
            if tag not in rr:
                continue
            x = rr[tag]
            bs = ", ".join(f"{k} ({v:.2f})" for k, v in sorted(x["base_sigma"].items(), key=lambda kv: -kv[1])[:2])
            print(f"| {name} | {tag} | {x['dthdp']:+.3f} | {x['frac_lt_0.1']:.2f} | {x['frac_lt_0.25']:.2f} | "
                  f"{x['c8']:.2f} | {x['cinv']:.2f} | {x['lts']:.1f} | {x['subsidence']:.2f} | {bs} |")
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
    fig = plt.figure(figsize=(13, 9))
    for i, (key, lab, vmin, vmax, cmap) in enumerate((
            ("dthdp", "mean Δθ/Δp of the most stable layer below 750 hPa [K/hPa]", -0.25, 0.0, "viridis_r"),
            ("c8", "mean Slingo eq. (8) C' (no RH factor)", 0, 0.6, "Blues"),
            ("cinv", "mean C_inv (eqs. 8–9)", 0, 0.6, "Blues"),
            ("sub", "share of ω ≥ 0 at σ ≥ 0.8 (eq. 7 suppression)", 0, 1, "RdBu_r"))):
        ax = fig.add_subplot(3, 2, i + 1)
        z = r["maps"][key][:, order]
        im = ax.pcolormesh(lo, lat, z, vmin=vmin, vmax=vmax, cmap=cmap, shading="auto")
        ax.contour(lo, lat, r["land"][:, order].astype(float), [0.5], colors="k", linewidths=0.5)
        for name, la0, la1, lo0, lo1, months in REGIONS[:5]:
            x0 = lo0 - 360 if lo0 > 180 else lo0
            ax.add_patch(plt.Rectangle((x0, la0), lo1 - lo0, la1 - la0, fill=False, ec="r", lw=1))
        ax.set_title(lab, fontsize=9)
        ax.set_ylim(-60, 60)
        fig.colorbar(im, ax=ax, shrink=0.8)
    ax = fig.add_subplot(3, 2, (5, 6))
    sig = np.asarray(r["sigma"])
    for name, rr in r["regions"].items():
        x = rr.get("peak", rr["all"])
        ax.plot(x["theta_profile"], sig * 1000, marker=".", label=f"{name} ({'peak' if 'peak' in rr else 'all'})")
    ax.set_ylim(1000, 600)
    ax.set_xlim(285, 330)
    ax.set_xlabel("θ [K]")
    ax.set_ylabel("σ × 1000")
    ax.axhline(750, color="0.6", lw=0.5)
    ax.legend(fontsize=7, ncol=2)
    ax.grid(alpha=0.3)
    ax.set_title("mean θ profile (model levels marked)", fontsize=9)
    fig.suptitle(str(title), fontsize=10)
    fig.tight_layout()
    fig.savefig(path, dpi=100)


if __name__ == "__main__":
    main()
