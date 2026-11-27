"""Boundary-layer humidity diagnosis (P4 question 13, owner decision after the diagnosis,
2026-11-26): near-surface relative humidity, evaporation and precipitation by band against
observations, the moisture budget of the lowest levels, and the per-run toggle table.

    python tools/bl_humidity.py <run> [<run> ...] [--labels a b ...] [--json out.json]
        [--fig <png>]

Observations (all fetched 2026-11-26 into `reference/raw/`, long-term monthly means):
- near-surface RH over the ocean: ICOADS 2° enhanced monthly `rhum.mean.nc` (NOAA PSL;
  Freeman et al. 2017, Int. J. Climatol. 37, 2211–2232; ship and buoy reports, about
  10–20 m), averaged here over 1991–2020 per calendar month; and NCEP/NCAR reanalysis RH at
  σ = 0.995 (`rhum.sig995.mon.ltm.1991-2020.nc`, Kalnay et al. 1996; a model level close to
  ours, σ 0.9925);
- precipitation: GPCP v2.3 (`precip.mon.ltm.1991-2020.nc`, Adler et al. 2018);
- evaporation over the ocean: NCEP/NCAR latent heat flux (`lhtfl.sfc.mon.ltm.1991-2020.nc`),
  E = LHF / 2.5e6 J/kg. This is reanalysis model output, not an observation (no OAFlux
  copy could be fetched);
- convective share of precipitation: NCEP/NCAR `cprat` / `prate` (reanalysis model output,
  its own convection scheme; no observed convective share was fetched).
Ocean cells: OISST v2 land-sea mask (nearest 1° cell) for the NCEP and GPCP grids; ICOADS
has ocean cells only; model: `reference/derived/t<N>/land.npy` < 0.5. Sea ice is not
masked. Bands are area-weighted over ocean cells (cos φ on the regular grids, the Gaussian
weights in the model).

Model (monthly climatology maps of year 2): `rh_low` (lowest level, σ 0.9925, at the start
of each physics step), `evap`, `precip`, `precip_conv`; budget probes (physics/budget.rs)
`qlow_bl`, `qlow_conv`, `qlow_ls`, `qlev1_*`, `wlow`; `conv_active`, `conv_skipped`,
`conv_top_p`, `pblh`. Budget layer: the levels σ_k ≥ 0.87 (L26: σ 0.9295, 0.9705, 0.9925,
the layer σ 0.903–1). Terms [mm/day into the layer]: surface evaporation E; boundary-layer
mixing = qlow_bl − E (the K-profile solve holds both); convection = qlow_conv − qlow_bl
(dry adjustment changes no q); large-scale condensation = qlow_ls − qlow_conv; storage
dW/dt from the monthly W (centred over the season: month after − month before, / 4 months);
dynamics (advection, hyperdiffusion and the fixer) = dW/dt − physics, a residual. The
climatology samples every 6 h, so the frequencies are those of 4 instants a day.

Per run also: low cover (raw, maximum-random below 680 hPa) by band against GOCCP and the
online cover against CERES (tools/cloud_split.py), criteria 1–2, tropical net CRE, Hadley
strength (tools/scorecard.py streamfunction, max |ψ| within 35° at σ = 0.7) and the P5
annual precipitation map pattern correlation against GPCP (scorecard's `precip_map_corr`).
"""

import argparse
import json
import pathlib
import sys

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import cloud_split  # noqa: E402
import hadley_ref  # noqa: E402
import scorecard  # noqa: E402

RAW = ROOT / "reference/raw"
LV = 2.5e6
BANDS = [("10°S–10°N", -10, 10), ("10–30°N", 10, 30), ("10–30°S", -30, -10),
         ("40–60°N", 40, 60), ("40–60°S", -60, -40)]
SEASONS = {"ann": list(range(1, 13)), "DJF": [12, 1, 2], "JJA": [6, 7, 8]}
# budget regions: (name, lat lo, lat hi, season)
REGIONS = [("40–60°N DJF", 40, 60, "DJF"), ("40–60°S JJA", -60, -40, "JJA"),
           ("trades 10–25°N ann", 10, 25, "ann"), ("trades 10–25°S ann", -25, -10, "ann"),
           ("tropics 10°S–10°N ann", -10, 10, "ann")]


