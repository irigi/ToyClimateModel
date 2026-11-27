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
- with a surface model: system energy closure |d(atmosphere + surface energy)/dt − (net TOA
  + q-flux + ice-cap flux)| and system water closure |d(vapour + bucket + snow)/dt + runoff
  + water taken by the ocean| (SPEC.md P2 acceptance 2–3);
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
- recorded: zonal-mean precipitation, Hadley cell strength and edge, jets (maximum zonal-mean u at σ > 0.1).

Suite `earth_amip` (SPEC.md §9.6 P5; recorded without thresholds in P2, item 7: use
`--record`) compares against the reference data of tools/fetch_reference.py
(reference/derived/t<N>/, the run's truncation). Land = cells with land fraction ≥ 0.5;
correlations are area-weighted and centred; seasons from the monthly climatology
(DJF = Dec, Jan, Feb):
- global-mean precipitation; zonal-mean annual precipitation vs GPCP (correlation over
  latitude, tropical peak 10°S–12°N, subtropical minima 10–40° per hemisphere); annual
  precipitation map pattern correlation;
- annual-mean land 2 m temperature RMSE after a 6.5 K/km lapse-rate correction from the
  model orography to the NCEP/NCAR reanalysis orography of the reference;
- July − January land 2 m temperature: pattern correlation and amplitude ratio (ratio of
  the area-weighted RMS of the difference, model / reference);
- sea-level pressure DJF and JJA, 60°S–60°N: pattern correlation;
- winter jets at 200 hPa: maximum zonal-mean u200 at 15–70° (DJF north, JJA south);
- monsoon: JJA − DJF precipitation over 10–25°N, 70–90°E;
- Köppen (tools/koppen.py rules on the monthly tas and precip) vs Beck et al. (2018):
  main-group and 30-class agreement by land area; majority main group in the named boxes;
- diurnal 2 m range from hourly stations `sahara` and `tropical_pacific` (mean daily
  max − min of the window);
- NH snow cover extent in February and August: Σ area × land fraction × S/(S + S_c) of
  the monthly-mean snow (S_c = 15 kg m⁻²), against Σ area × the Rutgers cover;
- G and the closures from the world metrics; cost from the run log, if given.
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


def storm_index(precip, orog, floor=1.0 / 86400):
    """G: max over cells with flat surroundings of P / mean(P of the neighbours), counting
    only cells whose neighbour mean is at least `floor` (1 mm/day; SPEC.md §9.5, owner
    decision 2026-11-18, open question 8)."""
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
            if m >= floor:
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
        # P2: whole-system closures with the surface reservoirs (slab ocean, soil, snow,
        # sea ice); the ocean is an infinite water reservoir, a fixed SST also for energy
        fixed = bool(((meta.get("config") or {}).get("surface") or {}).get("fixed", False))
        if "surface_energy" in diag and "cum_qflux" in diag and not fixed:
            es = diag["surface_energy"]
            ds = (es[i1] - es[i0]) / T / area
            src = toa + rate("cum_qflux") + rate("cum_ice_cap")
            m["surface_energy_tendency_W_m2"] = float(ds)
            m["energy_closure_system_W_m2"] = float(abs(de + ds - src))
            m["ice_cap_flux_W_m2"] = float(rate("cum_ice_cap"))
        if "surface_water" in diag and "latent_energy" in diag and "cum_runoff" in diag:
            lv = 2.5e6
            dw = (diag["latent_energy"][i1] - diag["latent_energy"][i0]) / lv / T / area
            sw = diag["surface_water"]
            dws = (sw[i1] - sw[i0]) / T / area
            out = rate("cum_runoff") + rate("cum_ocean_uptake")
            m["runoff_mm_day"] = float(rate("cum_runoff") * 86400)
            m["water_closure_system_mm_day"] = float(abs(dw + dws + out) * 86400)
    dt = (meta.get("config") or {}).get("dt")
    if "max_u" in diag and dt:
        # SPEC.md §9.8 time-step trigger (owner decision 2026-11-18, open question 5):
        # spectral Courant number |v|_max Δt √(M(M+1)) / a, at the diagnostics rows
        mt = meta["truncation"]
        m["spectral_courant_max"] = float(diag["max_u"].max() * dt * np.sqrt(mt * (mt + 1)) / a)
    # planetary albedo from the climatology maps (sampled every `climatology.every`); the
    # diagnostics rows are a few instantaneous snapshots when diagnostics_every is long
    asr_map, rsdt_map = mp("asr"), mp("rsdt")
    if asr_map is not None and rsdt_map is not None:
        m["planetary_albedo"] = float(1 - gmean(asr_map, wts) / gmean(rsdt_map, wts))
        m["planetary_albedo_source"] = "climatology"
    elif "rsdt" in diag and "asr" in diag:
        sel = slice(i0, i1 + 1)
        m["planetary_albedo"] = float(1 - diag["asr"][sel].mean() / diag["rsdt"][sel].mean())
        m["planetary_albedo_source"] = "diagnostics rows"

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
        # tropospheric jet: levels below σ = 0.1 (the top levels carry the
        # stratospheric/sponge winds, which are not a target, SPEC.md §9.7)
        sub = np.where(sel[None, :] & (sigma[:, None] > 0.1), u, -np.inf)
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


def wcorr(a, b, w):
    """Area-weighted centred pattern correlation over the finite cells of w."""
    sel = np.isfinite(a) & np.isfinite(b) & (w > 0)
    a, b, w = a[sel], b[sel], w[sel]
    a = a - np.average(a, weights=w)
    b = b - np.average(b, weights=w)
    return float(np.sum(w * a * b) / np.sqrt(np.sum(w * a * a) * np.sum(w * b * b)))


def season(x, months):
    return x[list(months)].mean(axis=0)


DJF, JJA = (11, 0, 1), (5, 6, 7)

# boxes: (lat0, lat1, lon0, lon1) in degrees east 0-360, expected main group
NAMED_REGIONS = [
    ("Sahara", (18, 30, 350, 30), "B"), ("Arabia", (18, 28, 42, 55), "B"),
    ("central Australia", (-30, -20, 120, 140), "B"), ("Amazon", (-10, 0, 290, 305), "A"),
    ("Congo", (-5, 5, 15, 28), "A"), ("Borneo/Sumatra", (-5, 5, 100, 118), "A"),
    ("central Siberia", (55, 65, 90, 120), "D"), ("Greenland interior", (65, 80, 310, 330), "E"),
    ("Antarctica", (-90, -70, 0, 360), "E"),
]


def in_box(latd, lond, box):
    la0, la1, lo0, lo1 = box
    lat_ok = (latd >= la0) & (latd <= la1)
    lon_ok = (lond >= lo0) & (lond <= lo1) if lo0 <= lo1 else (lond >= lo0) | (lond <= lo1)
    return lat_ok[:, None] & lon_ok[None, :]


def earth_amip(run, meta, c, info, diag, m):
    import sys

    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))  # works under python -I
    import koppen

    lat = np.asarray(meta["grid"]["lat"])
    latd = np.degrees(lat)
    nlon = len(meta["grid"]["lon"])
    lond = np.degrees(np.asarray(meta["grid"]["lon"]))
    wts = area_weights(lat, nlon)
    ref = ROOT / "reference" / "derived" / f"t{meta['truncation']}"
    if not ref.exists():
        raise SystemExit(f"{ref} missing: run tools/fetch_reference.py")
    R = lambda k: np.load(ref / f"{k}.npy").astype(float)  # noqa: E731
    M = lambda k: np.load(c / f"monthly_{k}.npy")  # noqa: E731
    land = R("land")
    lm = land >= 0.5
    a = meta["planet"]["radius"]
    cell_area = wts * 4 * np.pi * a * a
    rec = {}

    # precipitation [mm/day]
    pm = M("precip") * 86400
    pr = R("pr")
    pa, ra = pm.mean(0), pr.mean(0)
    zm, zr = pa.mean(1), ra.mean(1)
    rec["global_precip_mm_day"] = gmean(pa, wts)
    rec["global_precip_ref_mm_day"] = gmean(ra, wts)
    wl = wts[:, 0]
    rec["zonal_precip_corr"] = wcorr(zm, zr, wl)
    trop = (latd >= -10) & (latd <= 12)
    j = np.where(trop)[0][np.argmax(zm[trop])]
    rec["tropical_peak"] = {"mm_day": float(zm[j]), "lat": float(latd[j])}
    jr = np.where(trop)[0][np.argmax(zr[trop])]
    rec["tropical_peak_ref"] = {"mm_day": float(zr[jr]), "lat": float(latd[jr])}
    for h, sel in [("NH", (latd > 10) & (latd < 40)), ("SH", (latd < -10) & (latd > -40))]:
        rec[f"subtropical_min_{h}_mm_day"] = float(zm[sel].min())
        rec[f"subtropical_min_{h}_ref_mm_day"] = float(zr[sel].min())
    rec["precip_map_corr"] = wcorr(pa, ra, wts)
    rec["zonal_precip_ref_mm_day"] = [float(x) for x in zr]

    # land 2 m temperature [K]
    tm, tr_ = M("tas"), R("tas")
    orog_f = sorted(run.glob("fields/orog_*.npy"))
    zmod = np.load(orog_f[0]) if orog_f else R("orog")
    corr_lapse = 0.0065 * (zmod - R("orog_tas_ref"))
    wland = wts * lm
    d = tm.mean(0) + corr_lapse - tr_.mean(0)
    rec["land_tas_rmse_K"] = float(np.sqrt(np.sum(wland * d * d) / wland.sum()))
    rec["land_tas_bias_K"] = float(np.sum(wland * d) / wland.sum())
    sm, sr = tm[6] - tm[0], tr_[6] - tr_[0]
    rec["land_tas_jul_jan_corr"] = wcorr(np.where(lm, sm, np.nan), sr, wts)
    rms = lambda x: np.sqrt(np.sum(wland * x * x) / wland.sum())  # noqa: E731
    rec["land_tas_jul_jan_amplitude_ratio"] = float(rms(sm) / rms(sr))

    # sea-level pressure, 60S-60N
    band = (np.abs(latd) <= 60)[:, None] * np.ones((1, nlon))
    for nm, ms in [("DJF", DJF), ("JJA", JJA)]:
        rec[f"psl_{nm}_corr"] = wcorr(season(M("psl"), ms), season(R("psl"), ms), wts * band)

    # 200 hPa winter jets
    for h, ms, sel in [("NH_DJF", DJF, (latd > 15) & (latd < 70)),
                       ("SH_JJA", JJA, (latd < -15) & (latd > -70))]:
        for tag, f in [("", M("u200")), ("_ref", R("ua200"))]:
            z = season(f, ms).mean(1)
            j = np.where(sel)[0][np.argmax(z[sel])]
            rec[f"jet200_{h}{tag}"] = {"u": float(z[j]), "lat": float(latd[j])}

    # monsoon
    box = in_box(latd, lond, (10, 25, 70, 90))
    for tag, f in [("", pm), ("_ref", pr)]:
        dj = season(f, JJA) - season(f, DJF)
        rec[f"monsoon_jja_minus_djf_mm_day{tag}"] = float(np.sum(wts * box * dj) / np.sum(wts * box))

    # Köppen
    month_s = meta["calendar"]["year"] / len(meta["calendar"]["months"])
    T = tm - 273.15
    P = M("precip") * month_s
    codes = np.zeros(land.shape, int)
    for j in range(len(lat)):
        for i in range(nlon):
            codes[j, i] = koppen.CODE[koppen.classify(T[:, j, i], P[:, j, i])] + 1
    beck = R("koppen_beck").astype(int)
    group = lambda x: np.array([" ABCDE"[0] if v == 0 else koppen.CLASSES[v - 1][0][0]  # noqa: E731
                                for v in x.ravel()]).reshape(x.shape)
    gm, gr = group(codes), group(beck)
    valid = (beck > 0) & lm
    wv = wts * land * valid
    rec["koppen_main_group_agreement"] = float(np.sum(wv * (gm == gr)) / wv.sum())
    rec["koppen_30class_agreement"] = float(np.sum(wv * (codes == beck)) / wv.sum())
    regions = {}
    for name, bx, want in NAMED_REGIONS:
        sel = in_box(latd, lond, bx) & lm
        if not sel.any():
            regions[name] = {"expected": want, "model": None}
            continue
        votes = {}
        for g, w in zip(gm[sel], (wts * land)[sel]):
            votes[g] = votes.get(g, 0.0) + w
        maj = max(votes, key=votes.get)
        regions[name] = {"expected": want, "model": maj, "cells": int(sel.sum())}
    rec["named_regions"] = regions
    np.save(c / "koppen_earth_amip.npy", codes)

    # diurnal 2 m range from hourly stations
    st = run / "stations"
    if (st / "tas.npy").exists():
        names = [s["name"] for s in json.loads((st / "stations.json").read_text())["stations"]]
        tas = np.load(st / "tas.npy")
        t = np.load(st / "time.npy")
        sel = t >= info["start_s"]
        day = np.floor(t[sel] / 86400.0).astype(int)
        for nm in ("sahara", "tropical_pacific"):
            if nm in names:
                x = tas[sel, names.index(nm)]
                rng = [x[day == dd].max() - x[day == dd].min() for dd in np.unique(day)
                       if (day == dd).sum() >= 20]
                rec[f"diurnal_range_{nm}_K"] = float(np.mean(rng))

    # NH snow cover extent [million km2]
    snw = M("snw")
    fs = snw / (snw + 15.0)
    snc = np.nan_to_num(R("snc"))
    nh = (latd > 0)[:, None]
    for nm, mo in [("Feb", 1), ("Aug", 7)]:
        rec[f"nh_snow_extent_{nm}_Mkm2"] = float(np.sum(cell_area * land * fs[mo] * nh) / 1e12)
        rec[f"nh_snow_extent_{nm}_ref_Mkm2"] = float(np.sum(cell_area * snc[mo] * nh) / 1e12)

    # cost from the run log (if it sits next to the run as <run>.log)
    log = run.parent / f"{run.name}.log"
    if log.exists():
        import re
        mt = re.findall(r"done: ([0-9.]+) model days in ([0-9.]+) s wall", log.read_text())
        if mt:
            days, wall = map(float, mt[-1])
            rec["cost_min_per_model_year"] = wall / 60 / (days / 365.25)
    m["earth_amip"] = rec

    rg = rec["named_regions"]
    checks = [
        ("global-mean precipitation 2.4–3.4 mm/day", 2.4 <= rec["global_precip_mm_day"] <= 3.4),
        ("zonal-mean precipitation vs GPCP: correlation ≥ 0.85", rec["zonal_precip_corr"] >= 0.85),
        ("tropical peak 5–9 mm/day between 10°S and 12°N",
         5 <= rec["tropical_peak"]["mm_day"] <= 9),
        ("subtropical minima < 2.5 mm/day in both hemispheres",
         max(rec["subtropical_min_NH_mm_day"], rec["subtropical_min_SH_mm_day"]) < 2.5),
        ("annual precipitation map pattern correlation ≥ 0.60", rec["precip_map_corr"] >= 0.60),
        ("land 2 m temperature RMSE ≤ 4.5 K (lapse-rate corrected)", rec["land_tas_rmse_K"] <= 4.5),
        ("July − January land 2 m temperature: correlation ≥ 0.90",
         rec["land_tas_jul_jan_corr"] >= 0.90),
        ("July − January land 2 m temperature: amplitude ratio 0.75–1.3",
         0.75 <= rec["land_tas_jul_jan_amplitude_ratio"] <= 1.3),
        ("sea-level pressure DJF and JJA, 60°S–60°N: correlation ≥ 0.60 each",
         min(rec["psl_DJF_corr"], rec["psl_JJA_corr"]) >= 0.60),
        ("winter jets at 200 hPa: 30–55 m/s at 25–45° in each hemisphere",
         all(30 <= rec[k]["u"] <= 55 and 25 <= abs(rec[k]["lat"]) <= 45
             for k in ("jet200_NH_DJF", "jet200_SH_JJA"))),
        ("monsoon: JJA − DJF precipitation 10–25°N, 70–90°E ≥ 3 mm/day",
         rec["monsoon_jja_minus_djf_mm_day"] >= 3),
        ("Köppen main group vs Beck et al.: ≥ 60% of land area agrees",
         rec["koppen_main_group_agreement"] >= 0.60),
        ("named regions have their majority class",
         all(r["model"] == r["expected"] for r in rg.values())),
        ("diurnal 2 m range: Sahara 10–22 K, open tropical ocean < 1.5 K",
         10 <= rec.get("diurnal_range_sahara_K", -1) <= 22
         and rec.get("diurnal_range_tropical_pacific_K", 99) < 1.5),
        ("NH snow cover: February 35–58 million km², August < 10 million km²",
         35 <= rec["nh_snow_extent_Feb_Mkm2"] <= 58 and rec["nh_snow_extent_Aug_Mkm2"] < 10),
        ("G ≤ 2.0", m.get("storm_index_G", 99) <= 2.0),
        ("energy closure (atmosphere) within 0.5 W/m²",
         m.get("energy_closure_atm_W_m2", 99) < 0.5),
        ("water closure within 1e-3 mm/day",
         m.get("water_closure_system_mm_day", 99) < 1e-3),
        ("cost ≤ 30 min per model year (T42 L26, 8 threads)",
         rec.get("cost_min_per_model_year", 99) <= 30),
    ]
    return checks


