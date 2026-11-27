"""Radiative feedback decomposition of a CO2 pair from climatology maps (P4 diagnostic).

    python tools/feedbacks.py <run_lo> <run_hi> [--forcing out/<run>/co2_forcing.json] [--name NAME]

Per latitude band and globally (area-weighted), with dT = global-mean T_s change:
  clear-sky LW  = -(d(olr_cs) + F)/dT     clear-sky SW = d(asr_cs)/dT
  (F per band from co2_forcing.json; without --forcing the LW term still contains the forcing)
  SW CRE        = d(asr - asr_cs)/dT     LW CRE       = d(olr_cs - olr)/dT
  (each a contribution to the GLOBAL feedback in W m-2 K-1: band sums add up to the global value)
  sum = all four; with the forcing F (global clear-sky instantaneous, semi-gray, from co2_forcing.json):
  implied equilibrium dT = F / -sum (sum dT = dN - F by construction, so this extrapolates
  the remaining imbalance dN at the same feedback).
CRE changes include cloud masking of the clear-sky changes (not a kernel decomposition).
"""
import argparse, json, pathlib
import numpy as np

ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
ap.add_argument("lo"); ap.add_argument("hi"); ap.add_argument("--forcing"); ap.add_argument("--name", default="")
a = ap.parse_args()

def load(r):
    r = pathlib.Path(r); m = json.loads((r / "meta.json").read_text())
    c = lambda n: np.load(r / "climatology" / f"mean_{n}.npy").astype(float).mean(-1)
    return np.degrees(np.asarray(m["grid"]["lat"])), {n: c(n) for n in ["ts", "olr", "olr_cs", "asr", "asr_cs"]}

lat, L = load(a.lo); _, H = load(a.hi)
w = np.cos(np.radians(lat)); w = w / w.sum()
g = lambda x: float((x * w).sum())
dT = g(H["ts"] - L["ts"])
terms = {"clear-sky LW": -(H["olr_cs"] - L["olr_cs"]), "clear-sky SW": H["asr_cs"] - L["asr_cs"],
         "SW CRE": (H["asr"] - H["asr_cs"]) - (L["asr"] - L["asr_cs"]),
         "LW CRE": (H["olr_cs"] - H["olr"]) - (L["olr_cs"] - L["olr"])}
bands = [(0, 30), (30, 60), (60, 90)]
F = None
if a.forcing:
    fj = json.loads(pathlib.Path(a.forcing).read_text())["semigray"]
    F = fj["forcing_toa_W_m2"]
    fb = fj["forcing_by_abs_lat"]
    flat = np.zeros_like(lat)
    for b0, b1 in bands:
        flat[(np.abs(lat) >= b0) & (np.abs(lat) < b1)] = fb[f"{b0}-{b1}"]
    terms["clear-sky LW"] = terms["clear-sky LW"] - flat
res = {"name": a.name, "lo": a.lo, "hi": a.hi, "dT_global_K": dT, "terms": {}}
print(f"{a.name}  dT = {dT:.2f} K   (contributions to global feedback, W m-2 K-1)")
print(f"{'term':14s} {'global':>8s} " + " ".join(f"{b0:>2d}-{b1:<2d}°" for b0, b1 in bands))
tot = np.zeros_like(lat)
for k, v in terms.items():
    tot = tot + v
    row = {"global": g(v) / dT}
    for b0, b1 in bands:
        s = (np.abs(lat) >= b0) & (np.abs(lat) < b1)
        row[f"{b0}-{b1}"] = float((v * w)[s].sum()) / dT
    res["terms"][k] = row
    print(f"{k:14s} {row['global']:8.3f} " + " ".join(f"{row[f'{b0}-{b1}']:6.3f}" for b0, b1 in bands))
lam = g(tot) / dT
print(f"{'sum':14s} {lam:8.3f}")
res["sum"] = lam
dN = g((H["asr"] - H["olr"]) - (L["asr"] - L["olr"]))
res["dN_W_m2"] = dN
if F is not None:
    res.update({"forcing_W_m2": F, "implied_dT_K": F / -lam})
    print(f"F = {F:.2f} W m-2: implied equilibrium dT = {F / -lam:.2f} K; remaining dN = {dN:.2f} W m-2")
print(json.dumps(res))
