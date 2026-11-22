"""Columns of saved model states with RRTMG clear-sky longwave at two CO₂ amounts, for the
radiation fit (`tools/calibrate_radiation.py --state-cols`; P4 question 11).

    .rrtmg/bin/python -I tools/state_columns.py out/<run> --out <file.npz>
        [--lon-stride 8] [--snapshots 2] [--co2 400 800] [--strat-from out/<other> --strat-sigma 0.1]

Columns as in `tools/co2_forcing.py` (every `lon-stride`-th longitude, all latitudes, the
last `snapshots` 3-D snapshots), optionally with the temperature above σ = `strat-sigma`
replaced by the time mean of another run (hybrid states: the stratosphere of one climate on
the troposphere of the other). Saves half-level pressures, T, q, T_s, area weights,
latitudes and the RRTMG OLR and surface LW↓ at each CO₂ amount.
"""

import argparse
import json
import pathlib
import sys

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from calibrate_radiation import rrtmg  # noqa: E402


def steps_of(run):
    s = sorted(int(p.stem.split("_")[1]) for p in (run / "fields").glob("t_*.npy"))
    return [x for x in s if x > 0] or s


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--out", required=True)
    ap.add_argument("--lon-stride", type=int, default=8)
    ap.add_argument("--snapshots", type=int, default=2)
    ap.add_argument("--co2", type=float, nargs=2, default=[400.0, 800.0])
    ap.add_argument("--strat-from")
    ap.add_argument("--strat-sigma", type=float, default=0.1)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    half = np.asarray(meta["config"]["sigma_half"])
    lat = np.asarray(meta["grid"]["lat"])
    nlon = len(meta["grid"]["lon"])
    strat = None
    if a.strat_from:
        o = pathlib.Path(a.strat_from)
        strat = np.mean([np.load(o / "fields" / f"t_{s:06d}.npy") for s in steps_of(o)], axis=0)
        top = 0.5 * (half[1:] + half[:-1]) < a.strat_sigma
    ph, tt, qq, tss, w, la = [], [], [], [], [], []
    for s in steps_of(run)[-a.snapshots:]:
        f = lambda n: np.load(run / "fields" / f"{n}_{s:06d}.npy")  # noqa: E731
        t, q, ps, ts = f("t").astype(float), f("q").astype(float), f("ps"), f("ts")
        if strat is not None:
            t[top] = strat[top]
        for j in range(len(lat)):
            for i in range(0, nlon, a.lon_stride):
                ph.append(half * float(ps[j, i]))
                tt.append(t[:, j, i])
                qq.append(np.maximum(q[:, j, i], 0.0))
                tss.append(float(ts[j, i]))
                w.append(np.cos(lat[j]))
                la.append(np.degrees(lat[j]))
    print(f"{len(ph)} columns", flush=True)
    res = {}
    for c in a.co2:
        o, d = [], []
        for n in range(len(ph)):
            x = rrtmg(ph[n], tt[n], qq[n], tss[n], c)
            o.append(x["olr"])
            d.append(x["lw_down_sfc"])
        res[c] = (np.array(o), np.array(d))
        print(f"RRTMG {c:g} ppm done", flush=True)
    w = np.asarray(w)
    lo, hi = a.co2
    np.savez(a.out, ph=np.array(ph), t=np.array(tt), q=np.array(qq), ts=np.array(tss), w=w / w.sum(),
             lat=np.array(la), co2=np.array(a.co2), olr_lo=res[lo][0], olr_hi=res[hi][0],
             dn_lo=res[lo][1], dn_hi=res[hi][1], run=str(run), strat_from=str(a.strat_from))
    F = ((res[lo][0] - res[hi][0]) * w).sum() / w.sum()
    print(f"RRTMG forcing {F:.3f} W/m²; wrote {a.out}")


if __name__ == "__main__":
    main()
