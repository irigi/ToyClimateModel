"""Why L32 (v4) changes the tropical cloud, and the offline cloud-threshold check (P4 question 13,
owner decision after the level and mixing runs, 2026-11-27, steps 1 and 2).

    python tools/q13_v4.py levels
    python tools/q13_v4.py cre <run> [<run> ...] --labels s1 v2 v4 [--lon-stride 2] [--json out]
    python tools/q13_v4.py profile <run> ... --labels ... [--json out]
    python tools/q13_v4.py thresh <run> --label s1 [--json out]

All model numbers are year 2 (snapshots at or after one year, climatology from day 365.25).

`cre`: 20°S–20°N, all surfaces. Per 5-day snapshot the model's own radiation is evaluated
offline (`toyclim radiation` with clouds, the same two-stream code, overlap and cloud optics as
the model) on the saved `cl`, `cl_split`, `t`, `q`, `rh`, `ps`, `ts` and `land`. The cloud
memory is rebuilt as the scheme builds it: in-cloud water r = min(β q_s, r_max) with β = 0.03,
r_max = `water_max` 1.5e-4 kg/kg (r3), q_s = q / rh (the model's own q_s), path r Δp / g, ice
share clamp((273.15 − T)/20, 0, 1). Insolation is the daily mean of the snapshot's day with the
insolation-weighted cos zenith (orbit of the case: obliquity 23.44°, e 0.0167, perihelion
282.9°, solar longitude 280° at t = 0, 365.25-day year, 1361 W m⁻²). Surface albedo: ocean
0.06 / 0.06, land 0.15 / 0.30 (the defaults; there is no snow or sea ice at 20°S–20°N). The
offline all-sky CRE is checked against the online climatology of the same run.

Each layer group's CRE is the change when that group's cloud is removed (the convention of the
2026-11-23 review round): low p > 680 hPa, middle 440–680, high < 440 (GOCCP classes);
"stratiform low" removes the stratiform part of the low layers and keeps their convective part
(c_v = 1 − (1 − c)/(1 − c_s), c_s = `cl_split`); "convective" = all-sky − stratiform-only.
Arithmetic test: the same clouds with each layer's in-cloud water rescaled to the thickness of
the other level set's layer that contains it (v4: ×Δp_L26/Δp_L32 = 2 in the six split layers;
s1: × 1/2 below σ 0.649). If v4 with L26 water gives back s1's CRE, the gain is arithmetic.

`profile`: RH, q and T at 600–900 hPa in the trade belts (10–25°N, 10–25°S, annual) and the
north-west Pacific in summer (10–30°N, 100–160°E, JJA), ocean, against NCEP/NCAR R1 long-term
monthly means 1991–2020 (`reference/raw/ncep_{rhum,shum,air}_p`).

`thresh` (step 2): 40–60°N DJF and 40–60°S JJA, ocean, every snapshot column of year 2. The
scheme's low-cloud fraction is recomputed offline from the model's RH, q, T and ω at each level
σ ≥ 0.75 (a transcription of `physics/clouds`: Sundqvist c_s = 1 − √((1 − RH)/(1 − RH_c)),
RH_c(σ) = u + (s − u) exp(1 − σ⁻⁴), u = 0.80; freeze-dry ×max(0.15, min(1, q/3 g/kg)) at
p ≥ 750 hPa; the Slingo gate ×clamp(−ω/0.1 Pa s⁻¹, 0, 1) at σ ≥ 0.8; no RH cloud in the
lowest layer at σ ≥ 0.8; the convective part and the LTS stratus are not recomputed and are
taken from the run (the stratus is combined by maximum with the layer it was placed in, read as
cl − RH cloud where that is positive: see the output), then the low cover is the
maximum-random overlap of the levels with p > 680 hPa. Repeated with the NCEP RH and q profile
of the same region and season (monthly long-term mean, interpolated in ln p to each column's
level pressures) in place of the model's. Grid of RH_c(surface) 0.75–0.95, freeze-dry on/off,
gate on/off. Against GOCCP low cover of the band.
"""

import argparse
import json
import math
import pathlib
import subprocess
import sys
import tempfile

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import bl_humidity as blh  # noqa: E402
from q13_step1 import SEAS, Run, ncep_profiles, wmean  # noqa: E402

G = 9.80616
BETA, RMAX = 0.03, 1.5e-4
P_LOW, P_HIGH = 68000.0, 44000.0
EXE = ROOT / "target/release/toyclim"
KNOBS = "ozone_tau = 0.02425\nozone_p = 6429.6\n"


def sigma_half(run):
    sh = run.meta["config"].get("sigma_half")
    if sh is None:
        sh = json.loads((ROOT / "out/p4/q13res/v2_l30/meta.json").read_text())["config"][
            "sigma_half"]
        # L26: the v2 set without its four inserted half levels
        sh = [s for s in sh if round(s, 6) not in (0.867161, 0.929649, 0.970555, 0.992556)]
    sh = np.asarray(sh, float)
    assert len(sh) == len(run.sigma) + 1
    return sh


