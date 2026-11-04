#!/usr/bin/env python3
"""Moist runs: zonal-mean precipitation and evaporation (mm/day) from the online
climatology, global means and the water budget.

Usage:
    python tools/moist_stats.py out/<run> [-o figure.png]

Needs [climatology] maps "precip" and "evap" (kg m-2 s-1) and optionally "pw" (kg m-2).
"""
import argparse
import json
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

DAY = 86400.0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("-o", "--output", default=None)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    info = json.loads((c / "sums.json").read_text())
    lat = np.asarray(meta["grid"]["lat"])
    w = np.asarray(meta["grid"]["weights"]) / 2.0
    latd = np.degrees(lat)
    p = np.load(c / "mean_precip.npy") * DAY
    e = np.load(c / "mean_evap.npy") * DAY
    zp, ze = p.mean(axis=1), e.mean(axis=1)
    gp, ge = (zp * w).sum(), (ze * w).sum()
    print(f"samples {info['count']} from day {info['start_s']/DAY:.0f}")
    print(f"global mean precipitation {gp:.3f} mm/day, evaporation {ge:.3f} mm/day, P/E = {gp/ge:.3f}")
    jmax = np.argmax(zp)
    print(f"zonal-mean precipitation max {zp[jmax]:.2f} mm/day at {latd[jmax]:.1f}°")
    if (c / "mean_pw.npy").exists():
        pw = np.load(c / "mean_pw.npy").mean(axis=1)
        print(f"global mean precipitable water {(pw * w).sum():.2f} mm")
    fig, axs = plt.subplots(1, 2, figsize=(12, 4), constrained_layout=True)
    axs[0].plot(latd, zp, label="precipitation")
    axs[0].plot(latd, ze, label="evaporation")
    axs[0].plot(latd, zp - ze, label="P − E", lw=0.8, ls="--")
    axs[0].axhline(0, color="k", lw=0.4)
    axs[0].set_xlabel("latitude [°]")
    axs[0].set_ylabel("mm/day")
    axs[0].set_xlim(-90, 90)
    axs[0].legend()
    axs[0].set_title("time-mean zonal means", fontsize=10)
    lon = np.degrees(np.asarray(meta["grid"]["lon"]))
    m = axs[1].pcolormesh(lon, latd, p, cmap="Blues", shading="nearest")
    axs[1].set_title("time-mean precipitation [mm/day]", fontsize=10)
    axs[1].set_xlabel("longitude [°E]")
    fig.colorbar(m, ax=axs[1], shrink=0.85)
    fig.suptitle(f"{run.name}: T{meta['truncation']}", fontsize=11)
    out = a.output or str(run / "moist.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