def band_avg(x, lat, w, mask, lo, hi):
    """Area mean of map x[lat, lon] over the cells with mask and lo ≤ φ ≤ hi (NaN skipped)."""
    sel = ((lat >= lo) & (lat <= hi))[:, None] & mask & np.isfinite(x)
    ww = np.broadcast_to(w[:, None], x.shape)
    return float(np.sum(np.where(sel, x * ww, 0.0)) / np.sum(np.where(sel, ww, 0.0)))


def season_mean(mon, months):
    return np.nanmean(np.stack([mon[m - 1] for m in months]), axis=0)


def oisst_ocean(lat, lon):
    import netCDF4

    d = netCDF4.Dataset(RAW / "oisst_lsmask/lsmask.nc")
    m = np.asarray(d["mask"][:], float).squeeze()
    la, lo = np.asarray(d["lat"][:], float), np.asarray(d["lon"][:], float)
    j = np.abs(la[None, :] - lat[:, None]).argmin(axis=1)
    i = np.abs(((lo[None, :] - lon[:, None]) + 180) % 360 - 180).argmin(axis=1)
    return m[np.ix_(j, i)] > 0.5


def nc_monthly(path, var, scale=1.0):
    import netCDF4

    d = netCDF4.Dataset(path)
    x = np.ma.filled(d[var][:].astype(float), np.nan) * scale
    return x, np.asarray(d["lat"][:], float), np.asarray(d["lon"][:], float)


def icoads_rh():
    import netCDF4

    d = netCDF4.Dataset(RAW / "icoads_rhum/rhum.mean.nc")
    t = d["time"]
    dates = netCDF4.num2date(t[:], t.units)
    yr = np.array([x.year for x in dates])
    mo = np.array([x.month for x in dates])
    sel = (yr >= 1991) & (yr <= 2020)
    v = np.ma.filled(d["rhum"][np.where(sel)[0]].astype(float), np.nan) / 100.0
    mon = np.stack([np.nanmean(v[mo[sel] == m], axis=0) for m in range(1, 13)])
    return mon, np.asarray(d["lat"][:], float), np.asarray(d["lon"][:], float)


def observations():
    out = {}
    import warnings

    warnings.simplefilter("ignore", RuntimeWarning)

    def bands(mon, lat, lon, ocean):
        w = np.cos(np.radians(lat))
        return {s: {b: band_avg(season_mean(mon, ms), lat, w, ocean, lo, hi)
                    for b, lo, hi in BANDS} for s, ms in SEASONS.items()}

    mon, la, lo = icoads_rh()
    out["rh_icoads"] = bands(mon, la, lo, np.ones((len(la), len(lo)), bool))
    mon, la, lo = nc_monthly(RAW / "ncep_rhum/rhum.sig995.mon.ltm.1991-2020.nc", "rhum", 0.01)
    out["rh_ncep995"] = bands(mon, la, lo, oisst_ocean(la, lo))
    mon, la, lo = nc_monthly(RAW / "ncep_lhtfl/lhtfl.sfc.mon.ltm.1991-2020.nc", "lhtfl",
                             86400.0 / LV)
    oc = oisst_ocean(la, lo)
    out["evap_ncep"] = bands(mon, la, lo, oc)
    pr, _, _ = nc_monthly(RAW / "ncep_prate/prate.sfc.mon.ltm.1991-2020.nc", "prate", 86400.0)
    cp, _, _ = nc_monthly(RAW / "ncep_cprat/cprat.sfc.mon.ltm.1991-2020.nc", "cprat", 86400.0)
    out["precip_ncep"] = bands(pr, la, lo, oc)
    out["conv_share_ncep"] = {s: {b: bands(cp, la, lo, oc)[s][b] / out["precip_ncep"][s][b]
                                  for b, _, _ in BANDS} for s in SEASONS}
    w = np.cos(np.radians(la))
    w2 = np.broadcast_to(w[:, None], pr.shape[1:])
    out["global_precip_ncep"] = float(np.sum(pr.mean(0) * w2) / np.sum(w2))
    mon, la, lo = nc_monthly(RAW / "gpcp/precip.mon.ltm.1991-2020.nc", "precip")
    out["precip_gpcp"] = bands(mon, la, lo, oisst_ocean(la, lo))
    w2 = np.broadcast_to(np.cos(np.radians(la))[:, None], mon.shape[1:])
    out["global_precip_gpcp"] = float(np.nansum(mon.mean(0) * w2) / np.sum(
        np.where(np.isfinite(mon.mean(0)), w2, 0)))
    return out


