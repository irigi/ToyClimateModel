"""Q13 owner decision after the convection screen (2026-11-26), step 1 (existing output) and
the per-run profiles of step 2.

    python tools/q13_step1.py precip <run>... --labels ... [--fig-diff png] [--fig-obs png]
    python tools/q13_step1.py spread <multi-year AMIP run>
    python tools/q13_step1.py profile <run>... --labels ...
    python tools/q13_step1.py pblh <run>... --labels ...
    python tools/q13_step1.py cloud <run>... --labels ...
    (each: [--json out.json])

precip: JJA and DJF precipitation of each run minus the first (s1) and minus GPCP v2.3
(`reference/derived/t<N>/pr.npy`); for 10–30°N in JJA the band-mean change against s1
split into land and ocean cells (contributions that add up to the band change), into
convective and large-scale, and into evaporation E and convergence P − E (the time-mean
vapour budget: P − E = moisture convergence plus storage, the storage being negligible for a
season mean); the change by longitude sector, and the pattern correlation of the JJA change
maps between runs over 0–40°N.

spread: year-by-year annual precipitation maps of a multi-year run from its 10-day `precip`
snapshots (instantaneous rates at the output step), pattern correlation against GPCP for each
year after the first; the sampling noise of 36 instants per year is estimated from the
correlation of odd against even snapshots within each year.

profile: RH and q at each model level up to 600 hPa (5-day snapshots of `rh`, `q`, `ps` in
year 2, ocean cells) for 40–60°N DJF, 40–60°S JJA and the trade belts 10–25°N / 10–25°S
(annual), against NCEP/NCAR reanalysis 1 RH and q on pressure levels (Kalnay et al. 1996;
`rhum` and `shum` long-term monthly means 1991–2020, derived pressure-level set, fetched
2026-11-26 from NOAA PSL), interpolated in ln p to the band-mean pressure of each model
level; ICOADS near-surface RH as in tools/bl_humidity.py. NCEP's RH is with respect to
liquid water above 0 °C and its own blend below; the model `rh` field is the one the cloud
scheme uses.

pblh: boundary-layer depth (the K-profile depth, critical bulk Richardson number from the
lowest level; physics/boundary_layer.rs) by band and season over the ocean: mean of the
monthly climatology and, where `pblh` snapshots exist, the 10/50/90 % quantiles of the
instantaneous values; otherwise the quantiles of the monthly-mean cell values.

cloud: cloud fraction `cl` by level at 40–60°N DJF and 40–60°S JJA over the ocean (5-day
snapshots, year 2).
"""

import argparse
import json
import pathlib
import sys

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import bl_humidity as blh  # noqa: E402

RAW = ROOT / "reference/raw"
DAY = 86400.0
SEAS = {"DJF": [12, 1, 2], "JJA": [6, 7, 8], "ann": list(range(1, 13))}
PROFILE_REGIONS = [("40–60°N DJF", 40, 60, "DJF"), ("40–60°S JJA", -60, -40, "JJA"),
                   ("trades 10–25°N ann", 10, 25, "ann"),
                   ("trades 10–25°S ann", -25, -10, "ann")]
SECTORS = [("Africa 20°W–60°E", -20, 60), ("S Asia 60–100°E", 60, 100),
           ("W Pacific 100–160°E", 100, 160), ("C/E Pacific 160°E–100°W", 160, 260),
           ("Americas–Atlantic 100–20°W", 260, 340)]


class Run:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        m = json.loads((self.path / "meta.json").read_text())
        self.meta = m
        g = m["grid"]
        self.nlat, self.nlon = len(g["mu"]), 2 * len(g["mu"])
        self.lat = np.degrees(np.arcsin(np.asarray(g["mu"], float)))
        self.lon = np.arange(self.nlon) * 360.0 / self.nlon
        self.w = np.asarray(g["weights"], float)
        self.sigma = np.asarray(m["sigma"], float)
        self.dt = float(m["config"]["dt"])
        self.year = float(m["calendar"]["year"])
        ref = ROOT / "reference/derived" / f"t{m['truncation']}"
        self.ref = ref
        self.land = np.load(ref / "land.npy").astype(float).reshape(self.nlat, self.nlon)
        self.ocean = self.land < 0.5

    def monthly(self, k):
        return np.load(self.path / "climatology" / f"monthly_{k}.npy").astype(float).reshape(
            12, self.nlat, self.nlon)

    def snaps(self, name):
        """[(time s, file)] of the snapshots of one field."""
        out = []
        for f in sorted((self.path / "fields").glob(f"{name}_*.npy")):
            st = f.stem[len(name) + 1:]
            if st.isdigit():
                out.append((int(st) * self.dt, f))
        return out

    def month_of(self, t):
        return int((t % self.year) / (self.year / 12.0)) + 1

    def snap_mean(self, name, months, start, nlev=None):
        acc, n = None, 0
        for t, f in self.snaps(name):
            if t < start or self.month_of(t) not in months:
                continue
            x = np.load(f).astype(float)
            x = x.reshape(-1, self.nlat, self.nlon)
            acc = x if acc is None else acc + x
            n += 1
        return None if acc is None else acc / n, n


