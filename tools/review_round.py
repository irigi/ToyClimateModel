"""Tropical cloud diagnostics of one or more runs side by side (P4 question 13, review round of
2026-11-23: anvil-pair split, fixed-SST factorial on slab SSTs, Earth low-cloud split).

    python tools/review_round.py <run> [<run> ...] [--labels a b ...] [--band 20] [--json out.json]

Each run needs `[clouds] cre_split = true` and climatology maps asr, asr_cs, olr, olr_cs, asr_nc,
olr_nc, clt, cc_conv, cwp_conv, precip, precip_conv, conv_active, conv_skipped, cape; snapshots
`cl` and `cl_split` are used when present. Means over |φ| ≤ band (default 20°), area-weighted:

- CRE [W m⁻²] all-cloud (SW = asr − asr_cs, LW = olr_cs − olr) and of the extra call
  ("nc": SW = asr_nc − asr_cs, LW = olr_cs − olr_nc); "removed" = all − nc, which is the
  convective part for the convective split and the low-stratiform part with
  `cre_split_low_sigma` (overlap given to the removed part; not additive under random overlap);
- `clt`, the time-mean convective fraction and in-cloud path (`cc_conv`, `cwp_conv`, zero when
  no convective cloud, so frequency-weighted), the deep-convection and skipped (CAPE > 0,
  P_q ≤ 0) frequencies per column-step, CAPE, the convective share of precipitation;
- from the snapshots inside the climatology window: the mean total and fourth-block fraction
  per level for σ ≥ 0.6, and "low strat" = the largest mean fourth-block fraction at σ ≥ 0.7
  (for the convective split, the fourth block is the stratiform-only fraction).
"""

import argparse
import json
import pathlib

import numpy as np


def load(run, band):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlat, nlon = len(lat), len(meta["grid"]["lon"])
    sigma = np.asarray(meta["sigma"], float)
    w = np.cos(np.radians(lat))
    sel = np.abs(lat) <= band

    def zm(a):
        z = a.reshape(-1, nlat, nlon).mean(axis=-1)
        return (z[..., sel] * w[sel]).sum(axis=-1) / w[sel].sum()

    def mp(n):
        f = run / "climatology" / f"mean_{n}.npy"
        return float(zm(np.load(f).astype(float))[0]) if f.exists() else float("nan")

    v = {n: mp(n) for n in ("asr", "asr_cs", "olr", "olr_cs", "asr_nc", "olr_nc", "clt", "cc_conv",
                            "cwp_conv", "precip", "precip_conv", "conv_active", "conv_skipped", "cape",
                            "ts", "sic")}
    o = {
        "ts": v["ts"], "clt": v["clt"],
        "sw_all": v["asr"] - v["asr_cs"], "lw_all": v["olr_cs"] - v["olr"],
        "sw_nc": v["asr_nc"] - v["asr_cs"], "lw_nc": v["olr_cs"] - v["olr_nc"],
        "cc_conv": v["cc_conv"], "cwp_conv": v["cwp_conv"],
        "f_deep": v["conv_active"], "f_skip": v["conv_skipped"], "cape": v["cape"],
        "conv_share": v["precip_conv"] / v["precip"] if v["precip"] > 0 else float("nan"),
        "precip_mm": v["precip"] * 86400.0,
    }
    for p in ("all", "nc"):
        o[f"net_{p}"] = o[f"sw_{p}"] + o[f"lw_{p}"]
    for x in ("sw", "lw", "net"):
        o[f"{x}_rm"] = o[f"{x}_all"] - o[f"{x}_nc"]
    # snapshots in the climatology window
    cfg = meta["config"]
    t0 = cfg.get("climatology", {}).get("start", 0.0)
    dt = cfg["dt"]
    prof = {}
    for name in ("cl", "cl_split"):
        fs = [f for f in sorted((run / "fields").glob(f"{name}_*.npy"))
              if f.stem[len(name) + 1:].isdigit() and int(f.stem[len(name) + 1:]) * dt >= t0]
        if fs:
            prof[name] = np.mean([zm(np.load(f).astype(float)) for f in fs], axis=0)
            o[f"n_{name}"] = len(fs)
    lev = sigma >= 0.6
    if "cl_split" in prof:
        low = sigma >= 0.7
        o["low_strat"] = float(prof["cl_split"][low].max())
        o["low_strat_sigma"] = float(sigma[low][np.argmax(prof["cl_split"][low])])
        o["profile"] = {f"{s:.3f}": [float(prof["cl"][k]), float(prof["cl_split"][k])]
                        for k, s in enumerate(sigma) if lev[k]}
    return o