def earth_energy(run, meta, c, info, diag, m):
    """SPEC.md P4 acceptance 1-3 and 5 (Earth AMIP with clouds): global radiation, cloud
    radiative effects, total cloud cover and its zonal shape."""
    lat = np.asarray(meta["grid"]["lat"])
    latd = np.degrees(lat)
    nlon = len(meta["grid"]["lon"])
    wts = area_weights(lat, nlon)

    def g(k):
        f = c / f"mean_{k}.npy"
        if not f.exists():
            raise SystemExit(f"{f} missing: the climatology needs maps rsdt, asr, olr, asr_cs, olr_cs, clt")
        x = np.load(f).reshape(len(lat), nlon)
        return gmean(x, wts), x

    rsdt, asr, asr_cs, olr, olr_cs = (g(k)[0] for k in ["rsdt", "asr", "asr_cs", "olr", "olr_cs"])
    clt, cltmap = g("clt")
    albedo = 1 - asr / rsdt
    sw_cre, lw_cre = asr - asr_cs, olr_cs - olr
    z = cltmap.mean(axis=1)
    m["earth_energy"] = rec = {
        "planetary_albedo": float(albedo),
        "clear_sky_albedo": float(1 - asr_cs / rsdt),
        "olr_W_m2": float(olr),
        "net_toa_W_m2": float(asr - olr),
        "sw_cre_W_m2": float(sw_cre),
        "lw_cre_W_m2": float(lw_cre),
        "total_cloud_cover": float(clt),
        "zonal_clt": [float(x) for x in z],
        "lat": [float(x) for x in latd],
    }
    if (c / "mean_clwvi.npy").exists():
        rec["clwvi_kg_m2"] = g("clwvi")[0]

    def band(lo, hi):
        sel = (np.abs(latd) >= lo) & (np.abs(latd) <= hi)
        return sel

    # shape: a maximum within 15° of the equator, a local maximum (not at the poleward edge
    # of the band) in 40-70° of each hemisphere, the 15-35° minimum of each hemisphere below
    # both
    shape = True
    eq = z[np.abs(latd) <= 15].max()
    notes = []
    for hem, sgn in (("N", 1), ("S", -1)):
        h = sgn * latd > 0
        sub = z[h & band(15, 35)].min()
        st_sel = h & band(40, 70)
        zs, ls = z[st_sel], np.abs(latd[st_sel])
        k = int(np.argmax(zs))
        local = ls[k] < ls.max() and zs[k] > zs[ls == ls.max()][0]
        ok = local and sub < eq and sub < zs[k]
        notes.append(f"{hem}: subtropical min {sub:.2f}, storm-track max {zs[k]:.2f} at {ls[k]:.0f}° "
                     f"(local {local}), ITCZ max {eq:.2f}")
        shape = shape and ok
    rec["zonal_shape"] = notes
    checks = [
        ("planetary albedo 0.27-0.33", 0.27 <= albedo <= 0.33),
        ("OLR 228-250 W/m²", 228 <= olr <= 250),
        ("net TOA within ±8 W/m²", abs(asr - olr) <= 8),
        ("SW cloud radiative effect -30 to -60 W/m²", -60 <= sw_cre <= -30),
        ("LW cloud radiative effect +15 to +35 W/m²", 15 <= lw_cre <= 35),
        ("total cloud cover 0.5-0.75", 0.5 <= clt <= 0.75),
        ("cloud maxima in the ITCZ and storm tracks, subtropical minima", shape),
        ("energy closure (atmosphere) within 0.5 W/m²",
         m.get("energy_closure_atm_W_m2", 99) < 0.5),
    ]
    return checks