def model(run):
    run = pathlib.Path(run)
    meta, c, info, diag = scorecard.load(run)
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlat, nlon = len(lat), len(meta["grid"]["lon"])
    w = np.asarray(meta["grid"]["weights"], float)
    ref = ROOT / "reference/derived" / f"t{meta['truncation']}"
    ocean = np.load(ref / "land.npy").astype(float).reshape(nlat, nlon) < 0.5
    allc = np.ones_like(ocean)
    M = lambda k: np.load(c / f"monthly_{k}.npy").astype(float).reshape(12, nlat, nlon)  # noqa
    r = {"run": str(run)}
    day = 86400.0
    mon = {k: M(k) for k in ("rh_low", "evap", "precip", "precip_conv", "precip_ls",
                             "qlow_bl", "qlow_conv", "qlow_ls", "qlev1_bl", "qlev1_conv",
                             "qlev1_ls", "wlow", "conv_active", "conv_skipped", "conv_top_p",
                             "pblh", "cape")}

    def bands(x, mask, scale=1.0):
        return {s: {b: band_avg(season_mean(x, ms) * scale, lat, w, mask, lo, hi)
                    for b, lo, hi in BANDS} for s, ms in SEASONS.items()}

    r["rh_low"] = bands(mon["rh_low"], ocean)
    r["evap"] = bands(mon["evap"], ocean, day)
    r["precip"] = bands(mon["precip"], ocean, day)
    r["precip_all"] = bands(mon["precip"], allc, day)
    pc, pp = bands(mon["precip_conv"], ocean, day), r["precip"]
    r["conv_share"] = {s: {b: pc[s][b] / pp[s][b] for b in pp[s]} for s in pp}
    w2 = np.broadcast_to(w[:, None], (nlat, nlon))
    g = lambda x: float(np.sum(x * w2) / np.sum(w2))  # noqa: E731
    r["global"] = {"precip": g(mon["precip"].mean(0)) * day,
                   "evap": g(mon["evap"].mean(0)) * day,
                   "precip_conv": g(mon["precip_conv"].mean(0)) * day,
                   "precip_ls": g(mon["precip_ls"].mean(0)) * day,
                   "evap_ocean": band_avg(mon["evap"].mean(0), lat, w, ocean, -90, 90) * day,
                   "rh_low_ocean": band_avg(mon["rh_low"].mean(0), lat, w, ocean, -90, 90)}
    # RH distribution of the lowest level over the ocean, 5-day snapshots (spread)
    # budget
    r["budget"] = {}
    for name, lo, hi, s in REGIONS:
        ms = SEASONS[s]
        a = lambda k: band_avg(season_mean(mon[k], ms), lat, w, ocean, lo, hi)  # noqa: E731
        e = a("evap") * day
        bl, cv, ls = a("qlow_bl") * day, a("qlow_conv") * day, a("qlow_ls") * day
        if s == "ann":
            dw = 0.0
        else:
            before, after = (ms[0] - 2) % 12, ms[-1] % 12  # month before / after (0-based)
            wb = band_avg(mon["wlow"][before], lat, w, ocean, lo, hi)
            wa = band_avg(mon["wlow"][after], lat, w, ocean, lo, hi)
            dw = (wa - wb) / (4 * 365.25 / 12)  # kg m-2 per day
        act = a("conv_active")
        top = a("conv_top_p") / act if act > 0 else float("nan")
        l1_conv = (a("qlev1_conv") - a("qlev1_bl")) * day
        r["budget"][name] = {
            "evaporation": e, "bl_mixing": bl - e, "convection": cv - bl,
            "large_scale": ls - cv, "physics": ls, "storage": dw, "dynamics": dw - ls,
            "lowest_level_convection": l1_conv,
            "conv_active": act, "conv_skipped": a("conv_skipped"),
            "conv_top_hPa": top / 100, "pblh_m": a("pblh"), "cape": a("cape"),
            "rh_low": a("rh_low"), "wlow": a("wlow"),
            "precip": a("precip") * day, "conv_share": a("precip_conv") / a("precip"),
        }
    # climate, low cover, Hadley, pattern correlation
    cs = cloud_split.analyse(run, cloud_split.rhc_of(run))
    r["climate"] = cs["climate"]
    r["low_cover"] = {b: cs["bands"]["low"]["ann"][b] for b in ("32–57°N", "45–60°S",
                                                                  "60–77°S", "30–45°N")}
    r["clt_online"] = cs["bands_clt_online"]
    r["ceres_clt"] = cs["ceres_clt"]
    sh = np.asarray(meta["config"]["sigma_half"]) if "sigma_half" in meta["config"] else None
    if sh is None:
        sig = np.asarray(meta["sigma"])
        sh = np.concatenate([[0.0], 0.5 * (sig[1:] + sig[:-1]), [1.0]])
    v = np.load(c / "zonal_v.npy")
    ps = np.load(c / "zonal_ps.npy")
    latr = np.asarray(meta["grid"]["lat"])
    psi = scorecard.streamfunction(v, sh, latr, ps, meta["planet"]["radius"],
                                   meta["planet"]["gravity"])
    r["hadley"] = scorecard.hadley(psi, sh, lat)
    k7 = int(np.argmin(np.abs(sh - 0.7)))
    r["hadley_cells"] = {"ann": hadley_ref.cells(psi[k7], lat)}
    # seasonal cells from the 5-day v snapshots (zonal means, year 2), if saved
    vs, pss = cloud_split.snaps(run, "v"), cloud_split.snaps(run, "ps")
    if vs:
        dt, year = float(meta["config"]["dt"]), 365.25 * 86400.0
        start = float(meta["config"].get("climatology", {}).get("start", 0.0))
        nl = len(meta["sigma"])
        for sname in ("DJF", "JJA"):
            ms = SEASONS[sname]
            vz, pz = [], []
            for st, f in vs.items():
                if st * dt < start or st not in pss:
                    continue
                if int(((st * dt) % year) / (year / 12.0)) + 1 not in ms:
                    continue
                vz.append(np.load(f).astype(float).reshape(nl, nlat, nlon).mean(axis=2))
                pz.append(np.load(pss[st]).astype(float).reshape(nlat, nlon).mean(axis=1))
            if vz:
                ps_ = scorecard.streamfunction(np.mean(vz, 0), sh, latr, np.mean(pz, 0),
                                               meta["planet"]["radius"],
                                               meta["planet"]["gravity"])
                r["hadley_cells"][sname] = hadley_ref.cells(ps_[k7], lat)
    wts = scorecard.area_weights(latr, nlon)
    pa = mon["precip"].mean(0) * day
    ra = np.load(ref / "pr.npy").astype(float).reshape(-1, nlat, nlon).mean(0)
    r["precip_map_corr"] = scorecard.wcorr(pa, ra, wts)
    r["zonal_precip_corr"] = scorecard.wcorr(pa.mean(1), ra.mean(1), wts[:, 0])
    r["global_precip_ref"] = scorecard.gmean(ra, wts)
    return r


