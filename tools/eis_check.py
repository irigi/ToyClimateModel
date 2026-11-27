"""Step A of the owner decision after the r3 gate (P4 question 13, 2026-11-25 evening):
existing output only, no new runs.

    python tools/eis_check.py <run> [--json out.json] [--fig-cover a.png] [--fig-eis b.png]
                              [--rh-run <run with rh snapshots>]

(a) Zonal-mean total cover of the run's climatology (`mean_clt`) against CERES EBAF Ed4.1
    `cldarea_total_daynight` (2001–2020; `reference/derived/t<N>/clt_ceres.npy`, see
    `reference/derived/SOURCES.md`), per Gaussian row from 30° to 90° in both hemispheres.

(b) The estimated-inversion-strength law next to the lower-tropospheric-stability law, both
    evaluated on the run's snapshots (`t`, `tos`, `sic`, `land`, `orog`, `psl`, `cl`, `clt`)
    in the climatology window.

Wood, R. and C. S. Bretherton, 2006: On the relationship between stratiform low cloud cover and
lower-tropospheric stability. J. Climate 19, 6425–6432, DOI 10.1175/JCLI3988.1, read in full
(author's final copy, atmos.uw.edu/~robwood/papers/eis/eis_final.pdf, fetched 2026-11-26):

- eq. (4) EIS = LTS − Γ_m^850 (z700 − LCL), with Γ_m^850 = Γ_m([T0 + T700]/2, 850 hPa) (the
  single-adiabat simplification of eq. 3, "our results are insensitive" to it);
- eq. (5) Γ_m(T, p) = (g/c_p) [1 − (1 + L_v q_s/(R_a T)) / (1 + L_v² q_s/(c_p R_v T²))],
  q_s the saturation mixing ratio;
- eq. (6) z700 = (R_a T0/g) ln(p0/700 hPa);
- LCL from T0, p0 and a constant surface relative humidity RH0 = 0.8 ("consistent with
  surface observations over subsiding regions of the oceans"; COADS 60°S–60°N mean 0.8,
  s.d. 0.07); "standard procedures exist" (no formula given);
- §3: CF = a + b·EIS, a = 0.14 ± 0.06, b = 0.060 ± 0.010 K⁻¹ (95 %), r = 0.92, seasonal means
  of low stratiform cloud amount (St, Sc, fractocumulus, fractostratus, fog; not cumulus) of
  the Klein & Hartmann (1993) regions with LTS > 10 K.

How it is applied here (as the model would, mirroring `low_cloud = "slingo_lts"`):
- LTS = θ(k700) − θ_s exactly as "slingo_lts": k700 the level with σ nearest 0.7,
  θ_s = (SST − 6.5 K km⁻¹ · orog)(1000 hPa/p_s)^κ; SST = `tos`; open-ocean fraction
  f_ocn = (1 − land)(1 − sic); p_s from `psl` reduced to the model orography;
- T0 = SST − 6.5 K km⁻¹ · orog (the scheme has no separate surface air temperature; W&B use the
  surface air temperature, about 1 K below the SST over the oceans), p0 = p_s;
- 700 hPa → the k700 level (p = σ p_s), 850 hPa → 0.85 p_s;
- q_s = the model's single saturation curve, ε e_s(T)/p (TJ16 constants);
- LCL: the parcel (T0, p0, mixing ratio RH0 q_s(T0, p0)) lifted dry-adiabatically to
  saturation; z_LCL = c_p (T0 − T_LCL)/g;
- strat_LTS = clamp(f_ocn (0.057 LTS − 0.5573)); strat_EIS = clamp(f_ocn (0.14 + 0.060 EIS)).
No relative-humidity cap and no freeze-dry factor: the r3 run saved no humidity. Both laws are
treated the same way.

Implied change of the cloud field (central estimate, per snapshot): the stratus goes into the
lower level k of the most stable pair below 750 hPa (as the scheme). With c = the snapshot's
cloud fraction there and L, E the two laws:
- c ≤ L + 0.005 (the stratus sets the layer, possibly after the RH cap and freeze-dry):
  c' = min(1, c E / L) (cap and freeze-dry assumed multiplicative; c' = E where L = 0);
- c > L + 0.005 (the RH cloud sets the layer): c' = max(c, E) (upper estimate: no cap, no
  freeze-dry); the lower estimate leaves these layers unchanged.
Total cover after: 1 − (1 − clt)(1 − low')/(1 − low), low = the column's largest fraction at
σ ≥ 0.8 (random overlap of the change with the rest, as `tools/lts_check.py`).

With --rh-run (a run with `rh`, `ps` snapshots), EIS from RH0 = 0.8 is compared with EIS from
the lowest-level relative humidity of that run (sensitivity of the paper's assumption).
"""