SUITES = {"aquaplanet": aquaplanet, "earth_amip": earth_amip, "earth_energy": earth_energy}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--suite", choices=sorted(SUITES),
                    help="check suite; without it the world metrics are printed only")
    ap.add_argument("--milestone", default="P1")
    ap.add_argument("--no-write", action="store_true")
    ap.add_argument("--record", action="store_true",
                    help="record only: thresholds are reported, not applied (exit 0)")
    ap.add_argument("--tag", default=None, help="appended to the scorecard file name")
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta, c, info, diag = load(run)
    m = world_metrics(run, meta, c, info, diag)
    checks = SUITES[a.suite](run, meta, c, info, diag, m) if a.suite else []
    print(f"{run.name}: T{meta['truncation']}, window {m['window_days']:.0f} days "
          f"({info['count']} climatology samples)")
    skip = {"zonal_precip_mm_day", "lat_deg", "u_low_zonal", "zonal_precip_ref_mm_day"}
    for k, v in m.items():
        if k == "earth_amip":
            for kk, vv in v.items():
                if kk not in skip:
                    print(f"  {kk}: {vv}")
        elif k not in skip:
            print(f"  {k}: {v}")
    ok = all(c for _, c in checks)
    word = ("would pass", "would fail") if a.record else ("PASS", "FAIL")
    for name, c in checks:
        print(f"  {word[0] if c else word[1]:10s}  {name}")
    if not a.suite:
        return
    if a.record:
        print(f"suite {a.suite}: recorded ({sum(c for _, c in checks)} of {len(checks)} "
              f"rows would pass; thresholds not applied)")
    else:
        print(f"suite {a.suite}: {'PASS' if ok else 'FAIL'}")
    if not a.no_write:
        try:
            git = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], cwd=ROOT,
                                 capture_output=True, text=True).stdout.strip()
        except OSError:
            git = None
        out = ROOT / "docs" / "scorecards"
        out.mkdir(parents=True, exist_ok=True)
        tag = f"_{a.tag}" if a.tag else ""
        f = out / f"{a.milestone}_{a.suite}{tag}_{datetime.date.today().isoformat()}.json"
        f.write_text(json.dumps({
            "run": str(run), "run_git_hash": meta.get("git_hash"), "tool_git_hash": git,
            "truncation": meta["truncation"], "suite": a.suite,
            "pass": None if a.record else ok, "record_only": a.record,
            "checks": [{"name": n, "pass": bool(c)} for n, c in checks], "metrics": m,
        }, indent=1))
        print(f)
    raise SystemExit(0 if ok or a.record else 1)


if __name__ == "__main__":
    main()
