#!/usr/bin/env python3
"""Gate scorecards (SPEC.md §9.5.2).

    python tools/scorecard.py out/<run> --suite aquaplanet [--milestone P1] [--no-write]

Computes the metrics of a gate from a run's online climatology (window from the
climatology start to the end of the run) and diagnostics.csv, prints a pass/fail table and
writes docs/scorecards/<milestone>_<suite>_<date>.json.

Metrics every world suite includes:
- global means: T_s, precipitation, evaporation, precipitable water, planetary albedo, OLR,
  net top-of-atmosphere and net surface flux;
- energy closure of the atmosphere: |d(moist total energy)/dt − (net TOA − net surface)|
  over the window, from the time-integrated fluxes accumulated every step;
- water closure: |dW/dt − (E − P)|;
- grid-point storm index G: the largest ratio, over cells whose orography is within ±300 m
  of their neighbours, of a cell's time-mean precipitation to the mean of its neighbours
  (8; 5 at the first and last latitude rows);
- tracer fixer rate: water moved by the negative-humidity fixer per day as a fraction of
  the global vapour mass (SPEC.md §9.8 trigger: > 1 %/day).

Suite `aquaplanet` (SPEC.md §9.6 P1, item 5) adds:
- lowest-level zonal-mean wind: easterlies in the tropics (mean over |φ| < 15° and every
  latitude within ±10° negative), westerlies at every latitude of 35–60° in both
  hemispheres, peak surface westerly per hemisphere 5–12 m/s;
- global-mean precipitation = evaporation within 1 %;
- energy closure of the atmosphere within 0.5 W m⁻²;
- G ≤ 2.0;
- recorded: zonal-mean precipitation, Hadley cell strength and edge, jets.
"""
import argparse
import csv
import datetime
import json
import pathlib
import subprocess

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent


def load(run):
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    info = json.loads((c / "sums.json").read_text())
    rows = list(csv.DictReader(open(run / "diagnostics.csv")))
    diag = {k: np.array([float(r[k]) for r in rows]) for k in rows[0]}
    return meta, c, info, diag


def area_weights(lat, nlon):
    """Normalised cell area weights: Gaussian quadrature weights per row (symmetric, so the
    row order does not matter), equal in longitude."""
    _, w = np.polynomial.legendre.leggauss(len(lat))
    return np.repeat((w / w.sum() / nlon)[:, None], nlon, axis=1)


def gmean(field, wts):
    return float((field * wts).sum())


def storm_index(precip, orog):
    """G: max over cells with flat surroundings of P / mean(P of the neighbours)."""
    nlat, nlon = precip.shape
    best = (0.0, None)
    for j in range(nlat):
        for i in range(nlon):
            nb = []
            for dj in (-1, 0, 1):
                jj = j + dj
                if jj < 0 or jj >= nlat:
                    continue
                for di in (-1, 0, 1):
                    if dj == 0 and di == 0:
                        continue
                    nb.append((jj, (i + di) % nlon))
            if any(abs(orog[jj, ii] - orog[j, i]) > 300.0 for jj, ii in nb):
                continue
            m = np.mean([precip[jj, ii] for jj, ii in nb])
            if m > 0:
                r = precip[j, i] / m
                if r > best[0]:
                    best = (float(r), (j, i))
    return best


def streamfunction(v, sigma_half, lat, ps, a, g):
    """ψ(σ_half, φ) = 2πa cos φ / g ∫_0^p [v] dp [kg/s]."""
    dsig = np.diff(sigma_half)
    cum = np.vstack([np.zeros(v.shape[1]), np.cumsum(v * dsig[:, None], axis=0)])
    return 2 * np.pi * a * np.cos(lat)[None, :] * ps[None, :] / g * cum


