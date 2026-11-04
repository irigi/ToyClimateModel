#!/usr/bin/env python3
"""Generate a simple procedural planet heightmap (stand-in until the procedural terrain
milestone): random spherical-harmonic noise with a red power spectrum, thresholded to a
target land fraction.

Usage:
    python tools/make_planet.py worlds/<name>.npy [--seed 1] [--land 0.3] [--lmax 48]
        [--slope 2.2] [--max-height 4000] [--ocean-depth 4000] [--nlat 180]

Writes an equirectangular float32 array [lat][lon] (row 0 = 90°N edge row centre,
column 0 = 0°E), metres above sea level (negative = ocean), plus a PNG preview.
"""
import argparse
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402


def legendre_table(lmax, mu):
    """Fully normalized associated Legendre functions P̄_l^m(mu) for 0 ≤ m ≤ l ≤ lmax."""
    nmu = mu.size
    P = np.zeros((lmax + 1, lmax + 1, nmu))
    s = np.sqrt(1 - mu**2)
    P[0, 0] = 1 / np.sqrt(2)
    for m in range(1, lmax + 1):
        P[m, m] = np.sqrt((2 * m + 1) / (2 * m)) * s * P[m - 1, m - 1]
    for m in range(0, lmax):
        P[m + 1, m] = np.sqrt(2 * m + 3) * mu * P[m, m]
    for m in range(0, lmax + 1):
        for l in range(m + 2, lmax + 1):
            e1 = np.sqrt((l * l - m * m) / (4 * l * l - 1))
            e0 = np.sqrt(((l - 1) ** 2 - m * m) / (4 * (l - 1) ** 2 - 1))
            P[l, m] = (mu * P[l - 1, m] - e0 * P[l - 2, m]) / e1
    return P


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("output")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--land", type=float, default=0.3, help="target land fraction")
    ap.add_argument("--lmax", type=int, default=48)
    ap.add_argument("--slope", type=float, default=1.4, help="power ~ l^-slope")
    ap.add_argument("--max-height", type=float, default=4000.0)
    ap.add_argument("--ocean-depth", type=float, default=4000.0)
    ap.add_argument("--nlat", type=int, default=180)
    a = ap.parse_args()
    rng = np.random.default_rng(a.seed)
    nlat, nlon = a.nlat, 2 * a.nlat
    lat = np.pi / 2 - np.pi * (np.arange(nlat) + 0.5) / nlat
    lon = 2 * np.pi * (np.arange(nlon) + 0.5) / nlon
    P = legendre_table(a.lmax, np.sin(lat))
    h = np.zeros((nlat, nlon))
    for l in range(1, a.lmax + 1):
        amp = l ** (-a.slope / 2) / np.sqrt(2 * l + 1)
        for m in range(0, l + 1):
            c = rng.normal() * amp
            s = rng.normal() * amp if m > 0 else 0.0
            h += P[l, m][:, None] * (c * np.cos(m * lon)[None, :] + s * np.sin(m * lon)[None, :])
    # area-weighted threshold for the land fraction
    w = np.cos(lat)[:, None] * np.ones((1, nlon))
    order = np.argsort(h.ravel())
    cw = np.cumsum(w.ravel()[order]) / w.sum()
    sea = h.ravel()[order][np.searchsorted(cw, 1 - a.land)]
    h = h - sea
    land, ocean = h > 0, h <= 0
    h[land] = a.max_height * (h[land] / h[land].max()) ** 1.5
    h[ocean] = -a.ocean_depth * (h[ocean] / h[ocean].min()) ** 0.5
    out = pathlib.Path(a.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    np.save(out, h.astype(np.float32))
    frac = (w * land).sum() / w.sum()
    fig, ax = plt.subplots(figsize=(10, 5), constrained_layout=True)
    m = ax.imshow(h, extent=[0, 360, -90, 90], cmap="terrain", vmin=-a.ocean_depth, vmax=a.max_height)
    ax.contour(np.degrees(lon), np.degrees(lat), h, levels=[0], colors="k", linewidths=0.6)
    fig.colorbar(m, ax=ax, shrink=0.8, label="m")
    ax.set_title(f"{out.stem}: seed {a.seed}, land fraction {frac:.2f}")
    fig.savefig(out.with_suffix(".png"), dpi=90)
    print(f"{out}: land fraction {frac:.3f}, max {h.max():.0f} m, min {h.min():.0f} m")


if __name__ == "__main__":
    main()