import argparse
import json
import pathlib

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
RD, CP, G, RV, LV, E0, T00 = 287.04, 1004.64, 9.80616, 461.5, 2.5e6, 610.78, 273.16
EPS, KAPPA = RD / RV, RD / CP
LAPSE = 6.5e-3
LTS_A, LTS_B = 0.057, -0.5573
EIS_A, EIS_B, RH0 = 0.060, 0.14, 0.8

# W&B Table 1 boxes (= Klein & Hartmann 1993) for the five decks and the two midlatitude regions,
# plus the owner's regions; peak = local stratus season (KH93 maxima, as in tools/lts_check.py).
REGIONS = [
    ("Peru", -20, -10, 270, 280, (9, 10, 11)),
    ("Namibia", -20, -10, 0, 10, (6, 7, 8, 9, 10, 11)),
    ("California", 20, 30, 230, 240, (6, 7, 8)),
    ("Canary", 15, 25, 325, 335, (6, 7, 8)),
    ("Australia", -35, -25, 95, 105, (12, 1, 2)),
    ("trades 5-20N", 5, 20, 0, 360, None),
    ("trades 5-20S", -20, -5, 0, 360, None),
    ("warm pool", -10, 10, 130, 170, None),
    ("N Pacific 40-60N", 40, 60, 140, 235, (6, 7, 8)),
    ("N Atlantic 40-60N", 40, 60, 300, 355, (6, 7, 8)),
    ("Southern Ocean 45-65S", -65, -45, 0, 360, (12, 1, 2)),
    ("ocean poleward 65N", 65, 90, 0, 360, (6, 7, 8)),
    ("ocean poleward 65S", -90, -65, 0, 360, (12, 1, 2)),
    ("WB06 N Pacific 40-50N 170-180E", 40, 50, 170, 180, (6, 7, 8)),
    ("WB06 N Atlantic 50-60N 35-45W", 50, 60, 315, 325, (6, 7, 8)),
]
SEASONS = {"DJF": (12, 1, 2), "MAM": (3, 4, 5), "JJA": (6, 7, 8), "SON": (9, 10, 11)}
BANDS = [("68-80N", 68, 80), ("45-65N", 45, 65), ("40-65N", 40, 65), ("65-90N", 65, 90),
         ("45-65S", -65, -45), ("65-90S", -90, -65), ("20S-20N", -20, 20), ("global", -90, 90)]


def qsat(t, p):
    return EPS * E0 / p * np.exp(-(LV / RV) * (1.0 / t - 1.0 / T00))


def gamma_m(t, p):
    """W&B eq. (5): moist-adiabatic potential-temperature gradient Γ_m [K m⁻¹]."""
    qs = qsat(t, p)
    return G / CP * (1.0 - (1.0 + LV * qs / (RD * t)) / (1.0 + LV**2 * qs / (CP * RV * t * t)))