def wmean(x, w, sel):
    ww = np.broadcast_to(w[:, None], x.shape)
    s = sel & np.isfinite(x)
    return float(np.sum(np.where(s, x * ww, 0)) / np.sum(np.where(s, ww, 0)))


def gpcp(run):
    return np.load(run.ref / "pr.npy").astype(float).reshape(-1, run.nlat, run.nlon)


def seasonal(mon, s):
    return np.mean([mon[m - 1] for m in SEAS[s]], axis=0)


# ---------------------------------------------------------------- precip
def cmd_precip(runs, labels, a):
    r0 = runs[0]
    obs = gpcp(r0)
    lat, w = r0.lat, r0.w
    out = {"labels": labels}
    fields = {}
    for run, l in zip(runs, labels):
        f = {}
        for k in ("precip", "precip_conv", "precip_ls", "evap"):
            mon = run.monthly(k) * DAY
            f[k] = {s: seasonal(mon, s) for s in SEAS}
        fields[l] = f
    band = (lat >= 10) & (lat <= 30)
    allc = np.ones((r0.nlat, r0.nlon), bool)
    bsel = band[:, None] & allc
    # absolute band means (JJA) with GPCP, land and ocean
    print("## 10–30°N JJA precipitation, band means [mm/day]\n")
    print("| run | P all | P land | P ocean | P conv | P ls | E | P − E | P − E land | "
          "P − E ocean |")
    print("|---" * 10 + "|")
    gj = seasonal(obs, "JJA")
    print(f"| GPCP | {wmean(gj, w, bsel):.2f} | {wmean(gj, w, bsel & ~r0.ocean):.2f} | "
          f"{wmean(gj, w, bsel & r0.ocean):.2f} | | | | | | |")
    out["abs_10_30N_JJA"] = {}
    for l in labels:
        f = fields[l]
        P, E = f["precip"]["JJA"], f["evap"]["JJA"]
        row = {"P": wmean(P, w, bsel), "P_land": wmean(P, w, bsel & ~r0.ocean),
               "P_ocean": wmean(P, w, bsel & r0.ocean),
               "P_conv": wmean(f["precip_conv"]["JJA"], w, bsel),
               "P_ls": wmean(f["precip_ls"]["JJA"], w, bsel), "E": wmean(E, w, bsel),
               "PmE": wmean(P - E, w, bsel), "PmE_land": wmean(P - E, w, bsel & ~r0.ocean),
               "PmE_ocean": wmean(P - E, w, bsel & r0.ocean)}
        out["abs_10_30N_JJA"][l] = row
        print(f"| {l} | " + " | ".join(f"{row[k]:.2f}" for k in (
            "P", "P_land", "P_ocean", "P_conv", "P_ls", "E", "PmE", "PmE_land", "PmE_ocean"))
            + " |")
    # change against s1, contributions
    ww = np.broadcast_to(w[:, None], (r0.nlat, r0.nlon))
    tot = np.sum(np.where(bsel, ww, 0))

    def contrib(x, mask):
        return float(np.sum(np.where(bsel & mask, x * ww, 0)) / tot)

    print("\n## 10–30°N JJA change against s1 [mm/day, contributions to the band mean]\n")
    print("| run | ΔP | of which land | ocean | ΔP conv | ΔP ls | ΔE | Δ(P − E) | "
          + " | ".join(s[0] for s in SECTORS) + " |")
    print("|---" * (8 + len(SECTORS)) + "|")
    out["delta_10_30N_JJA"] = {}
    f1 = fields[labels[0]]
    for l in labels[1:]:
        f = fields[l]
        dP = f["precip"]["JJA"] - f1["precip"]["JJA"]
        dE = f["evap"]["JJA"] - f1["evap"]["JJA"]
        dc = f["precip_conv"]["JJA"] - f1["precip_conv"]["JJA"]
        dl = f["precip_ls"]["JJA"] - f1["precip_ls"]["JJA"]
        row = {"dP": contrib(dP, allc), "land": contrib(dP, ~r0.ocean),
               "ocean": contrib(dP, r0.ocean), "conv": contrib(dc, allc),
               "ls": contrib(dl, allc), "dE": contrib(dE, allc),
               "dPmE": contrib(dP - dE, allc)}
        lon = r0.lon
        for name, lo, hi in SECTORS:
            lo_, hi_ = lo % 360, hi % 360
            m = ((lon >= lo_) & (lon < hi_)) if lo_ < hi_ else ((lon >= lo_) | (lon < hi_))
            row[name] = contrib(dP, np.broadcast_to(m[None, :], dP.shape))
        out["delta_10_30N_JJA"][l] = row
        print(f"| {l} | " + " | ".join(f"{row[k]:+.2f}" for k in (
            "dP", "land", "ocean", "conv", "ls", "dE", "dPmE")) + " | "
            + " | ".join(f"{row[s[0]]:+.2f}" for s in SECTORS) + " |")
    # pattern similarity of the JJA change maps, 0–40°N
    sel = ((lat >= 0) & (lat <= 40))[:, None] & allc
    print("\n## Pattern correlation of the JJA change maps (run − s1) over 0–40°N\n")
    d = {l: fields[l]["precip"]["JJA"] - f1["precip"]["JJA"] for l in labels[1:]}
    print("| | " + " | ".join(labels[1:]) + " |")
    print("|---" * len(labels) + "|")
    out["delta_corr"] = {}
    for la in labels[1:]:
        cs = []
        for lb in labels[1:]:
            c = wcorr_sel(d[la], d[lb], w, sel)
            cs.append(c)
            out["delta_corr"][f"{la}-{lb}"] = c
        print(f"| {la} | " + " | ".join(f"{c:.2f}" for c in cs) + " |")
    # pattern correlation against GPCP by season
    print("\n## Pattern correlation against GPCP (global, all cells)\n")
    print("| run | ann | DJF | JJA | JJA 0–40°N |")
    print("|---|---|---|---|---|")
    out["corr"] = {}
    for l in labels:
        c = {s: wcorr_sel(fields[l]["precip"][s], seasonal(obs, s), w, allc) for s in SEAS}
        c["JJA 0–40N"] = wcorr_sel(fields[l]["precip"]["JJA"], gj, w, sel)
        out["corr"][l] = c
        print(f"| {l} | {c['ann']:.3f} | {c['DJF']:.3f} | {c['JJA']:.3f} | "
              f"{c['JJA 0–40N']:.3f} |")
    if a.fig_diff or a.fig_obs:
        figs(runs, labels, fields, obs, a)
    return out