# ---------------------------------------------------------------- levels
def cmd_levels(a):
    for lab, r in (("L26", "out/p4/q13scr/s1_ref"), ("L30", "out/p4/q13res/v2_l30"),
                   ("L32", "out/p4/q13res/v4_l32")):
        run = Run(ROOT / r)
        sh = sigma_half(run)
        print(f"\n{lab}: {len(run.sigma)} levels; σ ≥ 0.6 (half levels | full levels | Δσ)")
        for k in range(len(run.sigma)):
            if run.sigma[k] > 0.6:
                print(f"  {sh[k]:.4f}–{sh[k + 1]:.4f}  σ {run.sigma[k]:.4f}  Δσ {sh[k + 1] - sh[k]:.4f}"
                      f"  ({run.sigma[k] * 1000:.0f} hPa at p_s 1000)")
        k700 = int(np.argmin(np.abs(run.sigma - 0.7)))
        print(f"  LTS level (σ nearest 0.7): σ {run.sigma[k700]:.4f}")


# ---------------------------------------------------------------- insolation
def insolation(t, lat_deg):
    """Daily-mean TOA flux on a horizontal surface and the insolation-weighted cos zenith."""
    year = 365.25 * 86400.0
    lam = math.radians(280.0 + 360.0 * (t % year) / year)
    eps, e, varpi = math.radians(23.44), 0.0167, math.radians(282.9)
    dec = math.asin(math.sin(eps) * math.sin(lam))
    fac = ((1 + e * math.cos(lam - varpi)) / (1 - e * e)) ** 2
    phi = np.radians(lat_deg)
    a, b = np.sin(phi) * math.sin(dec), np.cos(phi) * math.cos(dec)
    h0 = np.arccos(np.clip(-a / b, -1, 1))
    s = 1361.0 * fac / math.pi * (h0 * a + b * np.sin(h0))
    num = h0 * a * a + 2 * a * b * np.sin(h0) + b * b * (h0 / 2 + np.sin(2 * h0) / 4)
    den = h0 * a + b * np.sin(h0)
    mu = np.where(den > 0, num / np.maximum(den, 1e-12), 0.0)
    return np.maximum(s, 0.0), mu


# ---------------------------------------------------------------- offline radiation
def radiate(cols):
    with tempfile.TemporaryDirectory() as d:
        kf, cf = pathlib.Path(d) / "k.toml", pathlib.Path(d) / "c.json"
        kf.write_text(KNOBS)
        cf.write_text(json.dumps(cols))
        r = subprocess.run([str(EXE), "radiation", "--knobs", str(kf), "--columns", str(cf)],
                           capture_output=True, text=True, check=True)
    return [(x["olr"], x["sw_up_top"]) for x in json.loads(r.stdout)]


def load_step(run, name, step):
    f = run.path / "fields" / f"{name}_{step:06d}.npy"
    return np.load(f).astype(float).reshape(-1, run.nlat, run.nlon)


def year2_steps(run):
    return [int(f.stem.split("_")[-1]) for t, f in run.snaps("cl") if t >= run.year]