def lcl_height(t0, p0, rh0):
    """Dry-adiabatic lift of (t0, p0, r = rh0 q_s) to saturation; height [m]."""
    r = rh0 * qsat(t0, p0)
    lo, hi = t0 - 60.0, t0 + 0.0
    for _ in range(60):  # bisection on f(T) = q_s(T, p(T)) − r, increasing in T
        tm = 0.5 * (lo + hi)
        pm = p0 * (tm / t0) ** (1.0 / KAPPA)
        f = qsat(tm, pm) - r
        lo = np.where(f < 0, tm, lo)
        hi = np.where(f < 0, hi, tm)
    return CP * (t0 - 0.5 * (lo + hi)) / G


def eis_of(t0, p0, t700, p700, rh0=RH0):
    th0 = t0 * (1.0e5 / p0) ** KAPPA
    th7 = t700 * (1.0e5 / p700) ** KAPPA
    lts = th7 - th0
    z700 = RD * t0 / G * np.log(p0 / p700)
    gm = gamma_m(0.5 * (t0 + t700), 0.85 * p0)
    return lts - gm * (z700 - lcl_height(t0, p0, rh0)), lts


def snaps(run, name):
    fs = sorted((run / "fields").glob(f"{name}_*.npy"), key=lambda f: int(f.stem.split("_")[-1]))
    return {int(f.stem.split("_")[-1]): f for f in fs}


