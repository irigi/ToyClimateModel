#!/usr/bin/env python3
"""Grid-mean liquid and ice water paths of model runs against ESA Cloud_cci AVHRR-PM v3
(P4 question 13, owner decision after step 4 of 2026-11-24, step 4b item b).

    .venv/bin/python -I tools/water_path.py <run> [<run> ...] [--labels a b ...]
        [--obs reference/raw/cloud_cci] [--fig out.png] [--json out.json]

Model: from the snapshots `t`, `cl`, `cldwp` (in-cloud condensate path per layer, liquid +
ice) inside the climatology window, grid-mean LWP = Σ_k cl·cldwp·(1 − f_ice(T_k)) and IWP =
Σ_k cl·cldwp·f_ice(T_k), with the model's f_ice(T) = clamp((273.15 − T)/20, 0, 1) applied to
the snapshot temperature (the scheme uses the temperature after the physics of the same
step, so the phase split is approximate; the sum is checked against the `clwvi` map).

Observations: Cloud_cci AVHRR-PM v3 in obs4MIPs format (Stengel et al. 2020; DOI
10.5285/919157930d7447caac6d42e84e377289; files `clwvi_mon_*` = liquid + ice and `clivi_mon_*`
= ice, all-sky grid means, daytime only, from optical thickness and effective radius).
LWP = clwvi − clivi. The technical notes give liquid CWP bias −3.2 g m⁻² (precision 26.4)
and in-cloud ice CWP bias −365 g m⁻² (precision 1017) against active A-Train sensors, so
the ice comparison is weak. All years present in the directory are averaged.

Bands (area-weighted): 10°S–10°N, 20°S–20°N, 15–35°N, 15–35°S, 45–65°S, global.
"""
import argparse
import glob
import json
import pathlib

import numpy as np

BANDS = [("10S-10N", -10, 10), ("20S-20N", -20, 20), ("15-35N", 15, 35), ("15-35S", -35, -15),
         ("45-65S", -65, -45), ("global", -90, 90)]


def band_mean(lat, z, lo, hi):
    w = np.cos(np.radians(lat))
    s = (lat >= lo) & (lat <= hi)
    return float((z[s] * w[s]).sum() / w[s].sum())


def model(run):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    cfg = meta["config"]
    clim = cfg.get("climatology", {}) if isinstance(cfg, dict) else {}
    def seconds(v, unit_s=86400.0):
        # the stored config gives durations in seconds; strings as in the case files
        if isinstance(v, (int, float)):
            return float(v)
        v = str(v)
        for suf, f in (("day", 86400.0), ("min", 60.0), ("h", 3600.0), ("s", 1.0)):
            if v.endswith(suf):
                return float(v[: -len(suf)]) * f
        return float(v) * unit_s

    start_s = seconds(clim.get("start", 0.0))
    step_s = seconds(cfg["dt"], 1.0)
    files = sorted(glob.glob(str(run / "fields" / "cldwp_*.npy")))
    lwp, iwp = [], []
    for f in files:
        step = int(pathlib.Path(f).stem.split("_")[-1])
        if step * step_s < start_s:
            continue
        w = np.load(f).astype(float)
        c = np.load(f.replace("cldwp_", "cl_")).astype(float)
        t = np.load(f.replace("cldwp_", "t_")).astype(float)
        fi = np.clip((273.15 - t) / 20.0, 0.0, 1.0)
        lwp.append((c * w * (1 - fi)).sum(axis=0))
        iwp.append((c * w * fi).sum(axis=0))
    n = len(lwp)
    lwp = np.mean(lwp, axis=0).mean(axis=-1)
    iwp = np.mean(iwp, axis=0).mean(axis=-1)
    out = {"lat": lat, "lwp": lwp, "iwp": iwp, "n": n}
    cw = run / "climatology" / "mean_clwvi.npy"
    if cw.exists():
        out["clwvi"] = np.load(cw).astype(float).reshape(len(lat), -1).mean(axis=-1)
    return out