def cmd_cre(runs, labels, a):
    out = {}
    for run, lab in zip(runs, labels):
        sh = sigma_half(run)
        L = len(run.sigma)
        rows = np.where(np.abs(run.lat) <= 20)[0]
        lons = np.arange(0, run.nlon, a.lon_stride)
        # the other level set's layer thickness for the arithmetic test
        if L == 32:
            fac = np.ones(L)
            fac[run.sigma > 0.6492097] = 2.0
        elif L == 26:
            fac = np.ones(L)
            fac[run.sigma > 0.6492097] = 0.5
        else:
            fac = None
        cfgs = ["clear", "all", "no_low", "no_mid", "no_high", "strat_only", "no_low_strat"]
        if fac is not None:
            cfgs.append("other_water")
        acc = {c: [0.0, 0.0] for c in cfgs}  # [olr, sw_up] area-weighted sums
        wsum = 0.0
        insol_sum = 0.0
        lev = {k: np.zeros(L) for k in ("cl", "cs", "cv", "wpath", "rh", "vol")}
        grp = {g: {"cover": 0.0, "wpath": 0.0} for g in ("low", "mid", "high")}
        nsnap = 0
        for step in year2_steps(run)[:a.max_snap or None]:
            t = step * run.dt
            cl, cs = load_step(run, "cl", step), load_step(run, "cl_split", step)
            T, q, rh = load_step(run, "t", step), load_step(run, "q", step), load_step(run, "rh", step)
            ps, ts = load_step(run, "ps", step)[0], load_step(run, "ts", step)[0]
            land = load_step(run, "land", step)[0]
            s0, mu = insolation(t, run.lat)
            cols, wts = [], []
            for i in rows:
                for j in lons:
                    p_half = sh * ps[i, j]
                    dp = np.diff(p_half)
                    p = run.sigma * ps[i, j]
                    qs = q[:, i, j] / np.maximum(rh[:, i, j], 1e-9)
                    w = np.minimum(BETA * qs, RMAX) * dp / G
                    w = np.where(cl[:, i, j] > 0, w, 0.0)
                    fi = np.clip((273.15 - T[:, i, j]) / 20.0, 0, 1)
                    c = cl[:, i, j]
                    cst = np.minimum(cs[:, i, j], c)
                    cv = np.where(cst < 1, 1 - (1 - c) / np.maximum(1 - cst, 1e-12), 0.0)
                    cv = np.clip(cv, 0, 1)
                    low, high = p > P_LOW, p < P_HIGH
                    mid = ~low & ~high
                    fl = land[i, j]
                    alb = [0.06 * (1 - fl) + 0.15 * fl, 0.06 * (1 - fl) + 0.30 * fl]
                    base = {"p_half": list(p_half), "t": list(T[:, i, j]), "q": list(q[:, i, j]),
                            "ts": float(ts[i, j]), "albedo": alb, "s0": float(s0[i]),
                            "mu0": float(mu[i]), "t_eff": 5772.0, "co2_ppm": 400.0,
                            "mu_min": 0.05}

                    def mem(frac, wl=w):
                        return list(frac) + list((1 - fi) * wl) + list(fi * wl)

                    variants = {
                        "clear": None, "all": mem(c),
                        "no_low": mem(np.where(low, 0, c)), "no_mid": mem(np.where(mid, 0, c)),
                        "no_high": mem(np.where(high, 0, c)), "strat_only": mem(cst),
                        "no_low_strat": mem(np.where(low, cv, c)),
                    }
                    if fac is not None:
                        variants["other_water"] = mem(c, w * fac)
                    for v in cfgs:
                        col = dict(base)
                        if variants[v] is not None:
                            col["cloud"] = variants[v]
                        cols.append(col)
                    wt = run.w[i]
                    wts.append(wt)
                    insol_sum += wt * s0[i]
                    wsum += wt
                    for k in range(L):
                        lev["cl"][k] += wt * c[k]
                        lev["cs"][k] += wt * cst[k]
                        lev["cv"][k] += wt * cv[k]
                        lev["wpath"][k] += wt * c[k] * w[k] * 1e3
                        lev["rh"][k] += wt * rh[k, i, j]
                        lev["vol"][k] += wt * c[k] * dp[k] / 100.0
                    for g, sel in (("low", low), ("mid", mid), ("high", high)):
                        grp[g]["cover"] += wt * mr_cover(c[sel])
                        grp[g]["wpath"] += wt * float(np.sum(c[sel] * w[sel])) * 1e3
            res = radiate(cols)
            nv = len(cfgs)
            for n, wt in enumerate(wts):
                for iv, v in enumerate(cfgs):
                    o, s = res[n * nv + iv]
                    acc[v][0] += wt * o
                    acc[v][1] += wt * s
            nsnap += 1
            print(f"  {lab}: snapshot {nsnap} (step {step})", file=sys.stderr)
        m = {v: (acc[v][0] / wsum, acc[v][1] / wsum) for v in cfgs}

        def cre(v_with, v_without):
            # SW CRE = SW↑ without − SW↑ with (< 0 for reflecting cloud); LW = OLR without − with
            sw = m[v_without][1] - m[v_with][1]
            lw = m[v_without][0] - m[v_with][0]
            return {"sw": sw, "lw": lw, "net": sw + lw}

        r = {"n_snap": nsnap, "insolation": insol_sum / wsum,
             "all": cre("all", "clear"), "low": cre("all", "no_low"),
             "mid": cre("all", "no_mid"), "high": cre("all", "no_high"),
             "low_strat": cre("all", "no_low_strat"),
             "strat": cre("strat_only", "clear")}
        r["conv"] = {k: r["all"][k] - r["strat"][k] for k in ("sw", "lw", "net")}
        if fac is not None:
            r["other_water_all"] = cre("other_water", "clear")
        r["levels"] = [{"sigma": float(run.sigma[k]), **{x: float(lev[x][k] / wsum) for x in lev}}
                       for k in range(L)]
        r["groups"] = {g: {x: grp[g][x] / wsum for x in grp[g]} for g in grp}
        # online check (climatology, year 2)
        sel = (np.abs(run.lat) <= 20)[:, None] & np.ones((1, run.nlon), bool)

        def clim(n):
            return np.load(run.path / "climatology" / f"mean_{n}.npy").astype(float).reshape(
                run.nlat, run.nlon)

        sw_on = wmean(clim("asr") - clim("asr_cs"), run.w, sel)
        lw_on = wmean(clim("olr_cs") - clim("olr"), run.w, sel)
        r["online"] = {"sw": sw_on, "lw": lw_on, "net": sw_on + lw_on,
                       "clwvi_gm2": wmean(clim("clwvi"), run.w, sel) * 1e3,
                       "conv_top_hPa": wmean(clim("conv_top_p"), run.w, sel) / 100,
                       "precip_conv": wmean(clim("precip_conv"), run.w, sel) * 86400,
                       "precip_ls": wmean(clim("precip_ls"), run.w, sel) * 86400,
                       "cl_lts": wmean(clim("cl_lts"), run.w, sel),
                       "lts": wmean(clim("lts"), run.w, sel),
                       "conv_active": wmean(clim("conv_active"), run.w, sel)}
        out[lab] = r
    # print
    print("\n### 20°S–20°N cloud radiative effect, offline (model radiation) [W m⁻², SW / LW / net]\n")
    print("| part | " + " | ".join(labels) + " |")
    print("|---" * (len(labels) + 1) + "|")
    for part, name in (("online", "all-sky, online climatology"), ("all", "all-sky, offline"),
                       ("low", "low p > 680 hPa (removal)"), ("mid", "middle 440–680"),
                       ("high", "high < 440"), ("low_strat", "stratiform low"),
                       ("strat", "stratiform-only cloud"), ("conv", "convective (all − strat-only)"),
                       ("other_water_all", "all-sky with the other level set's water")):
        cells = []
        for lab in labels:
            x = out[lab].get(part)
            cells.append("" if x is None else f"{x['sw']:.1f} / {x['lw']:.1f} / **{x['net']:.1f}**")
        print(f"| {name} | " + " | ".join(cells) + " |")
    print("\n### 20°S–20°N groups: max-random cover and grid-mean water path [g m⁻²]\n")
    print("| group | " + " | ".join(f"{l} cover | {l} water" for l in labels) + " |")
    print("|---" * (2 * len(labels) + 1) + "|")
    for g in ("low", "mid", "high"):
        print(f"| {g} | " + " | ".join(
            f"{out[l]['groups'][g]['cover']:.3f} | {out[l]['groups'][g]['wpath']:.1f}" for l in labels)
            + " |")
    print("\n### online 20°S–20°N (year 2)\n")
    for k in ("clwvi_gm2", "conv_top_hPa", "precip_conv", "precip_ls", "cl_lts", "lts",
              "conv_active"):
        print(f"- {k}: " + ", ".join(f"{l} {out[l]['online'][k]:.3f}" for l in labels))
    print("\n### 20°S–20°N by level, σ ≥ 0.6 (total / stratiform / convective fraction; "
          "grid-mean water g m⁻²; c·Δp hPa; RH)\n")
    for lab in labels:
        print(f"\n{lab}:")
        for e in out[lab]["levels"]:
            if e["sigma"] > 0.6:
                print(f"  σ {e['sigma']:.4f}: {e['cl']:.3f} / {e['cs']:.3f} / {e['cv']:.3f};"
                      f" water {e['wpath']:.1f}; c·Δp {e['vol']:.2f}; RH {e['rh']:.3f}")
        lo = [e for e in out[lab]["levels"] if e["sigma"] > 0.6]
        print(f"  sum σ > 0.6: water {sum(e['wpath'] for e in lo):.1f}, c·Δp "
              f"{sum(e['vol'] for e in lo):.2f} hPa")
    return out


