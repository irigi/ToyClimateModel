#!/usr/bin/env python3
"""Polar cloud-fraction profiles against the local tropopause, and the TOA longwave cloud effect
poleward of 65° (P4 question 13, owner decision after step 7 of 2026-11-24, step A; diagnostic,
record only).

    .venv/bin/python -I tools/polar_cloud_profile.py --pair <label> <run_a> <run_b> [--pair ...] \
        [--toa <label> <run_a> <run_b> ...] [--names r6 frozen] [--fig out.png] [--json out.json]

(a pair label "text|name" names the second run of that pair instead of --names)

For each `--pair` (runs that saved 3-D `cl` snapshots) and each polar cap (65–90°N, 65–90°S,
area-weighted):

- layer cloud fraction per level: time mean of the `cl` snapshots inside the climatology
  window, zonal mean; summed over the levels of four layers — above the tropopause, upper
  troposphere (tropopause to 440 hPa), middle (440–680 hPa), lower (below 680 hPa; ISCCP
  pressure boundaries). The sums are of layer fractions, not of cover (no overlap applied);
- the local tropopause: WMO lapse-rate definition (lowest level above 500 hPa where the lapse
  rate between it and the level above is 2 K km⁻¹ or less and the mean lapse rate to the
  highest level within 2 km above stays at or below 2 K km⁻¹) on the climatological zonal-mean
  temperature `zonal_t` of each latitude, heights from the hypsometric equation; the cap value
  is the area mean of the tropopause pressure (level resolution of L26: 30–40 hPa there);
- TOA LW CRE = olr_cs − olr from the climatology maps, per latitude and as a cap mean.

A layer split of the LW cloud effect is not computed: the runs saved neither humidity nor
level fluxes, and `toyclim radiation` evaluates clear-sky columns only. (A bookkeeping kernel
c ε (OLR_cs − σT_k⁴) on zonal means was tried on 2026-11-24 and failed its own check: −18 W m⁻²
summed against the model's +27 W m⁻² over 65–90°N, because it ignores the emission above the
cloud; it was dropped.)

`--toa` pairs (runs without 3-D cloud) print the cap means of TOA LW CRE, clear-sky OLR and the
tropopause pressure.
"""
import argparse
import glob
import json
import pathlib

import numpy as np

G, RD = 9.80665, 287.04
L26_HALF = np.array([0.0, 0.004895209, 0.009882418, 0.01805201, 0.02983724, 0.04462334,
                     0.06160587, 0.07851243, 0.0923658, 0.10866359, 0.12783708, 0.15039371,
                     0.17693043, 0.20814944, 0.24487709, 0.28808522, 0.33891731, 0.39871865,
                     0.4690718, 0.55183871, 0.64920969, 0.74438289, 0.83102123, 0.90330029,
                     0.95599746, 0.9851122, 1.0])
CAPS = [("65-90N", 65.0, 90.0), ("65-90S", -90.0, -65.0)]
LAYERS = ["above tropopause", "upper trop. (<440 hPa)", "middle (440-680 hPa)", "lower (>680 hPa)"]


def full_sigma(half):
    """Full levels as in toyclim's SigmaLevels::from_half (Simmons & Burridge 1981)."""
    out = np.empty(len(half) - 1)
    for k in range(len(out)):
        ds = half[k + 1] - half[k]
        alpha = np.log(2.0) if half[k] == 0.0 else 1.0 - half[k] / ds * np.log(half[k + 1] / half[k])
        out[k] = np.exp(np.log(half[k + 1]) - alpha)
    return out


SIG = full_sigma(L26_HALF)


def tropopause(t, ps):
    """WMO tropopause pressure [Pa] of one profile (index 0 = top), at level resolution."""
    p = SIG * ps
    z = np.zeros_like(p)
    for k in range(len(p) - 2, -1, -1):
        z[k] = z[k + 1] + RD * 0.5 * (t[k] + t[k + 1]) / G * np.log(p[k + 1] / p[k])
    for k in range(len(p) - 1, 0, -1):
        if p[k] > 50000.0:
            continue
        if -(t[k - 1] - t[k]) / (z[k - 1] - z[k]) * 1e3 > 2.0:
            continue
        above = [j for j in range(k - 1, -1, -1) if z[j] - z[k] <= 2000.0] or [k - 1]
        j = above[-1]
        if -(t[j] - t[k]) / (z[j] - z[k]) * 1e3 <= 2.0:
            return float(p[k])
    return float("nan")


