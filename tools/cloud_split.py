"""Low / middle / high cloud by latitude band against CALIPSO-GOCCP, level profiles and the
humidity of the low levels (P4 question 13, owner decision after step A, 2026-11-26).

    python tools/cloud_split.py <run> [<run> ...] [--labels a b ...] [--json out.json]
        [--fig-bands <png>] [--fig-profiles <png>]

Observations: CALIPSO-GOCCP v3.1.2 (Chepfer et al. 2010, JGR 115, D00H16,
doi:10.1029/2009JD012251), `MapLowMidHigh330m_200606-201803_avg_CFMIP2_sat_3.1.2.nc` (140
monthly means, day + night, 2° grid), fetched 2026-11-26 from
climserv.ipsl.polytechnique.fr/cfmip-obs/data/GOCCP_v3/2D_Maps/grid_2x2xL40/climato/ into
`reference/raw/goccp/`. Classes (ISCCP convention, as the product defines them): low
p > 680 hPa, middle 440–680 hPa, high p < 440 hPa. Total cover from CERES EBAF Ed4.1
(`reference/derived/t<N>/clt_ceres.npy`) is listed alongside.

Model (snapshots `cl`, `ps`; optional `cl_split`, `rh`, `q`, `t`, `omega`): level pressure
p = σ p_s; the cover of each class is the maximum-random overlap of its levels (the overlap of
the radiation, `solver::cloud_cover`). A lidar does not see low cloud under opaque upper
cloud, so the model's low (and middle) cover is also given "exclusive": the part of the column
with cloud in the class and none above it, = C(class and above) − C(above). The observed value
should fall between exclusive and raw (thin cirrus is transparent to the lidar, thick cloud is
not); no lidar simulator is used. Seasons: DJF and JJA from the snapshot month (365.25-day
year, January first), only snapshots at or after the climatology start.

Per run also, from the online climatology: albedo, OLR, net TOA, SW / LW CRE, cover (P4
criteria 1–2), and the 20°S–20°N net CRE; and in the regions 40–60°N, 40–60°S, 68–80°N the
mean cloud fraction per level (total and stratiform-only when `cl_split` exists), and at the
levels σ ≥ 0.75: mean RH, its 10/50/90 % quantiles, the share of points above the scheme's
RH_c(σ), the share with q < 3 g/kg (the freeze-dry scale) and the share with ω ≥ 0
(subsidence, the Slingo gate), all over all longitudes.
"""

import argparse
import json
import pathlib

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
GOCCP = ROOT / "reference/raw/goccp/MapLowMidHigh330m_200606-201803_avg_CFMIP2_sat_3.1.2.nc"
P_LOW, P_HIGH = 68000.0, 44000.0
BANDS = [("30–45°N", 30, 45), ("45–60°N", 45, 60), ("60–77°N", 60, 77), ("32–57°N", 32, 57),
         ("30–45°S", -45, -30), ("45–60°S", -60, -45), ("60–77°S", -77, -60)]
REGIONS = [("40–60°N", 40, 60), ("40–60°S", -60, -40), ("68–80°N", 68, 80)]
SEASONS = {"ann": list(range(1, 13)), "DJF": [12, 1, 2], "JJA": [6, 7, 8]}
WINTER = {"40–60°N": "DJF", "68–80°N": "DJF", "40–60°S": "JJA"}


def band_mean(x, lat, w, lo, hi):
    """Area-weighted mean of zonal-mean `x` [lat] over lo ≤ φ ≤ hi (NaN ignored)."""
    sel = (lat >= lo) & (lat <= hi) & np.isfinite(x)
    return float(np.sum(x[sel] * w[sel]) / np.sum(w[sel]))