ROWS = [
    ("T_s [K]", "ts", "{:.2f}"), ("SW / LW / net CRE, all", ("sw_all", "lw_all", "net_all"), "{:+.1f}"),
    ("… extra call (nc)", ("sw_nc", "lw_nc", "net_nc"), "{:+.1f}"),
    ("… removed part", ("sw_rm", "lw_rm", "net_rm"), "{:+.1f}"), ("cover clt", "clt", "{:.3f}"),
    ("deep-convection frequency", "f_deep", "{:.3f}"), ("skipped frequency", "f_skip", "{:.3f}"),
    ("CAPE [J kg⁻¹]", "cape", "{:.0f}"), ("P [mm/day], convective share", ("precip_mm", "conv_share"), "{:.2f}"),
    ("c_c mean / cwp_conv mean [kg m⁻²]", ("cc_conv", "cwp_conv"), "{:.3f}"),
    ("low strat max fraction (σ)", ("low_strat", "low_strat_sigma"), "{:.3f}"),
]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="+")
    ap.add_argument("--band", type=float, default=20.0)
    ap.add_argument("--json")
    ap.add_argument("--fig", help="PNG: zonal SW and LW CRE, all-cloud and removed part, per run")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    res = {lab: load(r, a.band) for lab, r in zip(labels, a.runs)}
    print(f"\n|φ| ≤ {a.band:g}° | " + " | ".join(labels) + " |")
    print("|---" * (len(labels) + 1) + "|")
    for name, key, fmt in ROWS:
        keys = key if isinstance(key, tuple) else (key,)
        cells = []
        for lab in labels:
            r = res[lab]
            cells.append(" / ".join(fmt.format(r[k]) if k in r else "–" for k in keys))
        print(f"| {name} | " + " | ".join(cells) + " |")
    if any("profile" in r for r in res.values()):
        print("\nmean fraction per level, total / fourth block:")
        sig = next(r["profile"] for r in res.values() if "profile" in r).keys()
        print("| σ | " + " | ".join(labels) + " |")
        print("|---" * (len(labels) + 1) + "|")
        for s in sig:
            print(f"| {s} | " + " | ".join(
                " / ".join(f"{x:.3f}" for x in res[lab]["profile"][s]) if "profile" in res[lab] else "–"
                for lab in labels) + " |")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(res, indent=1))
    if a.fig:
        figure(a.runs, labels, a.fig)


def figure(runs, labels, path):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(2, 2, figsize=(11, 7), sharex=True)
    for run, lab in zip(runs, labels):
        run = pathlib.Path(run)
        meta = json.loads((run / "meta.json").read_text())
        lat = np.degrees(np.asarray(meta["grid"]["lat"]))
        nlat, nlon = len(lat), len(meta["grid"]["lon"])
        z = {n: np.load(run / "climatology" / f"mean_{n}.npy").astype(float).reshape(nlat, nlon).mean(axis=1)
             for n in ("asr", "asr_cs", "olr", "olr_cs", "asr_nc", "olr_nc")}
        sw, lw = z["asr"] - z["asr_cs"], z["olr_cs"] - z["olr"]
        line = ax[0, 0].plot(lat, sw, label=lab)[0]
        ax[0, 1].plot(lat, lw, color=line.get_color())
        ax[1, 0].plot(lat, sw - (z["asr_nc"] - z["asr_cs"]), color=line.get_color())
        ax[1, 1].plot(lat, lw - (z["olr_cs"] - z["olr_nc"]), color=line.get_color())
    for a_, t in zip(ax.flat, ("SW CRE, all cloud", "LW CRE, all cloud", "SW CRE of the removed cloud",
                               "LW CRE of the removed cloud")):
        a_.set_title(t)
        a_.axhline(0, color="0.6", lw=0.5)
        a_.grid(alpha=0.3)
    for a_ in ax[1]:
        a_.set_xlabel("latitude [°]")
    for a_ in ax[:, 0]:
        a_.set_ylabel("W m⁻²")
    ax[0, 0].legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


if __name__ == "__main__":
    main()