def cap_weights(lat, lo, hi):
    w = np.cos(np.radians(lat)) * ((lat >= lo) & (lat <= hi))
    return w / w.sum()


def analyse(run, with_cl=True):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    c = run / "climatology"
    zt = np.load(c / "zonal_t.npy").astype(float)
    zps = np.load(c / "zonal_ps.npy").astype(float)
    olr = np.load(c / "mean_olr.npy").astype(float).mean(axis=-1)
    olr_cs = np.load(c / "mean_olr_cs.npy").astype(float).mean(axis=-1)
    o = {"lat": lat, "lw_cre": olr_cs - olr, "olr_cs": olr_cs,
         "ptp": np.array([tropopause(zt[:, j], zps[j]) for j in range(len(lat))]),
         "p": SIG[:, None] * zps[None, :]}
    if with_cl:
        start, dt = float(cfg.get("climatology", {}).get("start", 0.0)), float(cfg["dt"])
        acc, n = None, 0
        for f in glob.glob(str(run / "fields" / "cl_*.npy")):
            if int(pathlib.Path(f).stem.split("_")[-1]) * dt < start:
                continue
            z = np.load(f).astype(float).mean(axis=-1)
            acc = z if acc is None else acc + z
            n += 1
        o["cl"], o["n"] = acc / n, n
    return o