def print_report(obs, rs, labels):
    print("## Step 1: observations against the model by band (ocean cells)\n")
    print(f"Global precipitation: GPCP {obs['global_precip_gpcp']:.2f} mm/day "
          f"(NCEP {obs['global_precip_ncep']:.2f}); model: "
          + ", ".join(f"{l} {r['global']['precip']:.2f}" for l, r in zip(labels, rs)) + "\n")
    rows = [("near-surface RH", "rh_icoads", "rh_ncep995", "rh_low", "{:.3f}"),
            ("evaporation mm/day", "evap_ncep", None, "evap", "{:.2f}"),
            ("precipitation mm/day", "precip_gpcp", "precip_ncep", "precip", "{:.2f}"),
            ("convective share", "conv_share_ncep", None, "conv_share", "{:.2f}")]
    for title, o1, o2, mk, f in rows:
        print(f"### {title} (obs: {o1}{', ' + o2 if o2 else ''}; model {mk})\n")
        print("| band | season | " + o1 + (" | " + o2 if o2 else "") + " | "
              + " | ".join(labels) + " |")
        print("|---|---|---|" + ("---|" if o2 else "") + "---|" * len(labels))
        for b, _, _ in BANDS:
            for s in SEASONS:
                cells = [f.format(obs[o1][s][b])] + ([f.format(obs[o2][s][b])] if o2 else [])
                cells += [f.format(r[mk][s][b]) for r in rs]
                print(f"| {b} | {s} | " + " | ".join(cells) + " |")
        print()
    print("## Step 2: moisture budget of the levels σ ≥ 0.87 (ocean, mm/day into the layer)\n")
    keys = ["evaporation", "bl_mixing", "convection", "large_scale", "storage", "dynamics",
            "lowest_level_convection", "conv_active", "conv_skipped", "conv_top_hPa", "pblh_m",
            "rh_low", "wlow", "precip", "conv_share"]
    for l, r in zip(labels, rs):
        print(f"### {l}\n")
        print("| term | " + " | ".join(n for n, *_ in REGIONS) + " |")
        print("|---|" + "---|" * len(REGIONS))
        for k in keys:
            print(f"| {k} | " + " | ".join(f"{r['budget'][n][k]:.3f}" for n, *_ in REGIONS)
                  + " |")
        print()
    print("## Step 3: toggle table\n")
    print("| run | albedo | OLR | net TOA | SW CRE | LW CRE | cover | trop. net CRE | Hadley N/S "
          "(1e10 kg/s) | P map corr | P global | E ocean | RH low ocean | low 32–57°N | "
          "low 45–60°S | clt−CERES 32–57°N | clt−CERES 45–60°S |")
    print("|---" * 17 + "|")
    for l, r in zip(labels, rs):
        c = r["climate"]
        h = r["hadley"]
        print(f"| {l} | {c['albedo']:.3f} | {c['olr']:.1f} | {c['net']:+.2f} | {c['sw_cre']:.1f}"
              f" | {c['lw_cre']:.1f} | {c['cover']:.3f} | {c['trop_net_cre']:.1f} | "
              f"{h['NH']['max_psi_1e10_kg_s']:.2f}/{h['SH']['max_psi_1e10_kg_s']:.2f} | "
              f"{r['precip_map_corr']:.3f} | {r['global']['precip']:.2f} | "
              f"{r['global']['evap_ocean']:.2f} | {r['global']['rh_low_ocean']:.3f} | "
              f"{r['low_cover']['32–57°N']:.3f} | {r['low_cover']['45–60°S']:.3f} | "
              f"{r['clt_online']['32–57°N'] - r['ceres_clt']['32–57°N']:+.3f} | "
              f"{r['clt_online']['45–60°S'] - r['ceres_clt']['45–60°S']:+.3f} |")
    print()