def hadley(psi, sigma_half, latd):
    """Strength (max |ψ| within 35° at σ = 0.7) and edge (first sign change poleward)."""
    k = int(np.argmin(np.abs(sigma_half - 0.7)))
    prof = psi[k]
    out = {}
    for name, sel in [("NH", latd > 0), ("SH", latd < 0)]:
        idx = np.where(sel & (np.abs(latd) < 35))[0]
        jx = idx[np.argmax(np.abs(prof[idx]))]
        s0 = np.sign(prof[jx])
        edge = None
        allidx = np.where(sel)[0]
        for j in allidx[np.argsort(np.abs(latd[allidx]))]:
            if abs(latd[j]) > abs(latd[jx]) and np.sign(prof[j]) != s0:
                # linear interpolation of the zero crossing
                jp = allidx[np.argsort(np.abs(latd[allidx]))]
                pos = list(jp).index(j)
                jprev = jp[pos - 1]
                f = prof[jprev] / (prof[jprev] - prof[j])
                edge = float(abs(latd[jprev]) + f * (abs(latd[j]) - abs(latd[jprev])))
                break
        out[name] = {"max_psi_1e10_kg_s": float(abs(prof[jx]) / 1e10),
                     "lat_of_max": float(latd[jx]), "edge_lat": edge}
    return out


def world_metrics(run, meta, c, info, diag):
    lat = np.asarray(meta["grid"]["lat"])
    nlon = len(meta["grid"]["lon"])
    wts = area_weights(lat, nlon)
    a = meta["planet"]["radius"]
    area = 4 * np.pi * a * a
    m = {}
    def mp(name):
        f = c / f"mean_{name}.npy"
        return np.load(f).reshape(len(lat), nlon) if f.exists() else None
    for name, key, scale in [("ts", "global_ts_K", 1), ("precip", "global_precip_mm_day", 86400),
                             ("evap", "global_evap_mm_day", 86400), ("pw", "global_pw_mm", 1),
                             ("olr", "global_olr_W_m2", 1), ("asr", "global_asr_W_m2", 1),
                             ("tas", "global_tas_K", 1)]:
        f = mp(name)
        if f is not None:
            m[key] = gmean(f, wts) * scale
    # window: from the climatology start to the last output
    t = diag["time_s"]
    i0 = int(np.searchsorted(t, info["start_s"]))
    i1 = len(t) - 1
    T = t[i1] - t[i0]
    m["window_days"] = float(T / 86400)
    if "cum_toa_net" in diag:
        def rate(k):
            return (diag[k][i1] - diag[k][i0]) / T / area
        e_atm = diag["energy"] + diag.get("latent_energy", 0 * t)
        de = (e_atm[i1] - e_atm[i0]) / T / area
        toa, sfc = rate("cum_toa_net"), rate("cum_sfc_net")
        m["net_toa_W_m2"] = float(toa)
        m["net_surface_W_m2"] = float(sfc)
        m["atm_energy_tendency_W_m2"] = float(de)
        m["energy_closure_atm_W_m2"] = float(abs(de - (toa - sfc)))
        m["diffusion_ke_W_m2"] = float(rate("cum_diffusion_ke"))
        P, E = rate("cum_precip"), rate("cum_evap")
        m["window_precip_mm_day"] = float(P * 86400)
        m["window_evap_mm_day"] = float(E * 86400)
        m["p_minus_e_rel"] = float(abs(P - E) / E) if E > 0 else None
        if "latent_energy" in diag:
            lv = 2.5e6  # water (Condensable::water); the scorecard reports moist runs only
            dw = (diag["latent_energy"][i1] - diag["latent_energy"][i0]) / lv / T / area
            m["water_closure_kg_m2_s"] = float(abs(dw - (E - P)))
            vap = diag["latent_energy"][i0:i1 + 1].mean() / lv
            if "cum_fixer" in diag:
                fx = (diag["cum_fixer"][i1] - diag["cum_fixer"][i0]) / (T / 86400)
                m["fixer_percent_per_day"] = float(100 * fx / vap)
    if "rsdt" in diag and "asr" in diag:
        sel = slice(i0, i1 + 1)
        m["planetary_albedo"] = float(1 - diag["asr"][sel].mean() / diag["rsdt"][sel].mean())
    p = mp("precip")
    if p is not None:
        orog_f = sorted(run.glob("fields/orog_*.npy"))
        orog = np.load(orog_f[0]).reshape(len(lat), nlon) if orog_f else np.zeros_like(p)
        g, where = storm_index(p, orog)
        m["storm_index_G"] = g
        m["storm_index_cell_latlon"] = None if where is None else [
            float(np.degrees(lat[where[0]])), float(360.0 * where[1] / nlon)]
        m["zonal_precip_mm_day"] = [float(x) for x in p.mean(axis=1) * 86400]
        m["lat_deg"] = [float(x) for x in np.degrees(lat)]
    return m


