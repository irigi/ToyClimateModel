"""Maps of low cloud for P4 low-cloud variants (owner step 4 of 2026-11-23).

    python tools/low_cloud_map.py <run> [<run> ...] [--labels a b ...] --fig out.png

Per run, from the snapshots `cl` inside the climatology window: the time-mean largest cloud
fraction over the levels σ ≥ 0.8 ("low cloud"); and, when the climatology has `cl_lts`, the
mean stability stratus placed by `low_cloud = "slingo_lts"`. Also prints area-weighted means of
the low cloud over the deck boxes of tools/inversion_check.py and globally.
"""

import argparse
import json
import pathlib

import numpy as np

from inversion_check import REGIONS


def low_cloud(run):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"])) % 360
    sig = np.asarray(meta["sigma"], float)
    nlat, nlon, l = len(lat), len(lon), len(sig)
    t0 = float(cfg.get("climatology", {}).get("start", 0.0))
    dt = float(cfg["dt"])
    fs = [f for f in (run / "fields").glob("cl_*.npy") if f.stem[3:].isdigit() and int(f.stem[3:]) * dt >= t0]
    low = np.mean([np.load(f).astype(float).reshape(l, nlat, nlon)[sig >= 0.8].max(axis=0) for f in fs], axis=0)
    lts = run / "climatology" / "mean_cl_lts.npy"
    lts = np.load(lts).astype(float).reshape(nlat, nlon) if lts.exists() else None
    landf = sorted((run / "fields").glob("land_*.npy"))
    land = np.load(landf[0]).reshape(nlat, nlon) > 0.5 if landf else np.zeros((nlat, nlon), bool)
    return lat, lon, low, lts, land, len(fs)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--fig", required=True)
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, axs = plt.subplots(len(a.runs), 2, figsize=(13, 3.2 * len(a.runs)), squeeze=False)
    print("| run | snapshots | global low | " + " | ".join(r[0] for r in REGIONS[:5]) + " |")
    print("|---|---|---|" + "---|" * 5)
    for i, (lab, run) in enumerate(zip(labels, a.runs)):
        lat, lon, low, lts, land, n = low_cloud(run)
        w = np.cos(np.radians(lat))[:, None] * np.ones_like(low)
        cells = []
        for name, la0, la1, lo0, lo1, _ in REGIONS[:5]:
            m = (lat[:, None] >= la0) & (lat[:, None] <= la1) & (lon[None] >= lo0) & (lon[None] <= lo1) & ~land
            cells.append(f"{(low[m] * w[m]).sum() / w[m].sum():.2f}")
        print(f"| {lab} | {n} | {(low * w).sum() / w.sum():.3f} | " + " | ".join(cells) + " |")
        order = np.argsort(np.where(lon > 180, lon - 360, lon))
        lo = np.where(lon > 180, lon - 360, lon)[order]
        for j, (z, t) in enumerate(((low, "low cloud (max over σ ≥ 0.8)"), (lts, "stability stratus cl_lts"))):
            ax = axs[i, j]
            if z is None:
                ax.set_axis_off()
                continue
            im = ax.pcolormesh(lo, lat, z[:, order], vmin=0, vmax=1, cmap="Blues", shading="auto")
            ax.contour(lo, lat, land[:, order].astype(float), [0.5], colors="k", linewidths=0.5)
            ax.set_title(f"{lab}: {t}", fontsize=9)
            fig.colorbar(im, ax=ax, shrink=0.8)
    fig.tight_layout()
    fig.savefig(a.fig, dpi=100)


if __name__ == "__main__":
    main()
