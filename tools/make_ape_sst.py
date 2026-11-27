"""APE "Control" SST maps for the prescribed ocean, optionally with a uniform offset (Cess-type
fixed-SST feedback pair; P4 diagnostics, not a gate).

    python tools/make_ape_sst.py <run with meta.json at the target truncation> <out_dir> [--offsets 0 4]

APE Control (Neale & Hoskins 2000, Atmos. Sci. Lett. 1, 101-107): T_s = 27 (1 - sin^2(3 phi / 2)) degC for
|phi| < pi/3, 0 degC otherwise. Writes ape_control[_p<offset>k]_tos.npy (12 identical months, K, float32)
and ape_sic_zero.npy into out_dir, on the Gaussian grid of the given run.
"""
import argparse, json, pathlib
import numpy as np

ap = argparse.ArgumentParser(description=__doc__)
ap.add_argument("run"); ap.add_argument("out"); ap.add_argument("--offsets", type=float, nargs="+", default=[0.0, 4.0])
a = ap.parse_args()
m = json.loads((pathlib.Path(a.run) / "meta.json").read_text())
lat = np.asarray(m["grid"]["lat"]); nlon = len(m["grid"]["lon"])
c = np.where(np.abs(lat) < np.pi / 3, 27.0 * (1.0 - np.sin(1.5 * lat) ** 2), 0.0) + 273.15
out = pathlib.Path(a.out); out.mkdir(parents=True, exist_ok=True)
for d in a.offsets:
    f = np.broadcast_to((c + d)[None, :, None], (12, len(lat), nlon)).astype(np.float32)
    name = "ape_control_tos.npy" if d == 0 else f"ape_control_p{d:g}k_tos.npy"
    np.save(out / name, f); print(out / name, f.min(), f.max())
np.save(out / "ape_sic_zero.npy", np.zeros((12, len(lat), nlon), np.float32))
