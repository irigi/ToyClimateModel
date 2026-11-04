#!/usr/bin/env python3
"""Downscale a run's monthly climatology to the heightmap resolution and classify Köppen
climates there (first step of M11).

Usage:
    python tools/downscale.py out/<run> worlds/<heightmap>.npy [--lapse 6.5] [-o out.png]

- Temperature: bilinear interpolation of the monthly near-surface air temperature `tas`
  from the Gaussian grid, then a lapse-rate correction −Γ (h_fine − h_model), with h_model
  the model's filtered orography (`fields/orog_000000.npy`) interpolated the same way and
  Γ = 6.5 K/km by default.
- Precipitation: bilinear interpolation, plus (with --orographic) the linear theory of
  orographic precipitation of Smith & Barstad (2004, J. Atmos. Sci. 61, 1377–1391) applied
  to the unresolved terrain h_fine − h_model, on overlapping 40° tiles with the tile-mean
  monthly 850 hPa wind (`monthly_u850`, `monthly_v850` needed):
      P̂ = C_w i σ ĥ / [(1 − i m H_w)(1 + i σ τ_c)(1 + i σ τ_f)],  σ = U k + V l,
      m² = (N_m² − σ²)(k² + l²)/σ²,  C_w = ρ_vs Γ_m / γ,  H_w = R_v T² / (L γ),
  with N_m = 0.005 s⁻¹, τ_c = τ_f = 1000 s, Γ_m = 6 K/km, γ = 6.5 K/km; the result is
  added to the interpolated model precipitation and clipped at zero.
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


def smith_barstad(dh, dx, dy, U, V, T, Nm=0.005, tau=1000.0, gamma_env=6.5e-3, gamma_m=6.0e-3):
    """Linear orographic precipitation [kg m-2 s-1] for a height perturbation dh [m] on a
    Cartesian tile (rows = y northward decreasing index, i.e. north at row 0)."""
    L, Rv = 2.5e6, 461.5
    es = 610.78 * np.exp(L / Rv * (1 / 273.16 - 1 / T))
    cw = es / (Rv * T) * gamma_m / gamma_env
    hw = Rv * T * T / (L * gamma_env)
    ny, nx = dh.shape
    k = 2 * np.pi * np.fft.fftfreq(nx, dx)[None, :]
    l = -2 * np.pi * np.fft.fftfreq(ny, dy)[:, None]  # row index increases southward
    sigma = U * k + V * l
    kk = k * k + l * l
    with np.errstate(divide="ignore", invalid="ignore"):
        m2 = (Nm * Nm - sigma * sigma) * kk / (sigma * sigma)
        m = np.where(m2 >= 0, np.sign(sigma) * np.sqrt(np.abs(m2)), 1j * np.sqrt(np.abs(m2)))
        m = np.where(sigma == 0, 0.0, m)
        ph = np.fft.fft2(dh)
        pp = cw * 1j * sigma * ph / ((1 - 1j * m * hw) * (1 + 1j * sigma * tau) ** 2)
        pp = np.where(np.isfinite(pp), pp, 0.0)
    return np.real(np.fft.ifft2(pp))


def orographic(dh_full, lat, lon, u, v, t, a=6.371e6, tile=40.0, step=20.0, latmax=70.0):
    """Tile-wise Smith–Barstad on the equirectangular map; u, v, t on the same map."""
    nlat, nlon = dh_full.shape
    out = np.zeros_like(dh_full)
    wsum = np.zeros_like(dh_full)
    latd, lond = np.degrees(lat), np.degrees(lon)
    dlat = np.pi / nlat
    dlon = 2 * np.pi / nlon
    for c_lat in np.arange(-latmax + tile / 2, latmax - tile / 2 + 1e-9, step):
        rows = np.where(np.abs(latd - c_lat) <= tile / 2)[0]
        if rows.size < 4:
            continue
        for c_lon in np.arange(0.0, 360.0, step):
            dl = (lond - c_lon + 180.0) % 360.0 - 180.0
            cols = np.where(np.abs(dl) <= tile / 2)[0]
            cols = cols[np.argsort(dl[cols])]
            sub = dh_full[np.ix_(rows, cols)]
            if not np.any(sub > 0):
                continue
            ny, nx = sub.shape
            wy = np.hanning(ny + 2)[1:-1][:, None]
            wx = np.hanning(nx + 2)[1:-1][None, :]
            w = wy * wx
            pad = np.zeros((2 * ny, 2 * nx))
            pad[:ny, :nx] = sub
            coslat = np.cos(np.radians(c_lat))
            U = float(np.mean(u[np.ix_(rows, cols)]))
            V = float(np.mean(v[np.ix_(rows, cols)]))
            T = float(np.mean(t[np.ix_(rows, cols)]))
            p = smith_barstad(pad, a * coslat * dlon, a * dlat, U, V, T)[:ny, :nx]
            out[np.ix_(rows, cols)] += w * p
            wsum[np.ix_(rows, cols)] += w
    return np.where(wsum > 0, out / np.maximum(wsum, 1e-12), 0.0)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("heightmap")
    ap.add_argument("--lapse", type=float, default=6.5, help="K per km")
    ap.add_argument("--orographic", action="store_true", help="add Smith–Barstad orographic precipitation")
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
    if np.isnan(tas).any():
        raise SystemExit("monthly climatology incomplete; run at least one full year after the climatology start")
    pr = np.load(c / "monthly_precip.npy").astype(float) * cal["year"] / len(cal["months"])
    orog = np.load(run / "fields" / "orog_000000.npy").astype(float)
    t_f = bilinear(tas, glat, glon, lat, lon)
    p_f = np.maximum(bilinear(pr, glat, glon, lat, lon), 0.0)
    h_model = bilinear(orog, glat, glon, lat, lon)
    t_f -= a.lapse / 1000.0 * (np.maximum(h, 0.0) - h_model)[None]
    if a.orographic:
        month_s = cal["year"] / len(cal["months"])
        u_f = bilinear(np.load(c / "monthly_u850.npy").astype(float), glat, glon, lat, lon)
        v_f = bilinear(np.load(c / "monthly_v850.npy").astype(float), glat, glon, lat, lon)
        dh = np.maximum(h, 0.0) - np.maximum(h_model, 0.0)
        radius = meta["planet"]["radius"]
        for mth in range(len(cal["months"])):
            po = orographic(dh, lat, lon, u_f[mth], v_f[mth], t_f[mth], a=radius) * month_s
            p_f[mth] = np.maximum(p_f[mth] + po, 0.0)
        print("orographic precipitation added (Smith & Barstad 2004)")
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
