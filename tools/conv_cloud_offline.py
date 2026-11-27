"""Offline test of a time-averaged convective cloud (P4 question 13, option (a)).

    python tools/conv_cloud_offline.py out/<run400> out/<run800> [--hours 3 6 24] [--out <json>]

Each run must write `precip_conv` every step (cases/bench/p4_aquaplanet/snapseries_t21_*).
For every column the convective precipitation P [mm/day] is averaged over the last N hours
(trailing box mean, and an exponential mean with the same time constant), and the
diagnostic convective cover of clouds.rs, c_c = min(0.8, 0.125 ln(1 + P)), is applied to
the instantaneous P and to each mean. Prints the area-weighted mean c_c over 0–15° and
0–30° for both runs, the mean P and the fraction of column-steps with c_c > 0. The first
24 h are skipped so that every mean is fully formed.
"""

import argparse
import json
import pathlib

import numpy as np

B, CMAX = 0.125, 0.8


def series(run):
    files = sorted((run / "fields").glob("precip_conv_*.npy"), key=lambda p: int(p.stem.split("_")[-1]))
    return np.stack([np.load(f).astype(np.float64) for f in files]) * 86400.0  # mm/day


def cover(p):
    return np.minimum(CMAX, B * np.log1p(np.maximum(p, 0.0)))


def box_mean(p, n):
    c = np.cumsum(p, axis=0)
    out = np.empty_like(p)
    out[:n] = c[:n] / np.arange(1, n + 1)[:, None, None]
    out[n:] = (c[n:] - c[:-n]) / n
    return out


def exp_mean(p, n):
    a = 1.0 / n
    out = np.empty_like(p)
    m = p[0].copy()
    for i in range(len(p)):
        m += a * (p[i] - m)
        out[i] = m
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs=2)
    ap.add_argument("--hours", type=float, nargs="+", default=[3.0, 6.0, 24.0])
    ap.add_argument("--out")
    a = ap.parse_args()
    res = {}
    for r in a.runs:
        run = pathlib.Path(r)
        meta = json.loads((run / "meta.json").read_text())
        dt = meta["output_interval_s"]
        lat = np.degrees(np.asarray(meta["grid"]["lat"]))
        p = series(run)
        skip = int(round(86400.0 / dt))
        variants = {"instantaneous": p}
        for h in a.hours:
            n = max(1, int(round(h * 3600.0 / dt)))
            variants[f"box {h:g} h"] = box_mean(p, n)
            variants[f"exp {h:g} h"] = exp_mean(p, n)
        out = {}
        for name, x in variants.items():
            x = x[skip:]
            c = cover(x)
            row = {}
            for band, lo, hi in [("0-15", 0, 15), ("0-30", 0, 30)]:
                sel = (np.abs(lat) >= lo) & (np.abs(lat) < hi)
                w = np.cos(np.radians(lat[sel]))[None, :, None] * np.ones_like(c[:, sel, :])
                w /= w.sum()
                row[band] = {
                    "cover": float((c[:, sel, :] * w).sum()),
                    "precip": float((x[:, sel, :] * w).sum()),
                    "frac_cloudy": float(((c[:, sel, :] > 0) * w).sum()),
                }
            out[name] = row
        res[r] = out
    names = list(res[a.runs[0]])
    for band in ["0-15", "0-30"]:
        print(f"\n{band}°: mean c_c 400 → 800 ppm (change), fraction with c_c > 0, mean P [mm/day]")
        for n in names:
            x, y = res[a.runs[0]][n][band], res[a.runs[1]][n][band]
            print(f"  {n:14s} {x['cover']:.3f} → {y['cover']:.3f} ({y['cover'] - x['cover']:+.3f}, "
                  f"{100 * (y['cover'] / x['cover'] - 1):+.0f} %)   cloudy {x['frac_cloudy']:.2f} → "
                  f"{y['frac_cloudy']:.2f}   P {x['precip']:.2f} → {y['precip']:.2f}")
    if a.out:
        pathlib.Path(a.out).write_text(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
