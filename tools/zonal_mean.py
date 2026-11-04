#!/usr/bin/env python3
"""Time-mean zonal-mean cross sections from a run's online climatology, plus the
Held–Suarez acceptance metrics.

Usage:
    python tools/zonal_mean.py out/<run> [-o figure.png] [--title TEXT]

Reads out/<run>/climatology/zonal_*.npy (written when the config has a [climatology]
block) and meta.json (σ levels, latitudes). Prints:
- jets: per hemisphere max of [u], its latitude and σ
- lowest-level [u] averaged over |φ| < 15° and 30° < |φ| < 60°
- max hemispheric asymmetry max |[u](φ) − [u](−φ)|
- Hadley cell extent: latitude where the lower-tropospheric mean meridional
  streamfunction changes sign poleward of its tropical extremum
- if diagnostics.csv has kinetic_energy: relative trend over the accumulation window
"""
import argparse
import csv
import json
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402


def streamfunction(v, sigma_half, lat, ps, a, g):
    """Mean meridional streamfunction ψ(σ_half, φ) = 2πa cos φ / g ∫_0^p [v] dp [kg/s]."""
    dsig = np.diff(sigma_half)
    cum = np.vstack([np.zeros(v.shape[1]), np.cumsum(v * dsig[:, None], axis=0)])
    return 2 * np.pi * a * np.cos(lat)[None, :] * ps[None, :] / g * cum


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("-o", "--output", default=None)
    ap.add_argument("--title", default=None)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    info = json.loads((c / "sums.json").read_text())
    z = {n: np.load(c / f"zonal_{n}.npy") for n in ["u", "v", "t", "eddy_tt", "eddy_uv", "eddy_vt", "eddy_uu", "eddy_vv"]}
    ps = np.load(c / "zonal_ps.npy")
    sigma = np.asarray(meta["sigma"])
    sh = np.asarray(meta["config"].get("sigma_half") or np.linspace(0, 1, len(sigma) + 1))
    lat = np.asarray(meta["grid"]["lat"])
    latd = np.degrees(lat)
    planet = meta["planet"]
    u = z["u"]
    print(f"samples: {info['count']}  (start {info['start_s']/86400:.0f} d)")
    for name, sel in [("NH", latd > 0), ("SH", latd < 0)]:
        sub = np.where(sel[None, :], u, -np.inf)
        k, j = np.unravel_index(np.argmax(sub), u.shape)
        print(f"{name} jet: max [u] = {u[k, j]:.2f} m/s at {latd[j]:.1f}°, sigma = {sigma[k]:.3f}")
    low = u[-1]
    trop = low[np.abs(latd) < 15].mean()
    mid = low[(np.abs(latd) > 30) & (np.abs(latd) < 60)].mean()
    print(f"lowest level [u]: tropics (|lat|<15) {trop:.2f} m/s, midlatitudes (30-60) {mid:.2f} m/s")
    asym = np.abs(u - u[:, ::-1]).max()
    print(f"max hemispheric asymmetry of [u]: {asym:.2f} m/s")
    psi = streamfunction(z["v"], sh, lat, ps, planet["radius"], planet["gravity"])
    kmid = np.argmin(np.abs(sh - 0.7))
    prof = psi[kmid]
    for name, sel, sgn in [("NH", latd > 0, 1), ("SH", latd < 0, -1)]:
        idx = np.where(sel)[0]
        jx = idx[np.argmax(np.abs(prof[idx]) * (np.abs(latd[idx]) < 35))]
        s0 = np.sign(prof[jx])
        edge = None
        order = idx[np.argsort(np.abs(latd[idx]))]
        for j in order:
            if abs(latd[j]) > abs(latd[jx]) and np.sign(prof[j]) != s0:
                edge = abs(latd[j])
                break
        print(f"{name} Hadley cell: max |psi| {abs(prof[jx]):.3e} kg/s at {latd[jx]:.1f}°, edge ≈ {edge}°")
    diag = run / "diagnostics.csv"
    if diag.exists():
        rows = list(csv.DictReader(open(diag)))
        if rows and "kinetic_energy" in rows[0]:
            t = np.array([float(r["time_s"]) for r in rows]) / 86400
            ke = np.array([float(r["kinetic_energy"]) for r in rows])
            sel = t >= info["start_s"] / 86400
            if sel.sum() > 10:
                p = np.polyfit(t[sel], ke[sel], 1)
                span = t[sel][-1] - t[sel][0]
                print(f"KE trend over the window: {p[0]*span/ke[sel].mean()*100:.2f}% (mean {ke[sel].mean():.3e} J)")

    fig, axs = plt.subplots(2, 2, figsize=(11, 7.5), constrained_layout=True)
    def panel(ax, f, title, cmap="RdBu_r", sym=True, levels=None):
        lv = levels if levels is not None else 15
        vmax = np.abs(f).max() if sym else None
        cs = ax.contourf(latd, sigma, f, levels=lv, cmap=cmap, vmin=-vmax if sym else None, vmax=vmax)
        ax.contour(latd, sigma, f, levels=lv, colors="k", linewidths=0.4)
        ax.set_ylim(1, 0)
        ax.set_title(title, fontsize=10)
        ax.set_xlabel("latitude [°]")
        ax.set_ylabel("σ")
        fig.colorbar(cs, ax=ax, shrink=0.85)
    panel(axs[0, 0], u, "time-mean zonal-mean u [m/s]")
    panel(axs[0, 1], z["t"], "zonal-mean T [K]", cmap="viridis", sym=False)
    panel(axs[1, 0], z["eddy_tt"], "eddy temperature variance [K²]", cmap="magma", sym=False)
    shc = 0.5 * (sh[1:] + sh[:-1])
    psi_full = 0.5 * (psi[1:] + psi[:-1])
    cs = axs[1, 1].contourf(latd, shc, psi_full / 1e10, levels=15, cmap="RdBu_r")
    axs[1, 1].set_ylim(1, 0)
    axs[1, 1].set_title("mean meridional streamfunction [1e10 kg/s]", fontsize=10)
    axs[1, 1].set_xlabel("latitude [°]")
    fig.colorbar(cs, ax=axs[1, 1], shrink=0.85)
    fig.suptitle(a.title or f"{run.name}: T{meta['truncation']}, {info['count']} samples", fontsize=11)
    out = a.output or str(run / "zonal_mean.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