def goccp():
    import netCDF4

    d = netCDF4.Dataset(GOCCP)
    lat = np.asarray(d["latitude"][:], float)
    t = np.asarray(d["time"][:], float)
    month = np.array([(np.datetime64("2000-01-01") + np.timedelta64(int(round(x)), "D")).astype(
        "datetime64[M]").astype(int) % 12 + 1 for x in t])
    out = {}
    for name, var in (("low", "cllcalipso"), ("mid", "clmcalipso"), ("high", "clhcalipso"),
                      ("tot", "cltcalipso")):
        v = np.ma.filled(d[var][:].astype(float), np.nan)
        v[v < 0] = np.nan
        out[name] = {}
        for s, ms in SEASONS.items():
            # mean of the monthly means of each calendar month, then over the season's months
            mon = np.stack([np.nanmean(v[month == m], axis=0) for m in ms])
            zon = np.nanmean(np.nanmean(mon, axis=0), axis=1)
            w = np.cos(np.radians(lat))
            out[name][s] = {b: band_mean(zon, lat, w, lo, hi) for b, lo, hi in BANDS}
    return out


def snaps(run, name):
    fs = sorted((run / "fields").glob(f"{name}_*.npy"), key=lambda f: int(f.stem.split("_")[-1]))
    return {int(f.stem.split("_")[-1]): f for f in fs}


def maxrand(c):
    """Maximum-random cover of c[lev, ...] (levels top to bottom)."""
    clear = np.ones(c.shape[1:])
    prev = np.zeros(c.shape[1:])
    for k in range(c.shape[0]):
        ck = np.clip(c[k], 0, 1)
        ok = prev < 1 - 1e-12
        clear = np.where(ok, clear * (1 - np.maximum(prev, ck)) / np.where(ok, 1 - prev, 1), 0.0)
        prev = ck
    return 1 - clear


def climate(run, lat, w2):
    c = run / "climatology"
    m = {n: np.load(c / f"mean_{n}.npy").astype(float).reshape(w2.shape)
         for n in ("asr", "asr_cs", "olr", "olr_cs", "rsdt", "clt")}
    g = lambda x: float(np.sum(x * w2) / np.sum(w2))
    sw, lw = m["asr"] - m["asr_cs"], m["olr_cs"] - m["olr"]
    trop = np.abs(lat)[:, None] * np.ones_like(w2) <= 20
    gt = lambda x: float(np.sum((x * w2)[trop]) / np.sum(w2[trop]))
    return {"albedo": 1 - g(m["asr"]) / g(m["rsdt"]), "olr": g(m["olr"]),
            "net": g(m["asr"]) - g(m["olr"]), "sw_cre": g(sw), "lw_cre": g(lw),
            "cover": g(m["clt"]), "trop_net_cre": gt(sw + lw), "zonal_clt": m["clt"].mean(axis=1)}