def print_screen(obs, rs, labels, href):
    """Screen table and the step 4 bar (owner decision after the humidity diagnosis,
    2026-11-26); the first run is s1. Bar, at T21: winter ocean RH at 40–60° (DJF N, JJA S)
    within 0.05 of ICOADS in both hemispheres; global P within 10 % of GPCP; precipitation map
    correlation ≥ s1 − 0.01; each annual Hadley cell (scorecard metric, max |ψ| within 35° at
    σ 0.7, against NCEP at 700 hPa) closer to NCEP than in s1 or within 10 % of its s1 value.
    Stability (both years run) is checked from the run log, not here."""
    gp = obs["global_precip_gpcp"]
    rn, rs_ = obs["rh_icoads"]["DJF"]["40–60°N"], obs["rh_icoads"]["JJA"]["40–60°S"]
    hn, hs = href["ann"]["NH"]["psi700_1e10_kg_s"], href["ann"]["SH"]["psi700_1e10_kg_s"]
    s1 = rs[0]
    print(f"## Screen and step 4 bar (ICOADS winter RH 40–60°N DJF {rn:.3f}, 40–60°S JJA "
          f"{rs_:.3f}; GPCP {gp:.2f} mm/day; NCEP Hadley annual N {hn:.2f}, S {hs:.2f} "
          f"1e10 kg/s)\n")
    print("| run | RH N DJF | RH S JJA | P global | P corr | Hadley N cell | Hadley S cell | "
          "max |ψ| NH / SH | "
          "conv active 40–60°N DJF / S JJA / tropics | conv share same | E 40–60°N DJF / S JJA |"
          " bar: RH | P | corr | Hadley | all |")
    print("|---" * 16 + "|")
    out = []
    for l, r in zip(labels, rs):
        a, b = r["rh_low"]["DJF"]["40–60°N"], r["rh_low"]["JJA"]["40–60°S"]
        P, c = r["global"]["precip"], r["precip_map_corr"]
        # each cell by sign (hadley_ref.cells): the per-hemisphere max |ψ| can pick the
        # southern cell where it reaches north of the equator (NCEP: both give 6.24 / 8.16)
        cell = lambda x: (x["hadley_cells"]["ann"]["pos_1e10_kg_s"],  # noqa: E731
                          x["hadley_cells"]["ann"]["neg_1e10_kg_s"])
        n, sh = cell(r)
        n1, s1h = cell(s1)
        nh, shh = (r["hadley"][h]["max_psi_1e10_kg_s"] for h in ("NH", "SH"))
        ok_rh = abs(a - rn) <= 0.05 and abs(b - rs_) <= 0.05
        ok_p = abs(P - gp) <= 0.10 * gp
        ok_c = c >= s1["precip_map_corr"] - 0.01
        okh = lambda x, x1, ref: abs(x - ref) < abs(x1 - ref) or abs(x - x1) <= 0.10 * x1  # noqa
        ok_h = okh(n, n1, hn) and okh(sh, s1h, hs)
        B = r["budget"]
        f = lambda k: " / ".join(f"{B[x][k]:.2f}" for x in ("40–60°N DJF", "40–60°S JJA",  # noqa
                                                            "tropics 10°S–10°N ann"))
        yn = lambda x: "yes" if x else "**no**"  # noqa: E731
        allok = ok_rh and ok_p and ok_c and ok_h
        out.append({"run": l, "rh_ok": ok_rh, "p_ok": ok_p, "corr_ok": ok_c, "hadley_ok": ok_h,
                    "all": allok})
        print(f"| {l} | {a:.3f} | {b:.3f} | {P:.2f} | {c:.3f} | {n:.2f} | {sh:.2f} | "
              f"{nh:.2f} / {shh:.2f} | {f('conv_active')} | {f('conv_share')} | {B['40–60°N DJF']['evaporation']:.2f}"
              f" / {B['40–60°S JJA']['evaporation']:.2f} | {yn(ok_rh)} | {yn(ok_p)} | "
              f"{yn(ok_c)} | {yn(ok_h)} | {yn(allok)} |")
    print()
    print("Seasonal Hadley cells by sign (northern + / southern −, 1e10 kg/s, σ 0.7; NCEP "
          + ", ".join(f"{k} {v['cells']['pos_1e10_kg_s']:.2f} / {v['cells']['neg_1e10_kg_s']:.2f}"
                      for k, v in href.items()) + ")\n")
    print("| run | " + " | ".join(href) + " |")
    print("|---" * (len(href) + 1) + "|")
    for l, r in zip(labels, rs):
        hc = r["hadley_cells"]
        print(f"| {l} | " + " | ".join(
            f"{hc[k]['pos_1e10_kg_s']:.2f} / {hc[k]['neg_1e10_kg_s']:.2f}" if k in hc else "—"
            for k in href) + " |")
    print()
    return out


