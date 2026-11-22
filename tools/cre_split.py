"""Convective and stratiform parts of the cloud radiative effect in a CO₂ pair (P4 question 13,
option (b), step 1.3).

    python tools/cre_split.py <run_lo> <run_hi> [--labels 400 800] [--json <out.json>]

Needs runs with `[clouds] cre_split = true` (climatology maps asr, asr_cs, olr, olr_cs,
asr_nc, olr_nc): the radiation makes one extra call with the convective cloud removed
(stratiform-only fraction, same in-cloud water), so per band

    all-cloud CRE  SW = asr − asr_cs,     LW = olr_cs − olr
    stratiform     SW = asr_nc − asr_cs,  LW = olr_cs − olr_nc
    convective     = all-cloud − stratiform-only

(the convective part includes the overlap with the stratiform cloud: under random overlap
the parts are not additive, and this convention gives the overlap to the convective part).
Prints the 20°S–20°N and global means for both runs, their change, and the share of the
change in net CRE carried by the convective part. Also the time means of `cc_conv` and
`cwp_conv` (accumulated only where convective cloud exists, so they are frequency-weighted).
"""

import argparse
import json
import pathlib

import numpy as np

BANDS = [("20°S–20°N", -20, 20), ("global", -90, 90)]


def load(run):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlon = len(meta["grid"]["lon"])

    def z(n):
        f = run / "climatology" / f"mean_{n}.npy"
        return np.load(f).astype(float).reshape(len(lat), nlon).mean(axis=1) if f.exists() else None

    m = {n: z(n) for n in ["asr", "asr_cs", "olr", "olr_cs", "asr_nc", "olr_nc", "clt", "cc_conv", "cwp_conv", "ts"]}
    q = {
        "all_sw": m["asr"] - m["asr_cs"], "all_lw": m["olr_cs"] - m["olr"],
        "strat_sw": m["asr_nc"] - m["asr_cs"], "strat_lw": m["olr_cs"] - m["olr_nc"],
    }
    q["conv_sw"] = q["all_sw"] - q["strat_sw"]
    q["conv_lw"] = q["all_lw"] - q["strat_lw"]
    for p in ("all", "strat", "conv"):
        q[f"{p}_net"] = q[f"{p}_sw"] + q[f"{p}_lw"]
    for n in ("clt", "cc_conv", "cwp_conv", "ts"):
        if m[n] is not None:
            q[n] = m[n]
    w = np.cos(np.radians(lat))
    out = {}
    for name, lo, hi in BANDS:
        s = (lat >= lo) & (lat <= hi)
        out[name] = {k: float((v[s] * w[s]).sum() / w[s].sum()) for k, v in q.items()}
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("lo")
    ap.add_argument("hi")
    ap.add_argument("--labels", nargs=2, default=["lo", "hi"])
    ap.add_argument("--json")
    a = ap.parse_args()
    lo, hi = load(a.lo), load(a.hi)
    res = {}
    for band, _, _ in BANDS:
        L, H = lo[band], hi[band]
        print(f"\n**{band}** (T_s {L['ts']:.2f} → {H['ts']:.2f} K)\n")
        print(f"| W m⁻² | {a.labels[0]} SW / LW / net | {a.labels[1]} SW / LW / net | change SW / LW / net |")
        print("|---|---|---|---|")
        for p, pn in (("all", "all cloud"), ("strat", "stratiform only"), ("conv", "convective part")):
            v = [(L[f"{p}_{x}"], H[f"{p}_{x}"]) for x in ("sw", "lw", "net")]
            print(f"| {pn} | " + " / ".join(f"{x[0]:+.1f}" for x in v) + " | "
                  + " / ".join(f"{x[1]:+.1f}" for x in v) + " | " + " / ".join(f"{x[1] - x[0]:+.2f}" for x in v) + " |")
        d_all = H["all_net"] - L["all_net"]
        d_conv = H["conv_net"] - L["conv_net"]
        share = d_conv / d_all if d_all != 0 else float("nan")
        print(f"\nshare of the net CRE change carried by the convective part: {share:.2f} "
              f"({d_conv:+.2f} of {d_all:+.2f} W m⁻²)")
        extra = ", ".join(f"{k} {L[k]:.3f} → {H[k]:.3f}" for k in ("clt", "cc_conv", "cwp_conv") if k in L)
        print(f"{extra}")
        res[band] = {"lo": L, "hi": H, "share_conv": share}
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