def analyse(run, rhc):
    run = pathlib.Path(run)
    meta = json.loads((run / "meta.json").read_text())
    cfg = meta["config"]
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    nlat, nlon = len(lat), len(meta["grid"]["lon"])
    w = np.asarray(meta["grid"]["weights"], float)
    w2 = np.repeat(w[:, None], nlon, axis=1)
    sig = np.asarray(meta["sigma"], float)
    nl = len(sig)
    dt, year = float(cfg["dt"]), 365.25 * 86400.0
    start = float(cfg.get("climatology", {}).get("start", 0.0))
    have = {n: snaps(run, n) for n in ("cl", "ps", "psl", "cl_split", "rh", "q", "omega")}
    steps = sorted(s for s in have["cl"] if s * dt >= start)
    months = np.array([int(((s * dt) % year) / (year / 12.0)) + 1 for s in steps])
    orog = np.load(next(iter(snaps(run, "orog").values()))).astype(float).reshape(nlat, nlon) \
        if snaps(run, "orog") else np.zeros((nlat, nlon))
    zon = {k: [] for k in ("low", "mid", "high", "low_x", "mid_x", "tot")}
    regs = {r: {k: [] for k in ("cl", "cl_split", "rh", "q", "omega")} for r, _, _ in REGIONS}
    up = sig >= 0.75
    for s in steps:
        cl = np.load(have["cl"][s]).astype(float).reshape(nl, nlat, nlon)
        if s in have["ps"]:
            ps = np.load(have["ps"][s]).astype(float).reshape(nlat, nlon)
        else:  # p_s from sea-level pressure with a 288 K column (r3 T42 saved psl only)
            ps = np.load(have["psl"][s]).astype(float).reshape(nlat, nlon) * np.exp(
                -9.80616 * orog / (287.04 * 288.0))
        p = sig[:, None, None] * ps[None]
        lo, hi = p > P_LOW, p < P_HIGH
        mi = ~lo & ~hi
        c_lo, c_mi, c_hi = (maxrand(np.where(m, cl, 0)) for m in (lo, mi, hi))
        c_mh = maxrand(np.where(~lo, cl, 0))
        c_all = maxrand(cl)
        for k, v in (("low", c_lo), ("mid", c_mi), ("high", c_hi), ("low_x", c_all - c_mh),
                     ("mid_x", c_mh - c_hi), ("tot", c_all)):
            zon[k].append(v.mean(axis=1))
        extra = {n: np.load(have[n][s]).astype(float).reshape(nl, nlat, nlon)
                 for n in ("cl_split", "rh", "q", "omega") if s in have[n]}
        for r, a, b in REGIONS:
            jj = (lat >= a) & (lat <= b)
            regs[r]["cl"].append((cl[:, jj] * w[jj][None, :, None]).sum(axis=(1, 2))
                                 / (w[jj].sum() * nlon))
            if "cl_split" in extra:
                x = extra["cl_split"]
                regs[r]["cl_split"].append((x[:, jj] * w[jj][None, :, None]).sum(axis=(1, 2))
                                           / (w[jj].sum() * nlon))
            for n in ("rh", "q", "omega"):
                if n in extra:
                    regs[r][n].append(extra[n][up][:, jj].reshape(up.sum(), -1))
    res = {"run": str(run), "n_snapshots": len(steps), "bands": {}, "regions": {},
           "climate": climate(run, lat, w2)}
    zc = res["climate"].pop("zonal_clt")
    res["bands_clt_online"] = {b: band_mean(zc, lat, w, a, c) for b, a, c in BANDS}
    for k, v in zon.items():
        v = np.asarray(v)
        res["bands"][k] = {s: {b: band_mean(v[np.isin(months, ms)].mean(axis=0), lat, w, a, c)
                               for b, a, c in BANDS} for s, ms in SEASONS.items()}
    lrc = np.array([rhc(x) for x in sig[up]])
    for r, _, _ in REGIONS:
        o = res["regions"][r] = {}
        for s, ms in SEASONS.items():
            sel = np.isin(months, ms)
            e = o[s] = {"cl": np.asarray(regs[r]["cl"])[sel].mean(axis=0).tolist()}
            if regs[r]["cl_split"]:
                e["cl_split"] = np.asarray(regs[r]["cl_split"])[sel].mean(axis=0).tolist()
            if regs[r]["rh"]:
                rh = np.concatenate([x for x, m in zip(regs[r]["rh"], sel) if m], axis=1)
                e["rh_mean"] = rh.mean(axis=1).tolist()
                e["rh_q"] = np.quantile(rh, [0.1, 0.5, 0.9], axis=1).T.tolist()
                e["rh_above_rhc"] = (rh > lrc[:, None]).mean(axis=1).tolist()
            if regs[r]["q"]:
                q = np.concatenate([x for x, m in zip(regs[r]["q"], sel) if m], axis=1)
                e["q_below_3gkg"] = (q < 3e-3).mean(axis=1).tolist()
                e["q_mean_gkg"] = (1e3 * q.mean(axis=1)).tolist()
            if regs[r]["omega"]:
                om = np.concatenate([x for x, m in zip(regs[r]["omega"], sel) if m], axis=1)
                e["omega_sinking"] = (om >= 0).mean(axis=1).tolist()
    res["sigma"] = sig.tolist()
    res["sigma_up"] = sig[up].tolist()
    res["rh_crit_up"] = lrc.tolist()
    ceres = ROOT / f"reference/derived/t{meta['truncation']}/clt_ceres.npy"
    cz = np.load(ceres).astype(float).reshape(-1, nlat, nlon).mean(axis=0).mean(axis=1)
    res["ceres_clt"] = {b: band_mean(cz, lat, w, a, c) for b, a, c in BANDS}
    return res