def analyse(run):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"])) % 360
    nlat, nlon = len(lat), len(lon)
    sig = np.asarray(meta["sigma"], float)
    nl = len(sig)
    dt, year = float(cfg["dt"]), 365.25 * 86400.0
    start = float(cfg.get("climatology", {}).get("start", 0.0))
    names = ("t", "tos", "sic", "psl", "cl", "clt")
    fs = {n: snaps(run, n) for n in names}
    land = np.load(next(iter(snaps(run, "land").values()))).astype(float).reshape(nlat, nlon)
    orog = np.load(next(iter(snaps(run, "orog").values()))).astype(float).reshape(nlat, nlon)
    steps = sorted(s for s in fs["t"] if all(s in fs[n] for n in names) and s * dt >= start)
    k700 = int(np.argmin(np.abs(sig - 0.7)))
    lowlev = sig >= 0.8
    keys = ("lts", "eis", "s_lts", "s_eis", "low", "low_new", "clt", "clt_new", "clt_lo", "focn")
    acc = {k: [] for k in keys}
    months = []
    for s in steps:
        ld = {n: np.load(fs[n][s]).astype(float) for n in names}
        t = ld["t"].reshape(nl, nlat, nlon)
        sst = ld["tos"].reshape(nlat, nlon)
        sic = np.clip(ld["sic"].reshape(nlat, nlon), 0, 1)
        t0 = sst - LAPSE * orog
        ps = ld["psl"].reshape(nlat, nlon) * np.exp(-G * orog / (RD * t0))
        focn = (1.0 - land) * (1.0 - sic)
        ocean = focn > 0.01
        p = sig[:, None, None] * ps[None]
        th = t * (1.0e5 / p) ** KAPPA
        eis, lts = eis_of(t0, ps, t[k700], p[k700])
        s_l = np.where(ocean, np.clip(focn * (LTS_A * lts + LTS_B), 0, 1), 0.0)
        s_e = np.where(ocean, np.clip(focn * (EIS_B + EIS_A * eis), 0, 1), 0.0)
        best = np.full((nlat, nlon), np.inf)
        kd = np.full((nlat, nlon), nl - 1)
        for k in range(1, nl):
            sl = (th[k] - th[k - 1]) / (p[k] - p[k - 1])
            better = (p[k] >= 75000.0) & (sl < best)
            best = np.where(better, sl, best)
            kd = np.where(better, k, kd)
        cl = ld["cl"].reshape(nl, nlat, nlon)
        c = np.take_along_axis(cl, kd[None], 0)[0]
        strat_dom = c <= s_l + 0.005
        ratio = np.where(s_l > 1e-6, s_e / np.maximum(s_l, 1e-6), np.nan)
        c_new = np.where(strat_dom, np.where(s_l > 1e-6, np.minimum(1.0, c * ratio), s_e), np.maximum(c, s_e))
        c_new = np.where(ocean, c_new, c)
        c_lo = np.where(ocean & strat_dom, c_new, c)  # lower estimate: RH-set layers unchanged
        new = cl.copy()
        np.put_along_axis(new, kd[None], c_new[None], 0)
        new_lo = cl.copy()
        np.put_along_axis(new_lo, kd[None], c_lo[None], 0)
        lo, ln = cl[lowlev].max(axis=0), new[lowlev].max(axis=0)
        ln_lo = new_lo[lowlev].max(axis=0)
        clt = ld["clt"].reshape(nlat, nlon)
        acc["clt_new"].append(1.0 - (1.0 - clt) * (1.0 - ln) / np.maximum(1.0 - lo, 1e-6))
        acc["clt_lo"].append(1.0 - (1.0 - clt) * (1.0 - ln_lo) / np.maximum(1.0 - lo, 1e-6))
        acc["clt"].append(clt)
        acc["low"].append(lo)
        acc["low_new"].append(ln)
        acc["lts"].append(np.where(ocean, lts, np.nan))
        acc["eis"].append(np.where(ocean, eis, np.nan))
        acc["s_lts"].append(s_l)
        acc["s_eis"].append(s_e)
        acc["focn"].append(focn)
        months.append(int(((s * dt) % year) / (year / 12.0)) + 1)
    arr = {k: np.asarray(v) for k, v in acc.items()}
    months = np.asarray(months)
    w2 = np.cos(np.radians(lat))[:, None] * np.ones((nlat, nlon))
    res = {"run": str(run), "n_snapshots": len(steps), "k700_sigma": float(sig[k700]), "regions": {}}

    def regmean(x, sel, m):
        x = x[sel][:, m]
        ww = np.broadcast_to(w2[m], x.shape)
        g = np.isfinite(x)
        return float((x[g] * ww[g]).sum() / ww[g].sum()) if g.any() else float("nan")

    for name, la0, la1, lo0, lo1, peak in REGIONS:
        m = (lat[:, None] >= la0) & (lat[:, None] <= la1) & (lon[None] >= lo0) & (lon[None] <= lo1) & (land < 0.5)
        if not m.any():
            continue
        r = {"points": int(m.sum())}
        sels = {"annual": np.ones(len(steps), bool)}
        sels.update({k: np.isin(months, v) for k, v in SEASONS.items()})
        if peak:
            sels["peak"] = np.isin(months, peak)
        for tag, sel in sels.items():
            if sel.any():
                r[tag] = {k: regmean(arr[k], sel, m) for k in ("lts", "eis", "s_lts", "s_eis", "low", "low_new")}
        res["regions"][name] = r
    # zonal means (land counts as 0 for the laws) and bands
    z = {k: np.nanmean(arr[k], axis=0).mean(axis=1) for k in ("s_lts", "s_eis", "low", "low_new", "clt", "clt_new", "clt_lo")}
    clim = np.load(run / "climatology" / "mean_clt.npy").astype(float).reshape(nlat, nlon)
    z["clt_clim"] = clim.mean(axis=1)
    cer = ROOT / "reference" / "derived" / f"t{cfg.get('truncation')}" / "clt_ceres.npy"
    c = np.load(cer).astype(float)
    z["ceres"] = (c.mean(axis=0) if c.ndim == 3 else c).reshape(nlat, nlon).mean(axis=1)
    wl = np.cos(np.radians(lat))
    res["bands"] = {}
    for name, a0, a1 in BANDS:
        m = (lat >= a0) & (lat <= a1)
        res["bands"][name] = {k: float((v[m] * wl[m]).sum() / wl[m].sum()) for k, v in z.items()}
    res["lat"] = lat.tolist()
    res["zonal"] = {k: v.tolist() for k, v in z.items()}
    # seasonal zonal means of the stratus laws (ocean-weighted is the region table; this is zonal)
    res["zonal_season"] = {
        tag: {k: np.nanmean(arr[k][np.isin(months, v)], axis=0).mean(axis=1).tolist() for k in ("s_lts", "s_eis")}
        for tag, v in SEASONS.items()}
    res["_maps"] = {k: np.nanmean(arr[k], axis=0) for k in ("lts", "eis", "s_lts", "s_eis")}
    res["_lon"] = lon
    return res


