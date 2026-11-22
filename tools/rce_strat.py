"""Stratospheric temperature change under 2×CO₂ in single-column RCE: toyclim SCM (semi-gray,
cases/bench/p3_rce/rce_{400,800}.toml) with a calibration's longwave knobs, against the RRTMG
RCE of tools/rce_rrtmg.py (P4 question 11, check 3).

    python tools/rce_strat.py <calibration.json | default> [...] [--rrtmg out/p4/q11/rce/rrtmg.json]
        [--out-dir out/p4/q11/rce]

For each knob set the two RCE cases are run with the longwave knobs of the calibration
(lw_rest, lw_window, lw_n, lw_n_co2, lw_rest_strong, lw_strong_share, lw_co2_curvature,
lw_co2_band) written into `[radiation]`; "default" runs the cases unchanged. Prints T_s at
400 and 800 ppm and ΔT at the stratospheric levels next to RRTMG's.
"""

import argparse
import csv
import json
import pathlib
import subprocess

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "toyclim"
LW = ["lw_rest", "lw_window", "lw_n", "lw_n_co2", "lw_rest_strong", "lw_strong_share", "lw_co2_curvature",
      "lw_co2_band"]


def profile(d):
    rows = list(csv.DictReader(open(d / "profiles.csv")))
    p = np.array([float(r["p_pa"]) for r in rows]) / 100.0
    t = np.array([float(r["t"]) for r in rows])
    last = list(csv.DictReader(open(d / "scm.csv")))[-1]
    ts_key = next(k for k in last if k.lower().startswith("ts") or k.lower() == "t_s")
    return p, t, float(last[ts_key])


def run_pair(tag, knobs, out_dir):
    res = {}
    for c in (400, 800):
        src = (ROOT / f"cases/bench/p3_rce/rce_{c}.toml").read_text()
        if knobs:
            lines = [f"{k} = {json.dumps(v) if isinstance(v, list) else repr(float(v))}"
                     for k, v in knobs.items() if k in LW]
            src = src.replace('[radiation]\nscheme = "semigray"\n',
                              '[radiation]\nscheme = "semigray"\n' + "\n".join(lines) + "\n")
            assert lines[0] in src
        cfg = out_dir / f"{tag}_{c}.toml"
        cfg.write_text(src)
        d = out_dir / f"{tag}_{c}"
        subprocess.run([str(BIN), "scm", "--config", str(cfg), "--out", str(d)], check=True,
                       capture_output=True, env={"RUST_LOG": "error"})
        res[c] = profile(d)
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("calibrations", nargs="+")
    ap.add_argument("--rrtmg", default=str(ROOT / "out/p4/q11/rce/rrtmg.json"))
    ap.add_argument("--out-dir", default=str(ROOT / "out/p4/q11/rce"))
    a = ap.parse_args()
    out_dir = pathlib.Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    rr = json.loads(pathlib.Path(a.rrtmg).read_text())
    rp = np.array(rr[0]["p_hpa"])
    rdt = np.array(rr[-1]["t"]) - np.array(rr[0]["t"])
    levels = [2.4, 7.2, 13.8, 23.7, 37.0, 52.9, 69.9, 100.4]
    summary = {"rrtmg": {"ts": [rr[0]["ts"], rr[-1]["ts"]],
                         "dT": {f"{p:g}": float(np.interp(np.log(p), np.log(rp), rdt)) for p in levels}}}
    for cal in a.calibrations:
        if cal == "default":
            tag, knobs = "default", None
        else:
            path = pathlib.Path(cal)
            tag = path.parent.name if path.name == "calibration.json" else path.stem
            knobs = json.loads(path.read_text())["knobs"]
            band = knobs.get("lw_co2_band")
            if band is not None and len(band) == 3:  # fitted before c_win existed: c_win = c_ref
                knobs["lw_co2_band"] = list(band) + [band[1]]
        r = run_pair(tag, knobs, out_dir)
        (p, t4, ts4), (_, t8, ts8) = r[400], r[800]
        summary[tag] = {"ts": [ts4, ts8], "t400": {f"{x:g}": float(np.interp(np.log(x), np.log(p), t4)) for x in levels},
                        "dT": {f"{x:g}": float(np.interp(np.log(x), np.log(p), t8 - t4)) for x in levels}}
    print("ΔT (800 − 400 ppm) [K] at p [hPa]; T_s 400 → 800 ppm")
    print(f"{'':14s}" + "".join(f"{p:>7g}" for p in levels) + "   T_s")
    for tag, s in summary.items():
        print(f"{tag:14s}" + "".join(f"{s['dT'][f'{p:g}']:+7.1f}" for p in levels)
              + f"   {s['ts'][0]:.2f} → {s['ts'][1]:.2f} ({s['ts'][1] - s['ts'][0]:+.2f})")
    (out_dir / "strat_summary.json").write_text(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