def mr_cover(c):
    """Maximum-random overlap of a column of layer fractions (top first)."""
    clear, prev = 1.0, 0.0
    for x in c:
        if prev < 1:
            clear *= (1 - max(x, prev)) / (1 - prev)
        else:
            clear = 0.0
        prev = x
    return 1 - clear


# ---------------------------------------------------------------- profiles
PROF2 = [("trades 10–25°N ann", 10, 25, 0, 360, "ann"),
         ("trades 10–25°S ann", -25, -10, 0, 360, "ann"),
         ("NW Pacific 10–30°N 100–160°E JJA", 10, 30, 100, 160, "JJA")]


def box_avg(x, lat, lon, ocean, lo, hi, l0, l1):
    sel = ((lat >= lo) & (lat <= hi))[:, None] & ((lon >= l0) & (lon <= l1))[None, :] & ocean
    w = np.broadcast_to(np.cos(np.radians(lat))[:, None], x.shape)
    s = sel & np.isfinite(x)
    return float(np.sum(np.where(s, x * w, 0)) / np.sum(np.where(s, w, 0)))


def ncep_air():
    import netCDF4

    d = netCDF4.Dataset(ROOT / "reference/raw/ncep_air_p/air.mon.ltm.1991-2020.nc")
    x = np.ma.filled(d["air"][:].astype(float), np.nan)
    if np.nanmean(x) < 100:
        x = x + 273.15
    return x, np.asarray(d["level"][:], float) * 100.0


def cmd_profile(runs, labels, a):
    nc = ncep_profiles()
    _, p, lat, lon = nc["rhum"]
    air, pa = ncep_air()
    ocean = blh.oisst_ocean(lat, lon)
    out = {}
    for name, lo, hi, l0, l1, s in PROF2:
        ms = SEAS[s]
        obs = {}
        for v, x, pp in (("rh", nc["rhum"][0], p), ("q", nc["shum"][0], p), ("t", air, pa)):
            prof = [box_avg(np.mean([x[m - 1, k] for m in ms], 0), lat, lon, ocean, lo, hi, l0, l1)
                    for k in range(len(pp))]
            obs[v] = (np.asarray(pp), np.asarray(prof))
        res = {"ncep": {v: {"p_hPa": list(obs[v][0] / 100), "val": list(obs[v][1])} for v in obs}}
        for run, lab in zip(runs, labels):
            rh, _ = run.snap_mean("rh", ms, run.year)
            q, _ = run.snap_mean("q", ms, run.year)
            T, n = run.snap_mean("t", ms, run.year)
            ps = run.snap_mean("ps", ms, run.year)[0][0]
            sel = (((run.lat >= lo) & (run.lat <= hi))[:, None]
                   & ((run.lon >= l0) & (run.lon <= l1))[None, :] & run.ocean)
            lev = []
            for k, sg in enumerate(run.sigma):
                pk = wmean(sg * ps, run.w, sel)
                if not 58000 <= pk <= 92000:
                    continue
                e = {"sigma": float(sg), "p_hPa": pk / 100, "rh": wmean(rh[k], run.w, sel),
                     "q_gkg": wmean(q[k], run.w, sel) * 1e3, "t": wmean(T[k], run.w, sel)}
                for v in ("rh", "q", "t"):
                    pp, val = obs[v]
                    o = np.argsort(pp)
                    e["ncep_" + v] = float(np.interp(np.log(pk), np.log(pp[o]), val[o]))
                e["ncep_q"] *= 1e3
                lev.append(e)
            res[lab] = lev
        out[name] = res
        print(f"\n### {name} (ocean; RH, q [g/kg], T [K]; NCEP interpolated in ln p to each "
              "level's mean pressure)\n")
        print("| run | σ | p [hPa] | RH | NCEP RH | q | NCEP q | T | NCEP T | ΔT |")
        print("|---|---|---|---|---|---|---|---|---|---|")
        for lab in labels:
            for e in res[lab]:
                print(f"| {lab} | {e['sigma']:.4f} | {e['p_hPa']:.0f} | {e['rh']:.3f} | "
                      f"{e['ncep_rh']:.3f} | {e['q_gkg']:.2f} | {e['ncep_q']:.2f} | {e['t']:.1f} | "
                      f"{e['ncep_t']:.1f} | {e['t'] - e['ncep_t']:+.1f} |")
    return out


