#!/usr/bin/env python3
"""Plot a toyclim output field: map, polar view or zonal mean.

Usage:
    python tools/plot_field.py out/<run> <field> [--step N] [--level K]
        [--kind map|polar|zonal] [-o file.png]

Reads out/<run>/meta.json for the Gaussian grid and
out/<run>/fields/<field>_<step:06>.npy for the data ([lat][lon] or [lev][lat][lon]).
"""
import argparse
import json
import pathlib
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402


def load(run, field, step):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    fdir = run / "fields"
    if step is None:
        files = sorted(fdir.glob(f"{field}_*.npy"))
        if not files:
            sys.exit(f"no files for field {field!r} in {fdir}")
        path = files[-1]
    else:
        path = fdir / f"{field}_{step:06d}.npy"
    data = np.load(path)
    step = int(path.stem.rsplit("_", 1)[1])
    return meta, data, step


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("field")
    ap.add_argument("--step", type=int, default=None, help="output step (default: last)")
    ap.add_argument("--level", type=int, default=None, help="level index for 3D fields (default: lowest)")
    ap.add_argument("--kind", choices=["map", "polar", "zonal"], default="map")
    ap.add_argument("--cmap", default="RdBu_r")
    ap.add_argument("-o", "--output", default=None)
    args = ap.parse_args()

    meta, data, step = load(args.run, args.field, args.step)
    if data.ndim == 3:
        lev = data.shape[0] - 1 if args.level is None else args.level
        data = data[lev]
        level_label = f", level {lev}"
    else:
        level_label = ""
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"]))
    dt = meta["config"]["dt"]
    t_days = step * dt / 86400.0
    title = f"{args.field}{level_label}  step {step}  t = {t_days:.3g} d  (T{meta['truncation']}, {meta['planet']['name']})"

    # Close the longitude seam for plotting.
    lon_c = np.append(lon, 360.0)
    data_c = np.concatenate([data, data[:, :1]], axis=1)

    if args.kind == "map":
        fig, ax = plt.subplots(figsize=(9, 4.6), constrained_layout=True)
        m = ax.pcolormesh(lon_c, lat, data_c, shading="nearest", cmap=args.cmap)
        ax.contour(lon_c, lat, data_c, colors="k", linewidths=0.4, levels=12)
        ax.set_xlabel("longitude [°E]")
        ax.set_ylabel("latitude [°N]")
        ax.set_xlim(0, 360)
        ax.set_ylim(-90, 90)
        ax.set_aspect("equal")
        fig.colorbar(m, ax=ax, shrink=0.8)
    elif args.kind == "polar":
        fig, axs = plt.subplots(1, 2, figsize=(10, 5), subplot_kw={"projection": "polar"}, constrained_layout=True)
        vmin, vmax = np.nanmin(data), np.nanmax(data)
        for ax, hemi in zip(axs, ("N", "S")):
            sel = lat > 0 if hemi == "N" else lat < 0
            r = 90 - np.abs(lat[sel])
            m = ax.pcolormesh(np.radians(lon_c), r, data_c[sel], shading="nearest", cmap=args.cmap, vmin=vmin, vmax=vmax)
            ax.set_ylim(0, 60)
            ax.set_yticklabels([])
            ax.set_title(f"{hemi}H (to {'30°N' if hemi == 'N' else '30°S'})")
        fig.colorbar(m, ax=axs, shrink=0.8)
    else:
        fig, ax = plt.subplots(figsize=(6, 4), constrained_layout=True)
        ax.plot(lat, data.mean(axis=1))
        ax.set_xlabel("latitude [°N]")
        ax.set_ylabel(f"zonal mean {args.field}")
        ax.set_xlim(-90, 90)
        ax.grid(alpha=0.3)
    fig.suptitle(title, fontsize=10)

    out = args.output or str(pathlib.Path(args.run) / f"{args.field}_{step:06d}_{args.kind}.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