def aquaplanet(run, meta, c, info, diag, m):
    lat = np.asarray(meta["grid"]["lat"])
    latd = np.degrees(lat)
    sigma = np.asarray(meta["sigma"])
    sh = np.asarray(meta["config"]["sigma_half"])
    u = np.load(c / "zonal_u.npy")
    v = np.load(c / "zonal_v.npy")
    ps = np.load(c / "zonal_ps.npy")
    low = u[-1]
    m["u_low_zonal"] = [float(x) for x in low]
    trop = low[np.abs(latd) < 15]
    m["u_low_tropics_mean"] = float(trop.mean())
    m["u_low_max_within_10deg"] = float(low[np.abs(latd) < 10].max())
    mid = (np.abs(latd) >= 35) & (np.abs(latd) <= 60)
    m["u_low_min_35_60"] = float(low[mid].min())
    for name, sel in [("NH", latd > 0), ("SH", latd < 0)]:
        j = np.where(sel)[0][np.argmax(low[sel])]
        m[f"u_low_peak_{name}"] = float(low[j])
        m[f"u_low_peak_lat_{name}"] = float(latd[j])
        sub = np.where(sel[None, :], u, -np.inf)
        k, jj = np.unravel_index(np.argmax(sub), u.shape)
        m[f"jet_{name}"] = {"u": float(u[k, jj]), "lat": float(latd[jj]), "sigma": float(sigma[k])}
    psi = streamfunction(v, sh, lat, ps, meta["planet"]["radius"], meta["planet"]["gravity"])
    m["hadley"] = hadley(psi, sh, latd)
    zp = np.array(m.get("zonal_precip_mm_day", []))
    if len(zp):
        j = int(np.argmax(zp))
        m["zonal_precip_max"] = {"mm_day": float(zp[j]), "lat": float(latd[j])}
    checks = [
        ("lowest-level easterlies in the tropics (mean |φ|<15° and all |φ|<10° negative)",
         m["u_low_tropics_mean"] < 0 and m["u_low_max_within_10deg"] < 0),
        ("lowest-level westerlies at every latitude 35–60°", m["u_low_min_35_60"] > 0),
        ("peak surface westerlies 5–12 m/s (NH and SH)",
         all(5 <= m[f"u_low_peak_{h}"] <= 12 for h in ("NH", "SH"))),
        ("global P = E within 1%", m.get("p_minus_e_rel") is not None and m["p_minus_e_rel"] < 0.01),
        ("atmospheric energy closure within 0.5 W/m²",
         m.get("energy_closure_atm_W_m2") is not None and m["energy_closure_atm_W_m2"] < 0.5),
        ("grid-point storm index G <= 2.0", m.get("storm_index_G", 99) <= 2.0),
    ]
    return checks


SUITES = {"aquaplanet": aquaplanet}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--suite", required=True, choices=sorted(SUITES))
    ap.add_argument("--milestone", default="P1")
    ap.add_argument("--no-write", action="store_true")
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta, c, info, diag = load(run)
    m = world_metrics(run, meta, c, info, diag)
    checks = SUITES[a.suite](run, meta, c, info, diag, m)
    print(f"{run.name}: T{meta['truncation']}, window {m['window_days']:.0f} days "
          f"({info['count']} climatology samples)")
    skip = {"zonal_precip_mm_day", "lat_deg", "u_low_zonal"}
    for k, v in m.items():
        if k not in skip:
            print(f"  {k}: {v}")
    ok = all(c for _, c in checks)
    for name, c in checks:
        print(f"  {'PASS' if c else 'FAIL'}  {name}")
    print(f"suite {a.suite}: {'PASS' if ok else 'FAIL'}")
    if not a.no_write:
        try:
            git = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], cwd=ROOT,
                                 capture_output=True, text=True).stdout.strip()
        except OSError:
            git = None
        out = ROOT / "docs" / "scorecards"
        out.mkdir(parents=True, exist_ok=True)
        f = out / f"{a.milestone}_{a.suite}_{datetime.date.today().isoformat()}.json"
        f.write_text(json.dumps({
            "run": str(run), "run_git_hash": meta.get("git_hash"), "tool_git_hash": git,
            "truncation": meta["truncation"], "suite": a.suite, "pass": ok,
            "checks": [{"name": n, "pass": bool(c)} for n, c in checks], "metrics": m,
        }, indent=1))
        print(f)
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