# ---------------------------------------------------------------- thresholds (step 2)
def rh_crit(sigma, s, u=0.80, n=4.0):
    return u + (s - u) * np.exp(1 - sigma ** (-n))


def sundqvist(rh, rc):
    x = np.clip((1 - rh) / (1 - rc), 0, None)
    return np.where(rh > rc, 1 - np.sqrt(np.minimum(x, 1.0)), 0.0)


def low_cover(frac, p):
    """Max-random cover of the layers with p > 680 hPa; frac [L, N] top first."""
    clear = np.ones(frac.shape[1])
    prev = np.zeros(frac.shape[1])
    for k in range(frac.shape[0]):
        x = np.where(p[k] > P_LOW, frac[k], 0.0)
        ok = prev < 1
        clear = np.where(ok, clear * (1 - np.maximum(x, prev)) / np.where(ok, 1 - prev, 1), 0.0)
        prev = x
    return 1 - clear


def cmd_thresh(run, lab, a):
    nc = ncep_profiles()
    _, pn, nlat, nlon = nc["rhum"]
    nocean = blh.oisst_ocean(nlat, nlon)
    regions = [("40–60°N DJF", 40, 60, "DJF"), ("40–60°S JJA", -60, -40, "JJA")]
    L = len(run.sigma)
    out = {}
    # NCEP band profiles per month (ocean)
    for name, lo, hi, s in regions:
        ms = SEAS[s]
        cols = {k: [] for k in ("rh", "q", "t", "omega", "ps", "cl", "cs", "month")}
        for step in year2_steps(run):
            t = step * run.dt
            mo = run.month_of(t)
            if mo not in ms:
                continue
            sel = ((run.lat >= lo) & (run.lat <= hi))[:, None] & run.ocean
            for k, f in (("rh", "rh"), ("q", "q"), ("t", "t"), ("omega", "omega"), ("cl", "cl"),
                         ("cs", "cl_split")):
                cols[k].append(load_step(run, f, step)[:, sel])
            cols["ps"].append(load_step(run, "ps", step)[0][sel])
            cols["month"].append(np.full(int(sel.sum()), mo))
        wrow = np.broadcast_to(run.w[:, None], (run.nlat, run.nlon))[
            ((run.lat >= lo) & (run.lat <= hi))[:, None] & run.ocean]
        X = {k: np.concatenate(v, axis=-1) for k, v in cols.items()}
        wt = np.tile(wrow, len(cols["ps"]))
        p = run.sigma[:, None] * X["ps"][None, :]
        sig = run.sigma[:, None] * np.ones_like(p)
        # NCEP RH and q at each column's level pressures, by month
        nrh = np.zeros_like(p)
        nq = np.zeros_like(p)
        for mo in ms:
            prof_rh = [blh.band_avg(nc["rhum"][0][mo - 1, k], nlat, np.cos(np.radians(nlat)),
                                    nocean, lo, hi) for k in range(len(pn))]
            prof_q = [blh.band_avg(nc["shum"][0][mo - 1, k], nlat, np.cos(np.radians(nlat)),
                                   nocean, lo, hi) for k in range(len(pn))]
            o = np.argsort(pn)
            mm = X["month"] == mo
            lp = np.log(p[:, mm])
            nrh[:, mm] = np.interp(lp, np.log(pn[o]), np.asarray(prof_rh)[o])
            nq[:, mm] = np.interp(lp, np.log(pn[o]), np.asarray(prof_q)[o])
        res = {"goccp_low": goccp_band(lo, hi, s, ocean=True),
               "goccp_low_all": goccp_band(lo, hi, s, ocean=False)}

        def scheme(rh, q, om, pp, sg, s_surf, fd, gate):
            """The scheme's RH low cloud (see the module doc) on levels pp, sg [L, N]."""
            c = sundqvist(rh, rh_crit(sg, s_surf))
            if fd:
                c = np.where(pp >= 75000, c * np.maximum(0.15, np.minimum(1, q / 3e-3)), c)
            if gate:
                c = np.where(sg >= 0.8, c * np.clip(-om / 0.1, 0, 1), c)
            c[-1] = np.where(sg[-1] >= 0.8, 0.0, c[-1])  # no RH cloud in the lowest layer
            return c

        # stratus and convective parts kept from the run: the run's stratiform cloud above
        # its own RH cloud (r3 knobs: s 0.95, freeze-dry on, gate on) is the LTS stratus
        c_rh_run = scheme(X["rh"], X["q"], X["omega"], p, sig, 0.95, True, True)
        cs = np.minimum(X["cs"], X["cl"])
        extra = np.clip(cs - c_rh_run, 0, 1)
        cv = np.clip(np.where(cs < 1, 1 - (1 - X["cl"]) / np.maximum(1 - cs, 1e-12), 0), 0, 1)
        recon = 1 - (1 - np.maximum(c_rh_run, cs)) * (1 - cv)
        res["recon_err_low_cover"] = float(np.average(low_cover(recon, p) - low_cover(X["cl"], p),
                                                      weights=wt))
        res["run_low_cover"] = float(np.average(low_cover(X["cl"], p), weights=wt))
        res["rh_cloud_only_low_cover_r3"] = float(np.average(low_cover(c_rh_run, p), weights=wt))
        other = 1 - (1 - extra) * (1 - cv)
        res["other_low_cover"] = float(np.average(low_cover(other, p), weights=wt))
        # humidity sources: the model; the NCEP monthly band-mean profile (no spread); the
        # model's distribution shifted to the NCEP mean at each level and month (RH additive,
        # q multiplicative); NCEP daily columns (2015–2016) with NCEP ω for the gate
        srh, sq = X["rh"].copy(), X["q"].copy()
        for mo in ms:
            mm = X["month"] == mo
            for k in range(L):
                wk = wt[mm]
                srh[k, mm] = np.clip(X["rh"][k, mm] + np.average(nrh[k, mm], weights=wk)
                                     - np.average(X["rh"][k, mm], weights=wk), 0, 1)
                sq[k, mm] = X["q"][k, mm] * (np.average(nq[k, mm], weights=wk)
                                             / np.average(X["q"][k, mm], weights=wk))
        sources = {"model": (X["rh"], X["q"], X["omega"], p, sig, wt, True),
                   "NCEP mean profile": (nrh, nq, X["omega"], p, sig, wt, True),
                   "model shifted to NCEP mean": (srh, sq, X["omega"], p, sig, wt, True)}
        daily = ncep_daily_columns(lo, hi, ms, run.sigma, float(np.average(X["ps"], weights=wt)))
        if daily is not None:
            sources["NCEP daily 2015–16"] = daily + (False,)
            res["n_daily_columns"] = int(daily[0].shape[1])
        grid = {}
        for hum, (rh, q, om, pp, sg, ww, colwise) in sources.items():
            for fd in (True, False):
                for gate in (True, False):
                    for s_surf in (0.75, 0.80, 0.85, 0.90, 0.95):
                        c_rh = scheme(rh, q, om, pp, sg, s_surf, fd, gate)
                        rh_only = float(np.average(low_cover(c_rh, pp), weights=ww))
                        if colwise:
                            c = 1 - (1 - np.maximum(c_rh, extra)) * (1 - cv)
                            tot = float(np.average(low_cover(c, pp), weights=ww))
                        else:  # random combination with the run's mean stratus + convective
                            tot = 1 - (1 - rh_only) * (1 - res["other_low_cover"])
                        grid[f"{hum}|{'on' if fd else 'off'}|{'on' if gate else 'off'}|"
                             f"{s_surf:.2f}"] = [rh_only, tot]
        res["grid"] = grid
        res["sources"] = list(sources)
        # RH distribution at the lowest levels, model against NCEP daily
        res["rh_quantiles"] = {}
        for k in range(L - 3, L):
            e = {"model": [float(x) for x in np.quantile(X["rh"][k], [0.1, 0.5, 0.9])]}
            if daily is not None:
                e["ncep_daily"] = [float(x) for x in np.quantile(daily[0][k], [0.1, 0.5, 0.9])]
            res["rh_quantiles"][f"{run.sigma[k]:.4f}"] = e
        # level means for the explanation
        res["levels"] = []
        for k in range(L):
            if run.sigma[k] < 0.75:
                continue
            res["levels"].append({
                "sigma": float(run.sigma[k]), "p_hPa": float(np.average(p[k], weights=wt) / 100),
                "rh_model": float(np.average(X["rh"][k], weights=wt)),
                "rh_ncep": float(np.average(nrh[k], weights=wt)),
                "q_model": float(np.average(X["q"][k], weights=wt) * 1e3),
                "q_ncep": float(np.average(nq[k], weights=wt) * 1e3),
                "rhc_095": float(rh_crit(run.sigma[k], 0.95)),
                "rhc_075": float(rh_crit(run.sigma[k], 0.75)),
                "share_ascent": float(np.average(X["omega"][k] < 0, weights=wt)),
                "gate_mean": float(np.average(np.clip(-X["omega"][k] / 0.1, 0, 1), weights=wt)),
                "fd_mean_model": float(np.average(np.maximum(0.15, np.minimum(1, X["q"][k] / 3e-3)),
                                                  weights=wt)),
                "fd_mean_ncep": float(np.average(np.maximum(0.15, np.minimum(1, nq[k] / 3e-3)),
                                                 weights=wt)),
                "cl_run": float(np.average(X["cl"][k], weights=wt)),
            })
        out[name] = res
        print(f"\n### {name}, ocean, {lab}: GOCCP low {res['goccp_low']:.3f} over ocean "
              f"({res['goccp_low_all']:.3f} all surfaces); run low cover {res['run_low_cover']:.3f} "
              f"(offline reconstruction {res['recon_err_low_cover']:+.4f}); of it the RH cloud alone "
              f"{res['rh_cloud_only_low_cover_r3']:.3f}, stratus + convective {res['other_low_cover']:.3f}\n")
        print("Cells: RH cloud only / with the run's stratus and convective cloud.\n")
        print("| humidity | freeze-dry | gate | " + " | ".join(
            f"RH_c,s {x}" for x in ("0.75", "0.80", "0.85", "0.90", "0.95")) + " |")
        print("|---" * 8 + "|")
        for hum in res["sources"]:
            for fd in ("on", "off"):
                for gate in ("on", "off"):
                    print(f"| {hum} | {fd} | {gate} | " + " | ".join(
                        "{:.3f} / {:.3f}".format(*grid[f"{hum}|{fd}|{gate}|{x}"])
                        for x in ("0.75", "0.80", "0.85", "0.90", "0.95")) + " |")
        print("\nRH 10 / 50 / 90 % quantiles, lowest three levels: " + "; ".join(
            f"σ {k}: model " + " / ".join(f"{x:.2f}" for x in v["model"])
            + ("; NCEP daily " + " / ".join(f"{x:.2f}" for x in v["ncep_daily"])
               if "ncep_daily" in v else "") for k, v in res["rh_quantiles"].items()))
        print("\n| σ | p | RH model | RH NCEP | RH_c (0.95) | RH_c (0.75) | q model | q NCEP | "
              "fd factor model / NCEP | ascent share | gate factor | cl run |")
        print("|---" * 12 + "|")
        for e in res["levels"]:
            print(f"| {e['sigma']:.4f} | {e['p_hPa']:.0f} | {e['rh_model']:.3f} | {e['rh_ncep']:.3f} | "
                  f"{e['rhc_095']:.3f} | {e['rhc_075']:.3f} | {e['q_model']:.2f} | {e['q_ncep']:.2f} | "
                  f"{e['fd_mean_model']:.2f} / {e['fd_mean_ncep']:.2f} | {e['share_ascent']:.2f} | "
                  f"{e['gate_mean']:.2f} | {e['cl_run']:.3f} |")
    return out


