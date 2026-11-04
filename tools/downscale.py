#!/usr/bin/env python3
"""Downscale a run's monthly climatology to the heightmap resolution and classify Köppen
climates there (first step of M11).

Usage:
    python tools/downscale.py out/<run> worlds/<heightmap>.npy [--lapse 6.5] [-o out.png]

- Temperature: bilinear interpolation of the monthly near-surface air temperature `tas`
  from the Gaussian grid, then a lapse-rate correction −Γ (h_fine − h_model), with h_model
  the model's filtered orography (`fields/orog_000000.npy`) interpolated the same way and
  Γ = 6.5 K/km by default.
- Precipitation: bilinear interpolation (orographic enhancement à la Smith & Barstad 2004
  is a later step).
Writes <run>/downscaled/{tas,precip}_monthly.npy ([month][lat][lon], K and mm/month),
koppen.npy and a PNG of the Köppen classes over land (h > 0).
"""
import argparse
import json
import pathlib
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.colors import ListedColormap  # noqa: E402

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import koppen  # noqa: E402


def bilinear(field, glat, glon, lat, lon):
    """Interpolate field[..., nlat, nlon] on the Gaussian grid (glat descending, glon from 0)
    to the target lat (descending, 1D) × lon (1D) grid. Radians."""
    nlon = glon.size
    # latitude: rows j, j+1 with glat[j] >= lat > glat[j+1]; clamp at the poles
    j = np.clip(np.searchsorted(-glat, -lat, side="right") - 1, 0, glat.size - 2)
    wj = np.clip((glat[j] - lat) / (glat[j] - glat[j + 1]), 0.0, 1.0)
    dl = 2 * np.pi / nlon
    x = (lon % (2 * np.pi)) / dl
    k0 = np.floor(x).astype(int) % nlon
    k1 = (k0 + 1) % nlon
    wk = x - np.floor(x)
    f = field
    a = f[..., j, :][..., :, k0] * (1 - wk) + f[..., j, :][..., :, k1] * wk
    b = f[..., j + 1, :][..., :, k0] * (1 - wk) + f[..., j + 1, :][..., :, k1] * wk
    return a * (1 - wj)[:, None] + b * wj[:, None]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("heightmap")
    ap.add_argument("--lapse", type=float, default=6.5, help="K per km")
    ap.add_argument("-o", "--output", default=None)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    cal = meta["calendar"]
    c = run / "climatology"
    glat = np.asarray(meta["grid"]["lat"])
    glon = np.asarray(meta["grid"]["lon"])
    h = np.load(a.heightmap).astype(float)
    nlat, nlon = h.shape
    lat = np.pi / 2 - np.pi * (np.arange(nlat) + 0.5) / nlat
    lon = 2 * np.pi * (np.arange(nlon) + 0.5) / nlon
    tas = np.load(c / "monthly_tas.npy").astype(float)
    pr = np.load(c / "monthly_precip.npy").astype(float) * cal["year"] / len(cal["months"])
    orog = np.load(run / "fields" / "orog_000000.npy").astype(float)
    t_f = bilinear(tas, glat, glon, lat, lon)
    p_f = np.maximum(bilinear(pr, glat, glon, lat, lon), 0.0)
    h_model = bilinear(orog, glat, glon, lat, lon)
    t_f -= a.lapse / 1000.0 * (np.maximum(h, 0.0) - h_model)[None]
    out = run / "downscaled"
    out.mkdir(exist_ok=True)
    np.save(out / "tas_monthly.npy", t_f.astype(np.float32))
    np.save(out / "precip_monthly.npy", p_f.astype(np.float32))
    land = h > 0
    codes = np.full((nlat, nlon), -1, int)
    Tc = t_f - 273.15
    for jj in range(nlat):
        for ii in range(nlon):
            if land[jj, ii]:
                codes[jj, ii] = koppen.CODE[koppen.classify(Tc[:, jj, ii], p_f[:, jj, ii])]
    np.save(out / "koppen.npy", codes)
    w = np.cos(lat)[:, None] * np.ones((1, nlon))
    tot = (w * land).sum()
    print("class  land-area fraction")
    for k, (name, _) in enumerate(koppen.CLASSES):
        f = (w * (codes == k)).sum() / tot
        if f > 0.001:
            print(f"{name:<5}  {f:.3f}")
    cmap = ListedColormap([col for _, col in koppen.CLASSES])
    fig, ax = plt.subplots(figsize=(12, 6), constrained_layout=True)
    ax.imshow(np.where(land, np.nan, 1.0), extent=[0, 360, -90, 90], cmap="Blues", vmin=0, vmax=3)
    ax.imshow(np.where(land, codes, np.nan), extent=[0, 360, -90, 90], cmap=cmap, vmin=-0.5,
              vmax=len(koppen.CLASSES) - 0.5, interpolation="nearest")
    present = sorted(set(codes[land].ravel()))
    handles = [plt.Rectangle((0, 0), 1, 1, color=koppen.CLASSES[k][1]) for k in present]
    ax.legend(handles, [koppen.CLASSES[k][0] for k in present], loc="center left", bbox_to_anchor=(1.0, 0.5), fontsize=8)
    ax.set_xlabel("longitude [°E]")
    ax.set_ylabel("latitude [°]")
    ax.set_title(f"Köppen–Geiger classes, downscaled ({nlat}×{nlon}, lapse {a.lapse} K/km): {run.name}", fontsize=10)
    png = a.output or str(out / "koppen.png")
    fig.savefig(png, dpi=110)
    print(png)


if __name__ == "__main__":
    main()