def rhc_of(run):
    """RH_c(σ) of the run's [clouds] (defaults of DiagnosticClouds::new)."""
    meta = json.loads((pathlib.Path(run) / "meta.json").read_text())
    c = meta["config"].get("clouds", {})
    s, u, n = c.get("rh_crit_surface", 0.95), c.get("rh_crit_upper", 0.9), c.get(
        "rh_crit_exponent", 4.0)
    return lambda sg: u + (s - u) * np.exp(1 - max(sg, 1e-6) ** (-n))


def print_report(rs, labels, obs):
    print("## Cover by class and band, annual (model raw / exclusive; GOCCP; CERES total)\n")
    for cls, xk in (("low", "low_x"), ("mid", "mid_x"), ("high", None), ("tot", None)):
        print(f"**{cls}**\n")
        print("| band | " + " | ".join(labels) + " | GOCCP |" + (" CERES |" if cls == "tot" else ""))
        print("|---" * (len(labels) + 2 + (cls == "tot")) + "|")
        for b, _, _ in BANDS:
            cells = []
            for r in rs:
                v = r["bands"][cls]["ann"][b]
                cells.append(f"{v:.3f}" + (f" / {r['bands'][xk]['ann'][b]:.3f}" if xk else ""))
            row = f"| {b} | " + " | ".join(cells) + f" | {obs[cls]['ann'][b]:.3f} |"
            if cls == "tot":
                row += f" {rs[0]['ceres_clt'][b]:.3f} |"
            print(row)
        print()
    print("## Seasonal low (raw / exclusive) against GOCCP low\n")
    print("| band | season | " + " | ".join(labels) + " | GOCCP |")
    print("|---" * (len(labels) + 3) + "|")
    for b, _, _ in BANDS:
        for s in ("DJF", "JJA"):
            cells = [f"{r['bands']['low'][s][b]:.3f} / {r['bands']['low_x'][s][b]:.3f}" for r in rs]
            print(f"| {b} | {s} | " + " | ".join(cells) + f" | {obs['low'][s][b]:.3f} |")
    print()
    print("## Climate (criteria 1–2, cover, tropical net CRE) and online cover by band\n")
    print("| run | albedo | OLR | net TOA | SW CRE | LW CRE | cover | 20S–20N net CRE | "
          + " | ".join(b for b, _, _ in BANDS) + " |")
    print("|---" * (8 + len(BANDS)) + "|")
    for r, lab in zip(rs, labels):
        c = r["climate"]
        print(f"| {lab} | {c['albedo']:.3f} | {c['olr']:.1f} | {c['net']:+.2f} | {c['sw_cre']:.1f} | "
              f"{c['lw_cre']:+.1f} | {c['cover']:.3f} | {c['trop_net_cre']:.1f} | "
              + " | ".join(f"{r['bands_clt_online'][b]:.3f} ({r['bands_clt_online'][b] - r['ceres_clt'][b]:+.3f})"
                           for b, _, _ in BANDS) + " |")
    print("| CERES | | | | | | | | " + " | ".join(f"{rs[0]['ceres_clt'][b]:.3f}" for b, _, _ in BANDS) + " |\n")
    sig = rs[0]["sigma"]
    low = [k for k, s in enumerate(sig) if s >= 0.6]
    for reg, _, _ in REGIONS:
        for s in ("DJF", "JJA"):
            tag = "winter" if WINTER[reg] == s else "summer"
            print(f"### {reg} {s} ({tag}): cloud fraction by level (total / stratiform-only)\n")
            print("| σ | " + " | ".join(labels) + " |")
            print("|---" * (len(labels) + 1) + "|")
            for k in low:
                cells = []
                for r in rs:
                    e = r["regions"][reg][s]
                    cells.append(f"{e['cl'][k]:.3f}" + (f" / {e['cl_split'][k]:.3f}" if "cl_split" in e else ""))
                print(f"| {sig[k]:.3f} | " + " | ".join(cells) + " |")
            print()
            if "rh_mean" in rs[0]["regions"][reg][s]:
                print("RH at σ ≥ 0.75, run " + " / ".join(labels) +
                      ": mean; q10 q50 q90; share > RH_c; share q < 3 g/kg; share ω ≥ 0\n")
                print("| σ | RH_c | " + " | ".join(labels) + " |")
                print("|---" * (len(labels) + 2) + "|")
                for i, sg in enumerate(rs[0]["sigma_up"]):
                    cells = []
                    for r in rs:
                        e = r["regions"][reg][s]
                        if "rh_mean" not in e:
                            cells.append("—")
                            continue
                        qq = e["rh_q"][i]
                        cells.append(f"{e['rh_mean'][i]:.2f}; {qq[0]:.2f} {qq[1]:.2f} {qq[2]:.2f}; "
                                     f"{e['rh_above_rhc'][i]:.2f}; {e.get('q_below_3gkg', [np.nan] * 99)[i]:.2f}; "
                                     f"{e.get('omega_sinking', [np.nan] * 99)[i]:.2f}")
                    print(f"| {sg:.3f} | {rs[0]['rh_crit_up'][i]:.3f} | " + " | ".join(cells) + " |")
                print()