def wcorr_sel(x, y, w, sel):
    ww = np.broadcast_to(w[:, None], x.shape)[sel]
    x, y = x[sel], y[sel]
    xm, ym = np.sum(ww * x) / ww.sum(), np.sum(ww * y) / ww.sum()
    return float(np.sum(ww * (x - xm) * (y - ym))
                 / np.sqrt(np.sum(ww * (x - xm) ** 2) * np.sum(ww * (y - ym) ** 2)))


def figs(runs, labels, fields, obs, a):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    r0 = runs[0]
    lon = np.concatenate([r0.lon, [360.0]])

    def cyc(x):
        return np.concatenate([x, x[:, :1]], axis=1)

    def grid(rows, title, path):
        fig, ax = plt.subplots(len(rows), 2, figsize=(11, 1.9 * len(rows) + 0.6),
                               sharex=True, sharey=True, squeeze=False)
        lv = np.array([-6, -4, -3, -2, -1, -0.5, 0.5, 1, 2, 3, 4, 6])
        for i, (name, mp) in enumerate(rows):
            for j, s in enumerate(("JJA", "DJF")):
                cs = ax[i, j].contourf(lon, r0.lat, cyc(mp[s]), levels=lv, cmap="BrBG",
                                       extend="both")
                ax[i, j].contour(lon, r0.lat, cyc(r0.land), levels=[0.5], colors="k",
                                 linewidths=0.5)
                ax[i, j].set_title(f"{name} {s}", fontsize=9)
                ax[i, j].set_ylim(-60, 60)
                ax[i, j].axhline(10, color="grey", lw=0.4, ls=":")
                ax[i, j].axhline(30, color="grey", lw=0.4, ls=":")
        fig.colorbar(cs, ax=ax, shrink=0.6, label="mm/day")
        fig.suptitle(title)
        fig.savefig(path, dpi=110)
        plt.close(fig)

    f1 = fields[labels[0]]
    if a.fig_diff:
        rows = [(f"{l} − {labels[0]}", {s: fields[l]["precip"][s] - f1["precip"][s]
                                         for s in ("JJA", "DJF")}) for l in labels[1:]]
        grid(rows, "Precipitation change against s1 (T21, year 2)", a.fig_diff)
    if a.fig_obs:
        rows = [(f"{l} − GPCP", {s: fields[l]["precip"][s] - seasonal(obs, s)
                                 for s in ("JJA", "DJF")}) for l in labels]
        grid(rows, "Precipitation minus GPCP v2.3 (T21, year 2)", a.fig_obs)


