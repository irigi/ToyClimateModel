"""Global clear-sky 2×CO₂ forcing on saved model states, semi-gray against RRTMG (P4
completion plan, step 2; SPEC.md §9.6).

    .rrtmg/bin/python -I tools/co2_forcing.py out/<run> [--lon-stride 4] [--co2 400 800]
        [--knobs-json <calibration.json> --tag <name>]

`out/<run>` must hold 3-D snapshots `fields/t_*.npy`, `q_*.npy` and 2-D `ps_*.npy`, `ts_*.npy`
(e.g. `cases/bench/p4_aquaplanet/snap_t21_400.toml`). Every snapshot after step 0 is used;
columns are taken at every `lon-stride`-th longitude (all latitudes) and area-weighted.

The forcing is the instantaneous clear-sky top-of-atmosphere longwave forcing,
OLR(co2_lo) − OLR(co2_hi), with temperatures, humidity and surface unchanged and no
stratospheric adjustment, evaluated with the same semi-gray code as the model
(`toyclim radiation`, knobs from the run's `[radiation]` block) and with RRTMG (climlab, as in
`tools/calibrate_radiation.py`: other gases at climlab's defaults, no ozone). Also reported:
the change of surface downward longwave, and the forcing by latitude band.

Writes `<run>/co2_forcing.json` and prints a summary.
"""

import argparse
import json
import pathlib
import subprocess
import sys
import tempfile

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from calibrate_radiation import BIN, column, rrtmg  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--lon-stride", type=int, default=4)
    ap.add_argument("--co2", type=float, nargs=2, default=[400.0, 800.0])
    ap.add_argument("--knobs-json", help="override the longwave/shortwave knobs with the 'knobs' of a "
                    "calibration.json (candidate refits); the result is then written as co2_forcing_<tag>.json")
    ap.add_argument("--tag", default="candidate")
    ap.add_argument("--semigray-only", action="store_true", help="skip RRTMG (compare with an earlier co2_forcing.json)")
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    half = np.asarray(cfg["sigma_half"])
    lat = np.asarray(meta["grid"]["lat"])
    nlon = len(meta["grid"]["lon"])
    knobs = {k: v for k, v in ((cfg.get("physics") or {}).get("radiation") or {}).items() if k != "scheme"}
    if a.knobs_json:
        knobs.update(json.loads(pathlib.Path(a.knobs_json).read_text())["knobs"])
    steps = sorted(int(p.stem.split("_")[1]) for p in (run / "fields").glob("t_*.npy"))
    steps = [s for s in steps if s > 0] or steps
    if not steps:
        sys.exit(f"{run}/fields has no t_*.npy snapshots")
    lo, hi = a.co2
    cols, wts, lats = [], [], []
    for s in steps:
        f = lambda n: np.load(run / "fields" / f"{n}_{s:06d}.npy")  # noqa: E731
        t, q, ps, ts = f("t"), f("q"), f("ps"), f("ts")
        for j in range(len(lat)):
            for i in range(0, nlon, a.lon_stride):
                ph = half * float(ps[j, i])
                cols.append((ph, t[:, j, i].astype(float), np.maximum(q[:, j, i].astype(float), 0.0),
                             float(ts[j, i])))
                wts.append(np.cos(lat[j]))
                lats.append(np.degrees(lat[j]))
    wts, lats = np.asarray(wts), np.asarray(lats)
    wts /= wts.sum()
    print(f"{len(steps)} snapshots, {len(cols)} columns", flush=True)

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="toyclim_f2x_"))
    kf = tmp / "knobs.toml"
    kf.write_text("".join(f"{k} = {json.dumps(v)}\n" for k, v in knobs.items()))

    def semigray(co2):
        cf = tmp / "cols.json"
        cf.write_text(json.dumps([column(ph, t, q, ts, co2) for ph, t, q, ts in cols]))
        r = subprocess.run([str(BIN), "radiation", "--knobs", str(kf), "--columns", str(cf)],
                           capture_output=True, text=True, check=True, env={"RUST_LOG": "error"})
        out = json.loads(r.stdout)
        return np.array([x["olr"] for x in out]), np.array([x["lw_down_sfc"] for x in out])

    def ref(co2):
        o, d = [], []
        for n, (ph, t, q, ts) in enumerate(cols):
            x = rrtmg(ph, t, q, ts, co2)
            o.append(x["olr"])
            d.append(x["lw_down_sfc"])
            if n % 1000 == 999:
                print(f"  RRTMG {co2:g} ppm: {n + 1}/{len(cols)}", flush=True)
        return np.array(o), np.array(d)

    res = {}
    for name, fn in [("semigray", semigray)] + ([] if a.semigray_only else [("rrtmg", ref)]):
        o1, d1 = fn(lo)
        o2, d2 = fn(hi)
        F, D = o1 - o2, d2 - d1
        bands = {}
        for b0, b1 in [(0, 30), (30, 60), (60, 90)]:
            sel = (np.abs(lats) >= b0) & (np.abs(lats) < b1)
            bands[f"{b0}-{b1}"] = float((F[sel] * wts[sel]).sum() / wts[sel].sum())
        res[name] = {"forcing_toa_W_m2": float((F * wts).sum()), "olr_lo_W_m2": float((o1 * wts).sum()),
                     "lw_down_sfc_change_W_m2": float((D * wts).sum()), "forcing_by_abs_lat": bands,
                     "forcing_min": float(F.min()), "forcing_max": float(F.max())}
        print(f"{name:8s}: F = {res[name]['forcing_toa_W_m2']:.3f} W/m² (bands {bands}); "
              f"OLR({lo:g}) = {res[name]['olr_lo_W_m2']:.1f}; ΔLW↓sfc = {res[name]['lw_down_sfc_change_W_m2']:.3f}",
              flush=True)
    if "rrtmg" not in res:
        prev = json.loads((run / "co2_forcing.json").read_text()) if (run / "co2_forcing.json").exists() else None
        if prev and prev.get("columns") == len(cols):
            res["rrtmg"] = prev["rrtmg"]
            res["rrtmg_from"] = str(run / "co2_forcing.json")
    if "rrtmg" in res:
        res["ratio_semigray_to_rrtmg"] = res["semigray"]["forcing_toa_W_m2"] / res["rrtmg"]["forcing_toa_W_m2"]
    res.update({"run": str(run), "snapshots": steps, "columns": len(cols), "co2_ppm": [lo, hi],
                "knobs": knobs})
    dest = run / (f"co2_forcing_{a.tag}.json" if a.knobs_json else "co2_forcing.json")
    dest.write_text(json.dumps(res, indent=1))
    print(f"ratio semi-gray / RRTMG = {res.get('ratio_semigray_to_rrtmg', float('nan')):.3f}; wrote {dest}")


if __name__ == "__main__":
    main()