def fig(obs, rs, labels, path):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(1, 3, figsize=(15, 4.5))
    bn = [b for b, _, _ in BANDS]
    x = np.arange(len(bn))
    for a, (title, ok, mk) in zip(ax, [("near-surface RH, ocean (40–60°: winter; else annual)", "rh_icoads",
                                        "rh_low"),
                                       ("evaporation, ocean, annual [mm/day]", "evap_ncep",
                                        "evap"),
                                       ("precipitation, ocean, annual [mm/day]", "precip_gpcp",
                                        "precip")]):
        def val(d, b):
            if mk == "rh_low":
                return d["DJF" if b.endswith("N") and b.startswith("40") else
                         "JJA" if b.startswith("40") else "ann"][b]
            return d["ann"][b]
        a.plot(x, [val(obs[ok], b) for b in bn], "ko-", lw=2, label=ok)
        for l, r in zip(labels, rs):
            a.plot(x, [val(r[mk], b) for b in bn], "o--", label=l)
        a.set_xticks(x, bn, rotation=20)
        a.set_title(title)
        a.grid(alpha=0.3)
    ax[0].legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="+")
    ap.add_argument("--json")
    ap.add_argument("--fig")
    ap.add_argument("--screen", action="store_true",
                    help="screen table and the step 4 bar; the first run is the reference s1")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    obs = observations()
    rs = [model(r) for r in a.runs]
    print_report(obs, rs, labels)
    bar = None
    if a.screen:
        href = hadley_ref.reference()
        bar = print_screen(obs, rs, labels, href)
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps({"obs": obs, "runs": rs, "labels": labels,
                                                    "bar": bar}, indent=1, default=float))
    if a.fig:
        fig(obs, rs, labels, a.fig)


if __name__ == "__main__":
    main()