def obs(d):
    import netCDF4

    d = pathlib.Path(d)
    zs = {}
    for v in ("clwvi", "clivi"):
        acc, n = None, 0
        for f in sorted(d.glob(f"{v}_mon_*.nc")):
            with netCDF4.Dataset(f) as nc:
                a = nc.variables[v][:]  # (time, lat, lon), masked
                lat = np.asarray(nc.variables["lat"][:], float)
                zm = np.ma.mean(a, axis=-1)  # zonal mean of available cells
                s = np.ma.sum(zm, axis=0).filled(0.0)
                k = np.ma.count(zm, axis=0)
                acc = s if acc is None else acc + s
                n = k if isinstance(n, int) else n + k
        zs[v] = np.where(n > 0, acc / np.maximum(n, 1), np.nan)
    return {"lat": lat, "lwp": zs["clwvi"] - zs["clivi"], "iwp": zs["clivi"], "cwp": zs["clwvi"]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--obs", default="reference/raw/cloud_cci")
    ap.add_argument("--fig")
    ap.add_argument("--json")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    o = obs(a.obs)
    ok = np.isfinite(o["lwp"])
    rows = {}
    print(f"{'g m-2':24s}" + "".join(f"{b[0]:>10s}" for b in BANDS))
    for key in ("lwp", "iwp"):
        r = [band_mean(o["lat"][ok], o[key][ok], lo, hi) * 1e3 for _, lo, hi in BANDS]
        rows[f"Cloud_cci {key}"] = r
        print(f"{'Cloud_cci ' + key:24s}" + "".join(f"{x:10.1f}" for x in r))
    ms = []
    for run, lab in zip(a.runs, labels):
        m = model(run)
        ms.append((lab, m))
        for key in ("lwp", "iwp"):
            r = [band_mean(m["lat"], m[key], lo, hi) * 1e3 for _, lo, hi in BANDS]
            rows[f"{lab} {key}"] = r
            print(f"{lab + ' ' + key:24s}" + "".join(f"{x:10.1f}" for x in r))
        for key in ("lwp", "iwp"):
            r = [band_mean(m["lat"], m[key], lo, hi) / band_mean(o["lat"][ok], o[key][ok], lo, hi)
                 for _, lo, hi in BANDS]
            rows[f"{lab} {key} ratio"] = r
            print(f"{lab + ' ' + key + ' / obs':24s}" + "".join(f"{x:10.2f}" for x in r))
        if "clwvi" in m:
            tot = (m["lwp"] + m["iwp"])
            chk = band_mean(m["lat"], tot, -90, 90) / band_mean(m["lat"], m["clwvi"], -90, 90)
            print(f"  check: snapshot LWP+IWP / clwvi map, global = {chk:.3f} ({m['n']} snapshots)")
            rows[f"{lab} check"] = chk
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps({"bands": [b[0] for b in BANDS], "rows": rows},
                                                   indent=1))
    if a.fig:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        fig, ax = plt.subplots(1, 2, figsize=(11, 4), sharex=True)
        for i, key in enumerate(("lwp", "iwp")):
            ax[i].plot(o["lat"], o[key] * 1e3, "k", lw=2, label="Cloud_cci AVHRR-PM v3 2003-2012")
            for lab, m in ms:
                ax[i].plot(m["lat"], m[key] * 1e3, label=lab)
            ax[i].set_title({"lwp": "grid-mean liquid water path", "iwp": "grid-mean ice water path"}[key])
            ax[i].set_xlabel("latitude")
            ax[i].set_ylabel("g m$^{-2}$")
            ax[i].set_xlim(-90, 90)
            ax[i].grid(alpha=0.3)
        ax[0].legend(fontsize=8)
        fig.tight_layout()
        fig.savefig(a.fig, dpi=110)


if __name__ == "__main__":
    main()