def rh_sensitivity(run):
    """EIS with RH0 = 0.8 against EIS with the lowest level's RH, on a run with rh and ps."""
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlat, nlon = len(lat), len(meta["grid"]["lon"])
    sig = np.asarray(meta["sigma"], float)
    nl = len(sig)
    k700 = int(np.argmin(np.abs(sig - 0.7)))
    fs = {n: snaps(run, n) for n in ("t", "rh", "ts", "ps")}
    land = np.load(next(iter(snaps(run, "land").values()))).reshape(nlat, nlon) > 0.5
    steps = sorted(s for s in fs["t"] if all(s in fs[n] for n in fs))
    d, rhs = [], []
    for s in steps[:: max(1, len(steps) // 24)]:
        t = np.load(fs["t"][s]).astype(float).reshape(nl, nlat, nlon)
        rh = np.load(fs["rh"][s]).astype(float).reshape(nl, nlat, nlon)
        ts = np.load(fs["ts"][s]).astype(float).reshape(nlat, nlon)
        ps = np.load(fs["ps"][s]).astype(float).reshape(nlat, nlon)
        oc = ~land & (ts >= 271.35) & (np.abs(lat)[:, None] <= 60)
        e8, _ = eis_of(ts, ps, t[k700], sig[k700] * ps)
        r0 = np.clip(rh[-1], 0.3, 1.0)
        er, _ = eis_of(ts, ps, t[k700], sig[k700] * ps, r0)
        d.append((er - e8)[oc])
        rhs.append(rh[-1][oc])
    d, rhs = np.concatenate(d), np.concatenate(rhs)
    return {"run": str(run), "n": int(d.size), "rh_low_mean": float(rhs.mean()), "rh_low_sd": float(rhs.std()),
            "dEIS_mean": float(d.mean()), "dEIS_p05": float(np.percentile(d, 5)),
            "dEIS_p95": float(np.percentile(d, 95))}


def fig_cover(r, path):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    lat = np.asarray(r["lat"])
    z = r["zonal"]
    fig, axs = plt.subplots(1, 2, figsize=(12, 4.5), sharey=True)
    for ax, (a0, a1, t) in zip(axs, ((-90, -30, "30–90°S"), (30, 90, "30–90°N"))):
        m = (lat >= a0) & (lat <= a1)
        ax.plot(lat[m], np.asarray(z["ceres"])[m], "k-o", ms=3, label="CERES EBAF Ed4.1 2001–2020")
        ax.plot(lat[m], np.asarray(z["clt_clim"])[m], "C0-o", ms=3, label="r3 T42, years 2–6")
        ax.plot(lat[m], np.asarray(z["clt_new"])[m], "C1--", label="r3 with EIS law (offline estimate)")
        ax.set_title(t)
        ax.set_xlabel("latitude")
        ax.grid(alpha=0.3)
    axs[0].set_ylabel("total cloud cover")
    axs[1].legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


def fig_eis(r, path):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    lat = np.asarray(r["lat"])
    lon = r["_lon"]
    order = np.argsort(np.where(lon > 180, lon - 360, lon))
    lo = np.where(lon > 180, lon - 360, lon)[order]
    fig = plt.figure(figsize=(14, 9))
    for i, (k, lab, vmin, vmax, cm) in enumerate((
            ("lts", "LTS [K]", 0, 30, "viridis"), ("eis", "EIS [K] (W&B eq. 4, RH0 0.8)", -6, 14, "viridis"),
            ("s_lts", "LTS law 0.057 LTS − 0.5573", 0, 1, "Blues"), ("s_eis", "EIS law 0.14 + 0.060 EIS", 0, 1, "Blues"))):
        ax = fig.add_subplot(3, 2, i + 1)
        im = ax.pcolormesh(lo, lat, r["_maps"][k][:, order], vmin=vmin, vmax=vmax, cmap=cm, shading="auto")
        ax.set_title(lab, fontsize=9)
        fig.colorbar(im, ax=ax, shrink=0.8)
    ax = fig.add_subplot(3, 2, 5)
    z = r["zonal"]
    ax.plot(lat, z["s_lts"], label="LTS law (zonal, land 0)")
    ax.plot(lat, z["s_eis"], label="EIS law (zonal, land 0)")
    ax.plot(lat, z["low"], label="r3 low cloud σ≥0.8, max")
    ax.plot(lat, z["low_new"], "--", label="low cloud with EIS (est.)")
    ax.legend(fontsize=7)
    ax.grid(alpha=0.3)
    ax.set_xlabel("latitude")
    ax = fig.add_subplot(3, 2, 6)
    ax.plot(lat, z["ceres"], "k", label="CERES")
    ax.plot(lat, z["clt"], label="r3 cover (snapshots)")
    ax.plot(lat, z["clt_new"], "--", label="with EIS (est.)")
    ax.legend(fontsize=7)
    ax.grid(alpha=0.3)
    ax.set_xlabel("latitude")
    fig.tight_layout()
    fig.savefig(path, dpi=100)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--json")
    ap.add_argument("--fig-cover")
    ap.add_argument("--fig-eis")
    ap.add_argument("--rh-run")
    a = ap.parse_args()
    r = analyse(a.run)
    lat = np.asarray(r["lat"])
    z = r["zonal"]
    print(f"{a.run}: {r['n_snapshots']} snapshots, k700 σ {r['k700_sigma']:.3f}\n")
    print("(a) zonal-mean total cover, 30–90°: lat | r3 (climatology) | CERES | r3 − CERES")
    for j in np.argsort(lat):
        if abs(lat[j]) >= 30:
            print(f"{lat[j]:6.1f} | {z['clt_clim'][j]:.3f} | {z['ceres'][j]:.3f} | {z['clt_clim'][j] - z['ceres'][j]:+.3f}")
    print("\n(b) regions (open ocean, f_ocn-weighted laws): region | season | LTS | EIS | LTS law | EIS law "
          "| low σ≥0.8 now | low with EIS (est.)")
    for name, rr in r["regions"].items():
        for tag in ("annual", "DJF", "MAM", "JJA", "SON", "peak"):
            if tag in rr:
                x = rr[tag]
                print(f"| {name} | {tag} | {x['lts']:.1f} | {x['eis']:.1f} | {x['s_lts']:.2f} | {x['s_eis']:.2f} "
                      f"| {x['low']:.2f} | {x['low_new']:.2f} |")
    print("\nbands: band | LTS law | EIS law | low now | low EIS | cover (snap) | cover EIS (est.) | cover EIS (lower est.) | cover clim | CERES")
    for name, b in r["bands"].items():
        print(f"| {name} | {b['s_lts']:.3f} | {b['s_eis']:.3f} | {b['low']:.3f} | {b['low_new']:.3f} | {b['clt']:.3f} "
              f"| {b['clt_new']:.3f} | {b['clt_lo']:.3f} | {b['clt_clim']:.3f} | {b['ceres']:.3f} |")
    if a.rh_run:
        r["rh_sensitivity"] = rh_sensitivity(a.rh_run)
        print("\nRH0 sensitivity:", r["rh_sensitivity"])
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps({k: v for k, v in r.items() if not k.startswith("_")}, indent=1))
    if a.fig_cover:
        fig_cover(r, a.fig_cover)
    if a.fig_eis:
        fig_eis(r, a.fig_eis)


if __name__ == "__main__":
    main()
