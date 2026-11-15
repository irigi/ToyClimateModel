#!/usr/bin/env python3
"""Tidally locked runs: time-mean maps and the metrics of SPEC.md M5.

Usage:
    python tools/tidally_locked.py out/<run> [-o figure.png]

Uses the [climatology] maps t850, u850, v850 and t500 when present (missing maps are
skipped with a note). The substellar longitude is read from the run's config
(`[orbit] substellar_lon` or `[forcing] substellar_lon`, default 180°). Prints the zonal-mean u at the equator above σ = 0.5 (superrotation), and the day–night
temperature contrast at 850 and 500 hPa (weak temperature gradient regime: contrast
aloft much smaller than near the surface).
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
    ap.add_argument("-o", "--output", default=None)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"]))
    sigma = np.asarray(meta["sigma"])
    om = meta["planet"]["rotation_rate"] / 7.292e-5
    cfg = meta.get("config", {})
    lon_ss = 180.0
    for block in ("orbit", "forcing"):
        b = cfg.get(block) or {}
        if isinstance(b, dict) and b.get("substellar_lon") is not None:
            lon_ss = float(b["substellar_lon"])
    maps = {}
    for f in ["t850", "u850", "v850", "t500"]:
        path = c / f"mean_{f}.npy"
        if path.exists():
            maps[f] = np.load(path)
        else:
            print(f"note: no climatology map {f} in this run (add it to [climatology] maps)")
    zu = np.load(c / "zonal_u.npy")
    jeq = np.argsort(np.abs(lat))[:2]
    upper = sigma < 0.5
    ueq = zu[:, jeq].mean(axis=1)
    print(f"Omega/Omega_E = {om:.3g}")
    print("equatorial zonal-mean u above sigma=0.5: " + ", ".join(f"{s:.2f}:{u:.1f}" for s, u in zip(sigma[upper], ueq[upper])))
    print(f"  mean {ueq[upper].mean():.2f} m/s ({'superrotating' if ueq[upper].mean() > 0 else 'not superrotating'})")
    w = np.cos(np.radians(lat))[:, None] * np.ones((1, len(lon)))
    day = (np.cos(np.radians(lat))[:, None] * np.cos(np.radians(lon - lon_ss))[None, :]) > 0
    for f in [f for f in ["t850", "t500"] if f in maps]:
        t = maps[f]
        td = (t * w)[day].sum() / w[day].sum()
        tn = (t * w)[~day].sum() / w[~day].sum()
        print(f"{f}: day-side mean {td:.1f} K, night-side mean {tn:.1f} K, contrast {td - tn:.1f} K, max-min {t.max() - t.min():.1f} K")

    fig, axs = plt.subplots(1, 2, figsize=(13, 4.2), constrained_layout=True)
    if "t850" in maps:
        m = axs[0].pcolormesh(lon, lat, maps["t850"], cmap="RdYlBu_r", shading="nearest")
        title = "time-mean T850 [K]"
        if "u850" in maps and "v850" in maps:
            s = 4
            axs[0].quiver(lon[::s], lat[::s], maps["u850"][::s, ::s], maps["v850"][::s, ::s],
                          scale=400, width=0.002)
            title += " and 850 hPa wind"
        axs[0].set_title(title, fontsize=10)
        fig.colorbar(m, ax=axs[0], shrink=0.85)
    else:
        axs[0].text(0.5, 0.5, "no t850 map", ha="center", transform=axs[0].transAxes)
    axs[0].set_xlabel(f"longitude [°E] (substellar at {lon_ss:g}°)")
    axs[0].set_ylabel("latitude [°]")
    vmax = np.abs(zu).max()
    cs = axs[1].contourf(lat, sigma, zu, levels=15, cmap="RdBu_r", vmin=-vmax, vmax=vmax)
    axs[1].contour(lat, sigma, zu, levels=15, colors="k", linewidths=0.4)
    axs[1].set_ylim(1, 0)
    axs[1].set_title("time-mean zonal-mean u [m/s]", fontsize=10)
    axs[1].set_xlabel("latitude [°]")
    axs[1].set_ylabel("σ")
    fig.colorbar(cs, ax=axs[1], shrink=0.85)
    fig.suptitle(f"{run.name}: tidally locked, Ω/Ω⊕ = {om:.3g}, T{meta['truncation']}", fontsize=11)
    out = a.output or str(run / "tidally_locked.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
