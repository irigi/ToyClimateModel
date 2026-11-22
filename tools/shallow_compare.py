"""Compare runs that differ in the SBM shallow-convection variant (P4 question 13).

    python tools/shallow_compare.py out/<run> [...] [--fig <png>] [--labels a b ...]

From each run's online climatology: Hadley strength (max |ψ| within 35° at σ = 0.7, the
gate scorecard's definition, mean of both hemispheres, 10¹⁰ kg s⁻¹), zonal-mean
precipitation at the equator (mean of the two rows nearest it) and its maximum with
latitude (mm/day and W m⁻²), and over 0–15° the frequencies of deep convection
(`conv_active`), skipped columns (`conv_skipped`: CAPE > 0, P_q ≤ 0, nothing done) and
shallow convection (`conv_shallow`), mean CAPE and the convective share of the rain.
"""

import argparse
import json
import pathlib
import sys

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from scorecard import hadley, streamfunction  # noqa: E402

LV = 2.5e6


def metrics(run):
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    lat = np.asarray(meta["grid"]["lat"])
    latd = np.degrees(lat)
    nlon = len(meta["grid"]["lon"])
    sh = np.asarray(meta["config"]["sigma_half"])
    v, ps = np.load(c / "zonal_v.npy"), np.load(c / "zonal_ps.npy")
    psi = streamfunction(v, sh, lat, ps, meta["planet"]["radius"], meta["planet"]["gravity"])
    h = hadley(psi, sh, latd)

    def zm(name):
        f = c / f"mean_{name}.npy"
        return np.load(f).reshape(len(lat), nlon).mean(axis=1) if f.exists() else None

    p = zm("precip") * 86400.0
    eq = np.argsort(np.abs(latd))[:2]
    j = int(np.argmax(p))
    trop = np.abs(latd) < 15
    w = np.cos(lat[trop])

    def tm(name):
        x = zm(name)
        return None if x is None else float((x[trop] * w).sum() / w.sum())

    pc = zm("precip_conv")
    return {
        "hadley": 0.5 * (h["NH"]["max_psi_1e10_kg_s"] + h["SH"]["max_psi_1e10_kg_s"]),
        "hadley_lat": 0.5 * (h["NH"]["lat_of_max"] - h["SH"]["lat_of_max"]),
        "p_eq": float(p[eq].mean()), "p_max": float(p[j]), "p_max_lat": float(abs(latd[j])),
        "active": tm("conv_active"), "skipped": tm("conv_skipped"), "shallow": tm("conv_shallow"),
        "cape": tm("cape"),
        "conv_share": float(((pc * 86400.0)[trop] * w).sum() / (p[trop] * w).sum()) if pc is not None else None,
        "latd": latd.tolist(), "p_zonal": p.tolist(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--fig")
    ap.add_argument("--title", default="")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    res = {lab: metrics(pathlib.Path(r)) for lab, r in zip(labels, a.runs)}
    f = lambda x, s="{:.2f}": "—" if x is None else s.format(x)  # noqa: E731
    print("| run | Hadley [1e10 kg/s] | P equator [mm/d] (W m-2) | P max [mm/d] at lat | 0–15°: deep / skipped / shallow | CAPE | conv share |")
    print("|---|---|---|---|---|---|---|")
    for lab, m in res.items():
        print(f"| {lab} | {m['hadley']:.1f} | {m['p_eq']:.1f} ({m['p_eq'] * LV / 86400:.0f}) | "
              f"{m['p_max']:.1f} at {m['p_max_lat']:.0f}° | {f(m['active'])} / {f(m['skipped'])} / {f(m['shallow'])} | "
              f"{f(m['cape'], '{:.0f}')} | {f(m['conv_share'])} |")
    if a.fig:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        fig, ax = plt.subplots(figsize=(7, 4), constrained_layout=True)
        for lab, m in res.items():
            ax.plot(m["latd"], m["p_zonal"], label=f"{lab} (Hadley {m['hadley']:.1f})")
        ax.set_xlabel("latitude [°]")
        ax.set_ylabel("zonal-mean precipitation [mm/day]")
        ax.set_xlim(-60, 60)
        ax.set_title(a.title)
        ax.legend()
        fig.savefig(a.fig, dpi=100)


if __name__ == "__main__":
    main()