def fig_bands(rs, labels, obs, path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    names = [b for b, _, _ in BANDS]
    x = np.arange(len(names))
    fig, ax = plt.subplots(1, 3, figsize=(15, 4.2), sharey=True)
    for a, cls, xk in zip(ax, ("low", "mid", "high"), ("low_x", "mid_x", None)):
        a.plot(x, [obs[cls]["ann"][b] for b in names], "ko-", lw=2, label="CALIPSO-GOCCP")
        for i, (r, lab) in enumerate(zip(rs, labels)):
            col = f"C{i}"
            a.plot(x, [r["bands"][cls]["ann"][b] for b in names], "o-", color=col, label=f"{lab} raw")
            if xk:
                a.plot(x, [r["bands"][xk]["ann"][b] for b in names], "s--", color=col,
                       label=f"{lab} exclusive")
        a.set_xticks(x, names, rotation=35)
        a.set_title(f"{cls} cloud cover, annual")
        a.grid(alpha=0.3)
    ax[0].set_ylabel("cover")
    ax[0].legend(fontsize=7)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


def fig_profiles(rs, labels, path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    sig = np.asarray(rs[0]["sigma"])
    fig, ax = plt.subplots(2, 3, figsize=(13, 8), sharey=True)
    for j, (reg, _, _) in enumerate(REGIONS):
        for i, s in enumerate(("DJF", "JJA")):
            a = ax[i, j]
            for n, (r, lab) in enumerate(zip(rs, labels)):
                e = r["regions"][reg][s]
                a.plot(e["cl"], sig, "-", color=f"C{n}", label=f"{lab} total")
                if "cl_split" in e:
                    a.plot(e["cl_split"], sig, ":", color=f"C{n}")
            a.set_ylim(1.0, 0.3)
            a.set_title(f"{reg} {s}")
            a.grid(alpha=0.3)
    ax[0, 0].legend(fontsize=7)
    for a in ax[:, 0]:
        a.set_ylabel("σ")
    for a in ax[1]:
        a.set_xlabel("cloud fraction (solid total, dotted stratiform)")
    fig.tight_layout()
    fig.savefig(path, dpi=110)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("runs", nargs="+")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--json")
    ap.add_argument("--fig-bands")
    ap.add_argument("--fig-profiles")
    a = ap.parse_args()
    labels = a.labels or [pathlib.Path(r).name for r in a.runs]
    obs = goccp()
    rs = [analyse(r, rhc_of(r)) for r in a.runs]
    print_report(rs, labels, obs)
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps({"goccp": obs, "runs": dict(zip(labels, rs))}, indent=1))
    if a.fig_bands:
        fig_bands(rs, labels, obs, a.fig_bands)
    if a.fig_profiles:
        fig_profiles(rs, labels, a.fig_profiles)


if __name__ == "__main__":
    main()