def goccp_band(lo, hi, s, ocean=False):
    import netCDF4

    from cloud_split import GOCCP

    d = netCDF4.Dataset(GOCCP)
    lat = np.asarray(d["latitude"][:], float)
    t = np.asarray(d["time"][:], float)
    month = np.array([(np.datetime64("2000-01-01") + np.timedelta64(int(round(x)), "D")).astype(
        "datetime64[M]").astype(int) % 12 + 1 for x in t])
    v = np.ma.filled(d["cllcalipso"][:].astype(float), np.nan)
    v[v < 0] = np.nan
    mon = np.nanmean(np.stack([np.nanmean(v[month == m], axis=0) for m in SEAS[s]]), axis=0)
    if ocean:
        lon = np.asarray(d["longitude"][:], float)
        mon = np.where(blh.oisst_ocean(lat, lon % 360), mon, np.nan)
    return blh.band_avg(mon, lat, np.cos(np.radians(lat)), np.isfinite(mon), lo, hi)


def ncep_daily_columns(lo, hi, months, sigma, ps):
    """NCEP/NCAR R1 daily means (2015–2016, `reference/raw/ncep_daily_p`) over the ocean of a
    band in the given months, interpolated in ln p to σ·p_s (p_s the run's band mean; below
    1000 hPa the 1000 hPa value): (rh, q, ω, p, σ, weights), arrays [L, N]."""
    import netCDF4

    dd = ROOT / "reference/raw/ncep_daily_p"
    if not all((dd / f"{v}.{y}.nc").exists() for v in ("rhum", "shum", "omega") for y in (2015, 2016)):
        return None
    out = {v: [] for v in ("rhum", "shum", "omega")}
    wts = []
    pt = sigma * ps
    for y in (2015, 2016):
        for v, sc in (("rhum", 0.01), ("shum", 1e-3), ("omega", 1.0)):
            d = netCDF4.Dataset(dd / f"{v}.{y}.nc")
            lat, lon = np.asarray(d["lat"][:], float), np.asarray(d["lon"][:], float)
            lev = np.asarray(d["level"][:], float) * 100.0
            t = netCDF4.num2date(d["time"][:], d["time"].units)
            mi = np.array([x.month in months for x in t])
            rows = (lat >= lo) & (lat <= hi)
            x = np.ma.filled(d[v][mi][:, :, rows, :].astype(float), np.nan) * sc  # [t, lev, la, lo]
            msk = blh.oisst_ocean(lat[rows], lon)
            x = x[:, :, msk]  # [t, lev, n]
            o = np.argsort(lev)
            lp = np.log(lev[o])
            # interpolate each column to the σ levels
            xi = np.empty((len(sigma), x.shape[0] * x.shape[2]))
            flat = np.moveaxis(x[:, o, :], 1, 0).reshape(len(lev), -1)
            for k, pk in enumerate(pt):
                kk = np.searchsorted(lp, np.log(pk))
                if kk >= len(lp):
                    xi[k] = flat[-1]
                elif kk == 0:
                    xi[k] = flat[0]
                else:
                    f = (np.log(pk) - lp[kk - 1]) / (lp[kk] - lp[kk - 1])
                    xi[k] = flat[kk - 1] * (1 - f) + flat[kk] * f
            out[v].append(xi)
            if v == "rhum":
                wl = np.broadcast_to(np.cos(np.radians(lat[rows]))[:, None], msk.shape)[msk]
                wts.append(np.tile(wl, x.shape[0]))
    rh = np.clip(np.concatenate(out["rhum"], axis=1), 0, 1)
    q = np.concatenate(out["shum"], axis=1)
    om = np.concatenate(out["omega"], axis=1)
    pp = pt[:, None] * np.ones_like(rh)
    sg = sigma[:, None] * np.ones_like(rh)
    ok = np.all(np.isfinite(rh) & np.isfinite(q) & np.isfinite(om), axis=0)
    w = np.concatenate(wts)
    return rh[:, ok], q[:, ok], om[:, ok], pp[:, ok], sg[:, ok], w[ok]


