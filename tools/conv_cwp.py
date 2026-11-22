"""Distribution of the convective cloud's in-cloud water path over convecting columns (P4 question
13, option (b), step 1.4).

    python tools/conv_cwp.py out/<run> [...] [--labels a b] [--lat 20]

Reads every-step snapshots of `cc_conv` (convective fraction c_c) and `cwp_conv` (its in-cloud
water path Σ min(β q_sat, r_max) Δp/g over the convective layers, kg m⁻²; both 0 where the
column has no convective cloud) and prints, over columns with c_c > 0 within ±lat, the
median and 90th percentile of the path (g m⁻²), of c_c, and the visible in-cloud optical depth
implied by the model's own optics (τ = k W with k = 84.5 m² kg⁻¹ for ice at r_e = 30 µm,
Ebert & Curry 1992, and 158.7 m² kg⁻¹ for liquid at 10 µm, Slingo 1989; the path is not split
by phase here, so τ is given as the range between the two).
"""

import argparse
import json
import pathlib

import numpy as np

K_ICE, K_LIQ = 84.5, 158.7


def collect(run, latmax):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlon = len(meta["grid"]["lon"])
    rows = np.abs(lat) <= latmax
    key = lambda p: int(p.stem.split("_")[-1])  # noqa: E731
    cc_files = sorted((run / "fields").glob("cc_conv_*.npy"), key=key)
    cw, cc = [], []
    n_all = 0
    for f in cc_files:
        g = run / "fields" / f.name.replace("cc_conv_", "cwp_conv_")
        c = np.load(f).astype(float).reshape(len(lat), nlon)[rows]
        w = np.load(g).astype(float).reshape(len(lat), nlon)[rows]
        n_all += c.size
        m = c > 0
        cc.append(c[m])
        cw.append(w[m])
    return np.concatenate(cc), np.concatenate(cw), n_all, len(cc_files)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--lat", type=float, default=20.0)
    ap.add_argument("--json")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    print(f"| run (|φ| ≤ {a.lat:g}°) | snapshots | share of column-steps with c_c > 0 | c_c median / p90 | "
          "in-cloud path median / p90 [g m⁻²] | visible τ median (ice … liquid) | τ p90 |")
    print("|---|---|---|---|---|---|---|")
    res = {}
    for lab, r in zip(labels, a.runs):
        cc, cw, n, ns = collect(r, a.lat)
        p50, p90 = np.percentile(cw, [50, 90])
        c50, c90 = np.percentile(cc, [50, 90])
        res[lab] = {"snapshots": ns, "share": len(cc) / n, "cc_p50": c50, "cc_p90": c90,
                    "cwp_p50_g": 1e3 * p50, "cwp_p90_g": 1e3 * p90}
        print(f"| {lab} | {ns} | {len(cc) / n:.3f} | {c50:.3f} / {c90:.3f} | {1e3 * p50:.0f} / {1e3 * p90:.0f} | "
              f"{K_ICE * p50:.0f} … {K_LIQ * p50:.0f} | {K_ICE * p90:.0f} … {K_LIQ * p90:.0f} |")
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