def caps(o):
    res = {}
    for cap, lo, hi in CAPS:
        w = cap_weights(o["lat"], lo, hi)
        ok = np.isfinite(o["ptp"])
        ptp = float((o["ptp"][ok] * w[ok]).sum() / w[ok].sum())
        d = {"ptp_hPa": ptp / 100.0, "lw_cre_toa": float((o["lw_cre"] * w).sum()),
             "olr_cs": float((o["olr_cs"] * w).sum())}
        if "cl" in o:
            pm = (o["p"] * w[None, :]).sum(axis=1)
            groups = [pm < ptp, (pm >= ptp) & (pm < 44000.0), (pm >= 44000.0) & (pm < 68000.0),
                      pm >= 68000.0]
            clp = (o["cl"] * w[None, :]).sum(axis=1)
            d.update({"p_hPa": (pm / 100.0).tolist(), "cl": clp.tolist(),
                      "cl_layers": [float(clp[g].sum()) for g in groups], "n": o["n"]})
        res[cap] = d
    m = np.abs(o["lat"]) >= 60.0
    res["lat"] = o["lat"][m].tolist()
    res["lw_cre_lat"] = o["lw_cre"][m].tolist()
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pair", nargs=3, action="append", default=[], metavar=("LABEL", "A", "B"))
    ap.add_argument("--toa", nargs=3, action="append", default=[], metavar=("LABEL", "A", "B"))
    ap.add_argument("--names", nargs=2, default=["r6", "frozen"])
    ap.add_argument("--fig")
    ap.add_argument("--json")
    a = ap.parse_args()
    na, nb = a.names
    out = {"pairs": {}, "toa": {}}
    for label, ra, rb in a.pair:
        # "label|name" overrides the second run's name for this pair
        label, nb = label.split("|") if "|" in label else (label, a.names[1])
        rr = {na: caps(analyse(ra)), nb: caps(analyse(rb)), "runs": [ra, rb], "names": [na, nb]}
        out["pairs"][label] = rr
        print(f"== {label}: {na} {ra} ({rr[na]['65-90N']['n']} snapshots) vs {nb} {rb} "
              f"({rr[nb]['65-90N']['n']})")
        for cap, _, _ in CAPS:
            x, y = rr[na][cap], rr[nb][cap]
            d = np.array(x["cl_layers"]) - np.array(y["cl_layers"])
            pos = d[d > 0].sum()
            print(f"  {cap}: tropopause {x['ptp_hPa']:.0f} / {y['ptp_hPa']:.0f} hPa; TOA LW CRE "
                  f"{x['lw_cre_toa']:.1f} / {y['lw_cre_toa']:.1f} ({x['lw_cre_toa'] - y['lw_cre_toa']:+.1f}) "
                  f"W m-2; OLR_cs {x['olr_cs']:.1f} / {y['olr_cs']:.1f}")
            print(f"    {'layer (sum of layer fractions)':32s}{na:>8s}{nb:>8s}{'change':>8s}{'of increase':>12s}")
            for i, name in enumerate(LAYERS):
                sh = f"{d[i] / pos:12.0%}" if d[i] > 0 else f"{'-':>12s}"
                print(f"    {name:32s}{x['cl_layers'][i]:8.3f}{y['cl_layers'][i]:8.3f}{d[i]:+8.3f}{sh}")
    nb = a.names[1]
    for label, ra, rb in a.toa:
        rr = {na: caps(analyse(ra, False)), nb: caps(analyse(rb, False)), "runs": [ra, rb]}
        out["toa"][label] = rr
        for cap, _, _ in CAPS:
            x, y = rr[na][cap], rr[nb][cap]
            print(f"== {label} {cap}: TOA LW CRE {x['lw_cre_toa']:.1f} / {y['lw_cre_toa']:.1f} "
                  f"({x['lw_cre_toa'] - y['lw_cre_toa']:+.1f}); OLR_cs {x['olr_cs']:.1f} / "
                  f"{y['olr_cs']:.1f}; tropopause {x['ptp_hPa']:.0f} / {y['ptp_hPa']:.0f} hPa")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(out, indent=1))
    if a.fig and out["pairs"]:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        labels = list(out["pairs"])
        fig, ax = plt.subplots(len(labels), 3, figsize=(14, 4.0 * len(labels)), squeeze=False)
        for i, label in enumerate(labels):
            rr = out["pairs"][label]
            na, nb = rr["names"]
            for tag, col in ((nb, "tab:blue"), (na, "tab:red")):
                for j, (cap, _, _) in enumerate(CAPS):
                    d = rr[tag][cap]
                    ax[i, j].plot(d["cl"], d["p_hPa"], color=col, marker=".", label=tag)
                    ax[i, j].axhline(d["ptp_hPa"], color=col, ls="--", lw=1)
                la, lw = np.array(rr[tag]["lat"]), np.array(rr[tag]["lw_cre_lat"])
                for hem, ls in ((la > 0, "-"), (la < 0, "--")):
                    ax[i, 2].plot(np.abs(la[hem]), lw[hem], color=col, ls=ls, marker=".",
                                  label=f"{tag} {'N' if ls == '-' else 'S'}")
            for j, (cap, _, _) in enumerate(CAPS):
                ax[i, j].set_title(f"{label}\n{cap}: cloud fraction per level", fontsize=9)
                ax[i, j].set_xlabel("layer cloud fraction (zonal, time mean)")
                ax[i, j].set_ylim(1000, 50)
                ax[i, j].set_ylabel("hPa")
                for pp in (440, 680):
                    ax[i, j].axhline(pp, color="0.6", lw=0.6, ls=":")
                ax[i, j].grid(alpha=0.3)
            ax[i, 0].legend(fontsize=8, title="dashed: WMO tropopause\ndotted: 440, 680 hPa",
                            title_fontsize=7)
            ax[i, 2].set_title(f"{label}\nTOA LW CRE (olr_cs − olr), |lat| ≥ 60°", fontsize=9)
            ax[i, 2].set_xlabel("|latitude| (solid N, dashed S)")
            ax[i, 2].set_ylabel("W m$^{-2}$")
            ax[i, 2].axvline(65, color="0.6", lw=0.6, ls=":")
            ax[i, 2].grid(alpha=0.3)
            ax[i, 2].legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(a.fig, dpi=100)


if __name__ == "__main__":
    main()
