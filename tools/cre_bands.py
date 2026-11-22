"""Cloud radiative effect by latitude band against CERES EBAF (P4 question 13, option (b)).

    python tools/cre_bands.py out/<run> [...] [--labels a b ...] [--fig <png>] [--title T]
        [--json <out.json>]

For each run (online climatology `mean_{asr,asr_cs,olr,olr_cs,clt}.npy`, annual mean) and for
CERES EBAF Ed4.1 (2001–2020 climatology, `reference/derived/t<N>/*_ceres.npy`, written by
tools/fetch_reference.py at the run's truncation), area-weighted means of

    SW CRE = asr − asr_cs,   LW CRE = olr_cs − olr,   net = SW + LW   [W m⁻²],  cover clt

over the bands 20°S–20°N, 20–40°, 40–65° and 65–90° in each hemisphere, and globally.
CERES uses its "clear-sky for total region" fluxes (Ed4.1 `*_clr_t_*`), the definition
comparable with a model's clear-sky radiation call; CERES cover is the daytime-and-
nighttime cloud area fraction. Prints a markdown table; `--fig` draws the zonal means.
"""

import argparse
import json
import pathlib

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
BANDS = [("90–65°S", -90, -65), ("65–40°S", -65, -40), ("40–20°S", -40, -20), ("20°S–20°N", -20, 20),
         ("20–40°N", 20, 40), ("40–65°N", 40, 65), ("65–90°N", 65, 90), ("global", -90, 90)]
QUANT = [("sw", "SW CRE"), ("lw", "LW CRE"), ("net", "net CRE"), ("clt", "cover")]


def zonal_run(run):
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlon = len(meta["grid"]["lon"])
    c = run / "climatology"
    z = {n: np.load(c / f"mean_{n}.npy").astype(float).reshape(len(lat), nlon).mean(axis=1)
         for n in ["asr", "asr_cs", "olr", "olr_cs", "clt"]}
    sw, lw = z["asr"] - z["asr_cs"], z["olr_cs"] - z["olr"]
    return lat, {"sw": sw, "lw": lw, "net": sw + lw, "clt": z["clt"]}, meta["config"].get("truncation")


def zonal_ceres(trunc, nlat):
    d = ROOT / "reference" / "derived" / f"t{trunc}"
    z = {k: np.load(d / f"{k}_ceres.npy").astype(float).mean(axis=0).mean(axis=-1)
         for k in ["cre_sw", "cre_lw", "cre_net", "clt"]}
    assert len(z["clt"]) == nlat
    return {"sw": z["cre_sw"], "lw": z["cre_lw"], "net": z["cre_net"], "clt": z["clt"]}


def bands(lat, z):
    w = np.cos(np.radians(lat))
    out = {}
    for name, lo, hi in BANDS:
        m = (lat >= lo) & (lat <= hi)
        out[name] = {q: float((z[q][m] * w[m]).sum() / w[m].sum()) for q, _ in QUANT}
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--fig")
    ap.add_argument("--title", default="")
    ap.add_argument("--json")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    zon, res = {}, {}
    lat0 = trunc0 = None
    for lab, r in zip(labels, a.runs):
        lat, z, trunc = zonal_run(pathlib.Path(r))
        if lat0 is None:
            lat0, trunc0 = lat, trunc
        zon[lab] = (lat, z)
        res[lab] = bands(lat, z)
    cz = zonal_ceres(trunc0, len(lat0))
    res = {"CERES EBAF Ed4.1": bands(lat0, cz), **res}
    for q, qn in QUANT:
        unit = "" if q == "clt" else " [W m⁻²]"
        print(f"\n**{qn}{unit}**\n")
        print("| | " + " | ".join(b[0] for b in BANDS) + " |")
        print("|---|" + "---|" * len(BANDS))
        fmt = "{:.2f}" if q == "clt" else "{:+.1f}"
        for lab, b in res.items():
            print(f"| {lab} | " + " | ".join(fmt.format(b[n[0]][q]) for n in BANDS) + " |")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(res, indent=1))
    if a.fig:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        series = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"]
        fig, axs = plt.subplots(2, 2, figsize=(10, 6.5), sharex=True, constrained_layout=True)
        for ax, (q, qn) in zip(axs.flat, QUANT):
            ax.plot(lat0, cz[q], color="#333333", lw=2.0, label="CERES EBAF Ed4.1 (2001–2020)")
            for i, (lab, (lat, z)) in enumerate(zon.items()):
                ax.plot(lat, z[q], color=series[i], lw=2.0, label=lab)
            if q != "clt":
                ax.axhline(0.0, color="#999999", lw=0.8)
            for e in (-65, -40, -20, 20, 40, 65):
                ax.axvline(e, color="#dddddd", lw=0.8, zorder=0)
            ax.set_title(qn + ("" if q == "clt" else " [W m⁻²]"), loc="left", fontsize=11)
            ax.grid(False)
            for s in ("top", "right"):
                ax.spines[s].set_visible(False)
        for ax in axs[1]:
            ax.set_xlabel("latitude [°]")
            ax.set_xlim(-90, 90)
            ax.set_xticks(range(-90, 91, 30))
        axs[0, 0].legend(frameon=False, fontsize=9)
        if a.title:
            fig.suptitle(a.title)
        fig.savefig(a.fig, dpi=110)


if __name__ == "__main__":
    main()