# ---------------------------------------------------------------- offline knob screen (3b)
SCREEN_BANDS = [("20°S–20°N", -20, 20, "ann"), ("32–57°N", 32, 57, "ann"),
                ("45–60°S", -60, -45, "ann"), ("60–77°N", 60, 77, "ann"),
                ("40–60°N DJF", 40, 60, "DJF"), ("40–60°S JJA", -60, -40, "JJA")]


def rh_cloud(rh, q, om, p, sig, s, u, n, low_sigma, fd=True):
    """The scheme's stratiform RH cloud at every level (σ ≥ 0.1), the Slingo gate at
    σ ≥ low_sigma and no RH cloud in the lowest layer there; arrays [L, N]."""
    rc = u + (s - u) * np.exp(1 - sig ** (-n))
    c = sundqvist(rh, rc)
    if fd:
        c = np.where(p >= 75000, c * np.maximum(0.15, np.minimum(1, q / 3e-3)), c)
    c = np.where(sig >= low_sigma, c * np.clip(-om / 0.1, 0, 1), c)
    c[-1] = np.where(sig[-1] >= low_sigma, 0.0, c[-1])
    return np.where(sig >= 0.1, c, 0.0)


def total_cover(frac):
    clear = np.ones(frac.shape[1])
    prev = np.zeros(frac.shape[1])
    for k in range(frac.shape[0]):
        x = frac[k]
        ok = prev < 1
        clear = np.where(ok, clear * (1 - np.maximum(x, prev)) / np.where(ok, 1 - prev, 1), 0.0)
        prev = x
    return 1 - clear