# ---------------------------------------------------------------- spread
def cmd_spread(runs, labels, a):
    run = runs[0]
    obs = gpcp(run).mean(0)
    allc = np.ones((run.nlat, run.nlon), bool)
    by_year = {}
    for t, f in run.snaps("precip"):
        y = int(t // run.year)
        by_year.setdefault(y, []).append(np.load(f).astype(float).reshape(run.nlat, run.nlon))
    clim = run.monthly("precip").mean(0)
    out = {"clim_corr": wcorr_sel(clim, obs, run.w, allc), "years": {}}
    allsn = []
    print(f"## Year-to-year pattern correlation against GPCP ({run.path}, annual maps "
          "from the 10-day precip snapshots)\n")
    print("| year | n | corr | odd vs even half: corr(odd, GPCP) / corr(even, GPCP) |")
    print("|---|---|---|---|")
    for y in sorted(by_year):
        if y == 0 or len(by_year[y]) < 30:
            continue
        xs = by_year[y]
        allsn += xs
        m = np.mean(xs, 0)
        c = wcorr_sel(m, obs, run.w, allc)
        co = wcorr_sel(np.mean(xs[0::2], 0), obs, run.w, allc)
        ce = wcorr_sel(np.mean(xs[1::2], 0), obs, run.w, allc)
        out["years"][y + 1] = {"n": len(xs), "corr": c, "odd": co, "even": ce}
        print(f"| {y + 1} | {len(xs)} | {c:.3f} | {co:.3f} / {ce:.3f} |")
    cs = np.array([v["corr"] for v in out["years"].values()])
    halves = np.array([abs(v["odd"] - v["even"]) for v in out["years"].values()])
    out["mean"], out["std"], out["range"] = float(cs.mean()), float(cs.std(ddof=1)), \
        float(cs.max() - cs.min())
    out["snap_all_corr"] = wcorr_sel(np.mean(allsn, 0), obs, run.w, allc)
    out["half_diff_mean"] = float(halves.mean())
    # sampling-noise correction: deseasonalised within-year variance of the 10-day instants
    # (each snapshot minus the run's monthly climatology of its month) / n is the noise
    # variance of a year's mean; the year-to-year anomaly of each cell is shrunk by
    # sqrt(1 - noise / total) and added to the 6-hourly climatology (an estimate of what a
    # 6-hourly-sampled single year would give)
    mon = run.monthly("precip")
    ys = [y for y in sorted(by_year) if y > 0 and len(by_year[y]) >= 30]
    Y = np.array([np.mean(by_year[y], 0) for y in ys])
    noise = []
    for y in ys:
        ts = [t for t, _ in run.snaps("precip") if int(t // run.year) == y]
        dev = [x - mon[run.month_of(t) - 1] for x, t in zip(by_year[y], ts)]
        noise.append(np.var(dev, axis=0, ddof=1) / len(dev))
    noise = np.mean(noise, 0)
    tot = np.var(Y, axis=0, ddof=1)
    f = np.sqrt(np.clip(1 - noise / np.where(tot > 0, tot, np.inf), 0, 1))
    A = Y - Y.mean(0)
    den = [wcorr_sel(clim + A[i] * f, obs, run.w, allc) for i in range(len(ys))]
    out["denoised"] = {"corr": den, "std": float(np.std(den, ddof=1)),
                       "range": float(max(den) - min(den)),
                       "noise_fraction": float(np.sum(noise * run.w[:, None])
                                               / np.sum(tot * run.w[:, None]))}
    print("denoised single years: " + ", ".join(f"{c:.3f}" for c in den)
          + f"; std {out['denoised']['std']:.3f}, range {out['denoised']['range']:.3f}; "
          f"area-weighted noise / total variance {out['denoised']['noise_fraction']:.2f}")
    print(f"\nmean {out['mean']:.3f}, std {out['std']:.3f}, range {out['range']:.3f}; "
          f"all snapshots {out['snap_all_corr']:.3f}; climatology (every 6 h) "
          f"{out['clim_corr']:.3f}; mean |odd − even| {out['half_diff_mean']:.3f}")
    return out


# ---------------------------------------------------------------- profile
def ncep_profiles():
    import netCDF4

    res = {}
    for v, sc in (("rhum", 0.01), ("shum", 1e-3)):
        d = netCDF4.Dataset(RAW / f"ncep_{v}_p/{v}.mon.ltm.1991-2020.nc")
        x = np.ma.filled(d[v][:].astype(float), np.nan) * sc  # [12, lev, lat, lon]
        p = np.asarray(d["level"][:], float) * 100.0
        lat, lon = np.asarray(d["lat"][:], float), np.asarray(d["lon"][:], float)
        res[v] = (x, p, lat, lon)
    return res


def cmd_profile(runs, labels, a):
    nc = ncep_profiles()
    x, p, lat, lon = nc["rhum"]
    ocean = blh.oisst_ocean(lat, lon)
    wl = np.cos(np.radians(lat))
    ic, ilat, ilon = blh.icoads_rh()
    out = {"ncep": {}, "runs": {}}
    obs = {}
    for name, lo, hi, s in PROFILE_REGIONS:
        o = {}
        for v in ("rhum", "shum"):
            xx = nc[v][0]
            o[v] = [blh.band_avg(np.mean([xx[m - 1, k] for m in SEAS[s]], 0), lat, wl,
                                 ocean, lo, hi) for k in range(len(p))]
        o["icoads"] = blh.band_avg(np.nanmean([ic[m - 1] for m in SEAS[s]], 0), ilat,
                                   np.cos(np.radians(ilat)), np.ones_like(ic[0], bool), lo, hi)
        obs[name] = o
    out["ncep"] = {k: {"p_hPa": list(p / 100), **v} for k, v in obs.items()}
    for run, l in zip(runs, labels):
        start = run.year  # year 2
        res = {}
        for name, lo, hi, s in PROFILE_REGIONS:
            rh, n = run.snap_mean("rh", SEAS[s], start)
            q, _ = run.snap_mean("q", SEAS[s], start)
            ps, _ = run.snap_mean("ps", SEAS[s], start)
            ps = ps[0]
            sel = ((run.lat >= lo) & (run.lat <= hi))[:, None] & run.ocean
            lev = []
            for k, sg in enumerate(run.sigma):
                pk = wmean(sg * ps, run.w, sel)
                if pk < 59000:
                    continue
                lev.append({"sigma": float(sg), "p_hPa": pk / 100,
                            "rh": wmean(rh[k], run.w, sel), "q_gkg": wmean(q[k], run.w, sel)
                            * 1e3})
            ps_m = wmean(ps, run.w, sel)
            # NCEP at the model pressures (ln p interpolation; NCEP levels ascending in p)
            o = obs[name]
            lp = np.log(p[::-1])
            for e in lev:
                e["ncep_rh"] = float(np.interp(np.log(e["p_hPa"] * 100), lp, o["rhum"][::-1]))
                e["ncep_q_gkg"] = float(np.interp(np.log(e["p_hPa"] * 100), lp,
                                                  o["shum"][::-1]) * 1e3)
            res[name] = {"levels": lev, "ps_hPa": ps_m / 100, "n": n}
        out["runs"][l] = res
    # print
    for name, lo, hi, s in PROFILE_REGIONS:
        o = obs[name]
        print(f"\n### {name} (ocean; ICOADS near-surface RH {o['icoads']:.3f}; NCEP RH 1000 / "
              f"925 / 850 / 700 / 600 hPa: "
              + " / ".join(f"{v:.3f}" for v in o["rhum"][:5]) + "; q [g/kg] "
              + " / ".join(f"{v * 1e3:.2f}" for v in o["shum"][:5]) + ")\n")
        lv0 = out["runs"][labels[0]][name]["levels"]
        print("| σ | p [hPa] | NCEP RH | " + " | ".join(f"{l} RH" for l in labels)
              + " | NCEP q | " + " | ".join(f"{l} q" for l in labels) + " |")
        print("|---" * (4 + 2 * len(labels)) + "|")
        # rows by the first run's levels; other runs (other level sets) by nearest σ
        allsig = sorted({round(e["sigma"], 4) for l in labels for e in out["runs"][l][name]["levels"]})
        for sg in allsig:
            ent = {l: next((e for e in out["runs"][l][name]["levels"]
                            if abs(e["sigma"] - sg) < 6e-5), None) for l in labels}
            ref = next(e for e in ent.values() if e is not None)
            print(f"| {sg:.4f} | {ref['p_hPa']:.0f} | {ref['ncep_rh']:.3f} | "
                  + " | ".join(f"{ent[l]['rh']:.3f}" if ent[l] else "" for l in labels)
                  + f" | {ref['ncep_q_gkg']:.2f} | "
                  + " | ".join(f"{ent[l]['q_gkg']:.2f}" if ent[l] else "" for l in labels)
                  + " |")
        print("\nsurface pressure [hPa]: " + ", ".join(
            f"{l} {out['runs'][l][name]['ps_hPa']:.0f}" for l in labels))
        del lv0
    return out


# ---------------------------------------------------------------- pblh
def cmd_pblh(runs, labels, a):
    out = {}
    print("| run | region | mean of monthly climatology [m] | p10 / p50 / p90 [m] | source |")
    print("|---|---|---|---|---|")
    for run, l in zip(runs, labels):
        mon = run.monthly("pblh")
        res = {}
        for name, lo, hi, s in PROFILE_REGIONS + [("tropics 10°S–10°N ann", -10, 10, "ann")]:
            sel = ((run.lat >= lo) & (run.lat <= hi))[:, None] & run.ocean
            m = wmean(seasonal(mon, s), run.w, sel)
            sn = [(t, f) for t, f in run.snaps("pblh") if t >= run.year
                  and run.month_of(t) in SEAS[s]]
            if sn:
                vals = np.concatenate([np.load(f).astype(float).reshape(run.nlat, run.nlon)[sel]
                                       for _, f in sn])
                src = f"{len(sn)} snapshots"
            else:
                vals = np.concatenate([mon[mm - 1][sel] for mm in SEAS[s]])
                src = "monthly-mean cells"
            q = np.percentile(vals, [10, 50, 90])
            res[name] = {"mean": m, "p10": q[0], "p50": q[1], "p90": q[2], "src": src}
            print(f"| {l} | {name} | {m:.0f} | {q[0]:.0f} / {q[1]:.0f} / {q[2]:.0f} | {src} |")
        out[l] = res
    return out


# ---------------------------------------------------------------- cloud by level
def cmd_cloud(runs, labels, a):
    out = {}
    regs = [("40–60°N DJF", 40, 60, "DJF"), ("40–60°S JJA", -60, -40, "JJA")]
    for name, lo, hi, s in regs:
        print(f"\n### cloud fraction by level, {name}, ocean\n")
        print("| run | " + "level σ (p hPa): cl … |")
        print("|---|---|")
        for run, l in zip(runs, labels):
            cl, n = run.snap_mean("cl", SEAS[s], run.year)
            ps, _ = run.snap_mean("ps", SEAS[s], run.year)
            sel = ((run.lat >= lo) & (run.lat <= hi))[:, None] & run.ocean
            rows = []
            for k, sg in enumerate(run.sigma):
                pk = wmean(sg * ps[0], run.w, sel) / 100
                if pk < 500:
                    continue
                rows.append({"sigma": float(sg), "p_hPa": pk, "cl": wmean(cl[k], run.w, sel)})
            out.setdefault(l, {})[name] = rows
            print(f"| {l} | " + ", ".join(f"{e['p_hPa']:.0f}: {e['cl']:.3f}" for e in rows)
                  + " |")
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["precip", "spread", "profile", "pblh", "cloud"])
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="+")
    ap.add_argument("--json")
    ap.add_argument("--fig-diff")
    ap.add_argument("--fig-obs")
    a = ap.parse_args()
    import warnings

    warnings.simplefilter("ignore", RuntimeWarning)
    runs = [Run(r) for r in a.runs]
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    out = {"precip": cmd_precip, "spread": cmd_spread, "profile": cmd_profile,
           "pblh": cmd_pblh, "cloud": cmd_cloud}[a.cmd](runs, labels, a)
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(out, indent=1, default=float))


if __name__ == "__main__":
    main()