def cmd_screen(run, lab, a, knobsets):
    """Offline low and total cover by band for knob sets (s, u, n, low_sigma), the run's
    stratus and convective parts kept (as in `thresh`); the run's own knobs first."""
    L = len(run.sigma)
    res = {}
    acc = {k: {b[0]: [0.0, 0.0, 0.0] for b in SCREEN_BANDS} for k in knobsets}  # low, tot, w
    for step in year2_steps(run):
        mo = run.month_of(step * run.dt)
        F = {f: load_step(run, f, step) for f in ("rh", "q", "omega", "cl", "cl_split")}
        ps = load_step(run, "ps", step)[0]
        for b, lo, hi, s in SCREEN_BANDS:
            if mo not in SEAS[s]:
                continue
            sel = ((run.lat >= lo) & (run.lat <= hi))[:, None] & np.ones((1, run.nlon), bool)
            X = {f: F[f][:, sel] for f in F}
            p = run.sigma[:, None] * ps[sel][None, :]
            sig = run.sigma[:, None] * np.ones_like(p)
            wt = np.broadcast_to(run.w[:, None], (run.nlat, run.nlon))[sel]
            c_run = rh_cloud(X["rh"], X["q"], X["omega"], p, sig, 0.95, 0.80, 4.0, 0.8)
            cs = np.minimum(X["cl_split"], X["cl"])
            extra = np.clip(cs - c_run, 0, 1)
            cv = np.clip(np.where(cs < 1, 1 - (1 - X["cl"]) / np.maximum(1 - cs, 1e-12), 0), 0, 1)
            for ks in knobsets:
                c_rh = rh_cloud(X["rh"], X["q"], X["omega"], p, sig, *ks)
                c = 1 - (1 - np.maximum(c_rh, extra)) * (1 - cv)
                e = acc[ks][b]
                e[0] += float(np.sum(low_cover(c, p) * wt))
                e[1] += float(np.sum(total_cover(c) * wt))
                e[2] += float(np.sum(wt))
    print(f"\n### {lab}: offline cover by band (low p > 680 hPa / total), knob sets "
          "(rh_crit_surface, rh_crit_upper, rh_crit_exponent, low_sigma); stratus and "
          "convective cloud of the run kept\n")
    print("| knobs | " + " | ".join(b[0] for b in SCREEN_BANDS) + " |")
    print("|---" * (len(SCREEN_BANDS) + 1) + "|")
    for ks in knobsets:
        row = {b: (v[0] / v[2], v[1] / v[2]) for b, v in acc[ks].items()}
        res[str(ks)] = row
        print(f"| {ks} | " + " | ".join(f"{row[b[0]][0]:.3f} / {row[b[0]][1]:.3f}"
                                        for b in SCREEN_BANDS) + " |")
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["levels", "cre", "profile", "thresh", "screen"])
    ap.add_argument("runs", nargs="*")
    ap.add_argument("--labels", nargs="*")
    ap.add_argument("--label")
    ap.add_argument("--lon-stride", type=int, default=2)
    ap.add_argument("--max-snap", type=int, default=0)
    ap.add_argument("--json")
    a = ap.parse_args()
    if a.cmd == "levels":
        cmd_levels(a)
        return
    runs = [Run(ROOT / r) for r in a.runs]
    labels = a.labels or [r.path.name for r in runs]
    if a.cmd == "cre":
        out = cmd_cre(runs, labels, a)
    elif a.cmd == "profile":
        out = cmd_profile(runs, labels, a)
    elif a.cmd == "screen":
        ks = [(0.95, 0.80, 4.0, 0.8), (0.75, 0.80, 4.0, 0.8), (0.95, 0.80, 8.0, 0.8),
              (0.95, 0.75, 4.0, 0.8), (0.95, 0.80, 4.0, 0.85), (0.95, 0.80, 4.0, 0.75),
              (0.85, 0.80, 8.0, 0.8), (0.95, 0.80, 8.0, 0.85), (0.75, 0.80, 8.0, 0.85),
              (0.95, 0.80, 1.0, 0.8)]
        out = cmd_screen(runs[0], a.label or labels[0], a, ks)
    else:
        out = cmd_thresh(runs[0], a.label or labels[0], a)
    if a.json:
        pathlib.Path(a.json).write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
