"""Calibrate the semi-gray radiation knobs against RRTMG clear-sky fluxes (SPEC.md §9.4.1, P3).

    .rrtmg/bin/python -I tools/calibrate_radiation.py [--quick]

Needs the separate RRTMG environment (`.rrtmg/`, climlab + climlab-rrtmg from conda-forge, see
docs/progress.md "Dependencies") and a release build of `toyclim` (the fit evaluates the Rust
scheme itself through `toyclim radiation`).

Profile set:
- moist pseudo-adiabats (the model's water saturation curve, TJ16 eq. 1) from T_s to a 200 K
  isothermal stratosphere, relative humidity 0.5 / 0.75 / 1.0 below the tropopause and the
  tropopause specific humidity above it (as Koll & Cronin 2018);
- L26 σ levels at p_s = 0.3, 0.5 and 1 bar (fitted) and 2, 3 bar (reported only: RRTMG's
  absorption tables end at 1050 hPa, so it extrapolates there);
- longwave: T_s 250–310 K in 10 K steps, CO2 100, 280, 400, 800, 2000, 10000 ppm, other RRTMG
  gases at climlab's defaults, no ozone;
- shortwave (the Sun only: RRTMG has a fixed solar spectrum): cos zenith 0.25–1, T_s 260 / 288
  / 300 K, ocean (0.06) and bright (0.3) surfaces, 400 ppm; first without ozone (the default
  knobs), then with climlab's ozone profile for the Earth ozone column.

Writes docs/radiation/calibration.md, docs/radiation/calibration.json and
docs/radiation/calibration.png.
"""

import argparse
import json
import pathlib
import re
import subprocess
import sys
import tempfile
import warnings

import numpy as np

warnings.filterwarnings("ignore")
ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "toyclim"
OUT = ROOT / "docs" / "radiation"

# the model's constants (Planet::earth, Condensable::water)
RD, CP, G = 287.04, 1004.64, 9.80616
LV, RV, E0, T0 = 2.5e6, 461.5, 610.78, 273.16
T_STRAT = 200.0


def l26_half():
    src = (ROOT / "crates/toyclim-core/src/primitive/vertical.rs").read_text()
    body = re.search(r"L26_HALF: \[f64; 27\] = \[(.*?)\];", src, re.S).group(1)
    return np.array([float(x) for x in re.findall(r"[0-9.eE+-]+", body)])


def qsat(t, p):
    return RD / RV * E0 / p * np.exp(-(LV / RV) * (1.0 / t - 1.0 / T0))


def moist_adiabat(ts, ps, rh, half):
    """Layer T and q (top first) on σ half levels `half` × ps."""
    ph = half * ps
    p = 0.5 * (ph[1:] + ph[:-1])
    n = len(p)
    t, q = np.zeros(n), np.zeros(n)
    tc, lp = ts, np.log(ps)
    for k in range(n - 1, -1, -1):
        target = np.log(p[k])
        h = (target - lp) / 200
        for _ in range(200):
            pp = np.exp(lp)
            qs = qsat(tc, pp)
            tc += h * RD * tc / CP * (1 + LV * qs / (RD * tc)) / (1 + LV * LV * qs / (CP * RV * tc * tc))
            lp += h
        tc = max(tc, T_STRAT)
        t[k] = tc
        q[k] = rh * qsat(tc, p[k])
    trop = t > T_STRAT
    qmin = q[trop].min() if trop.any() else q.min()
    q[~trop] = np.minimum(q[~trop], qmin)
    return ph, t, q


def rrtmg(ph, t, q, ts, co2, albedo=0.06, s0=0.0, mu0=0.5, ozone=False):
    import climlab
    from climlab.domain.axis import Axis
    from climlab.domain.domain import Atmosphere, SlabOcean
    from climlab.domain.field import Field

    lev = Axis(axis_type="lev", bounds=ph / 100.0)
    atm = Atmosphere(axes=lev)
    sfc = SlabOcean(axes={"depth": Axis(axis_type="depth", bounds=[0, 1.0])})
    st = {"Tatm": Field(t.copy(), domain=atm), "Ts": Field([ts], domain=sfc)}
    r = climlab.radiation.RRTMG(state=st, specific_humidity=q.copy(), albedo=albedo,
                                insolation=max(s0, 1e-9), coszen=mu0)
    r.absorber_vmr["CO2"] = co2 * 1e-6
    if not ozone:
        r.absorber_vmr["O3"] = 0.0 * r.absorber_vmr["O3"]
    r.compute_diagnostics()
    d = r.diagnostics
    sw_net = np.asarray(d["SW_flux_net_clr"]).ravel()  # downward net, top first
    return {
        "olr": float(np.asarray(d["OLRclr"]).ravel()[0]),
        "lw_down_sfc": float(np.asarray(d["LW_flux_down_clr"]).ravel()[-1]),
        "sw_up_top": float(np.asarray(d["SW_flux_up_clr"]).ravel()[0]),
        "sw_down_top": float(np.asarray(d["SW_flux_down_clr"]).ravel()[0]),
        "sw_atm": float(sw_net[0] - sw_net[-1]),
    }


def column(ph, t, q, ts, co2, albedo=0.06, s0=0.0, mu0=0.5):
    return {"p_half": ph.tolist(), "t": t.tolist(), "q": q.tolist(), "ts": ts,
            "albedo": [albedo, albedo], "s0": s0, "mu0": mu0, "t_eff": 5772.0, "co2_ppm": co2}


def toyclim(knobs, cols, tmp):
    kf, cf = tmp / "knobs.toml", tmp / "cols.json"
    lines = []
    for k, v in knobs.items():
        val = json.dumps([float(x) for x in v]) if np.ndim(v) else repr(float(v))
        lines.append(f"{k} = {val}")
    kf.write_text("\n".join(lines) + "\n")
    if not cf.exists() or cf.stat().st_size == 0 or getattr(toyclim, "_cols", None) is not cols:
        cf.write_text(json.dumps(cols))
        toyclim._cols = cols
    r = subprocess.run([str(BIN), "radiation", "--knobs", str(kf), "--columns", str(cf)],
                       capture_output=True, text=True, check=True, env={"RUST_LOG": "error"})
    return json.loads(r.stdout)


def main():
    a = argparse.ArgumentParser()
    a.add_argument("--quick", action="store_true", help="smaller profile set (testing)")
    a.add_argument("--forcing-scale", type=float, default=1.0,
                   help="residual scale of the CO2 forcing pairs [W m-2] (P3 fit: 1.0; smaller = heavier)")
    a.add_argument("--slope-scale", type=float, default=2.0,
                   help="residual scale of the T_s pairs (clear-sky response) [W m-2] (P3 fit: 2.0)")
    a.add_argument("--ts-max", type=float, default=310.0, help="warmest surface of the profile set [K] (P3: 310)")
    a.add_argument("--out", default=str(OUT), help="output directory (default docs/radiation)")
    a.add_argument("--co2-band", action="store_true",
                   help="CO2 as the band of Jeevanjee et al. (2021) (lw_co2_band = [l, c_ref, kappa0, c_win] fitted); "
                        "the ln(CO2/360) terms d, d_w, d_strong, lw_n_co2 and lw_co2_curvature are fixed at 0")
    a.add_argument("--state-cols", nargs="*", default=[],
                   help="npz files of tools/state_columns.py: model-state columns whose RRTMG forcing (by latitude "
                        "band) and OLR (global mean) are added to the longwave fit")
    a.add_argument("--start", help="calibration.json whose longwave knobs start the --co2-band fit "
                   "(default: docs/radiation, the band at the paper's values)")
    a.add_argument("--state-scale", type=float, default=0.2,
                   help="residual scale of the state forcing by band [W m-2]; the state OLR uses 5x this")
    args = a.parse_args()
    from scipy.optimize import least_squares

    if not BIN.exists():
        sys.exit(f"{BIN} missing: cargo build --release")
    half = l26_half()
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="toyclim_cal_"))

    out_dir = pathlib.Path(args.out)
    tss = [250, 270, 290, 310] if args.quick else list(range(250, int(args.ts_max) + 1, 10))
    co2s = [100, 400, 2000] if args.quick else [100, 280, 400, 800, 2000, 10000]
    rhs = [0.5, 0.75, 1.0]
    fit_ps, extra_ps = [0.3e5, 0.5e5, 1e5], [2e5, 3e5]

    # ---- longwave reference
    lw = []
    for ps in fit_ps + extra_ps:
        for ts in tss:
            for rh in rhs:
                ph, t, q = moist_adiabat(ts, ps, rh, half)
                for co2 in co2s:
                    ref = rrtmg(ph, t, q, ts, co2)
                    lw.append({"ps": ps, "ts": ts, "rh": rh, "co2": co2, "ref": ref,
                               "col": column(ph, t, q, ts, co2)})
    print(f"longwave: {len(lw)} RRTMG columns", flush=True)
    fit_lw = [c for c in lw if c["ps"] in fit_ps]
    cols_lw = [c["col"] for c in fit_lw]
    ref_olr = np.array([c["ref"]["olr"] for c in fit_lw])
    ref_dn = np.array([c["ref"]["lw_down_sfc"] for c in fit_lw])

    def lw_knobs(x):
        k = {"lw_rest": x[0:4], "lw_window": x[4:8], "lw_n": x[8], "lw_n_co2": x[9],
             "lw_rest_strong": x[10:14], "lw_strong_share": x[14], "lw_co2_curvature": x[15]}
        if args.co2_band:
            k["lw_co2_band"] = x[16:20]
        return k

    # model-state columns (P4 question 11): RRTMG OLR at two CO2 amounts
    states = []
    for f in args.state_cols:
        z = np.load(f)
        lo_c, hi_c = (float(c) for c in z["co2"])
        cs = [column(z["ph"][n], z["t"][n], z["q"][n], float(z["ts"][n]), c) for c in (lo_c, hi_c)
              for n in range(len(z["ts"]))]
        bands = [(np.abs(z["lat"]) >= b0) & (np.abs(z["lat"]) < b1) for b0, b1 in [(0, 30), (30, 60), (60, 90)]]
        states.append({"name": pathlib.Path(f).stem, "cols": cs, "n": len(z["ts"]), "w": z["w"], "bands": bands,
                       "F_ref": z["olr_lo"] - z["olr_hi"], "olr_ref": z["olr_lo"]})
    cols_all = cols_lw + [c for st in states for c in st["cols"]]

    def state_diag(olr_all):
        out, i = [], len(cols_lw)
        for st in states:
            n = st["n"]
            o_lo, o_hi = olr_all[i:i + n], olr_all[i + n:i + 2 * n]
            i += 2 * n
            F = o_lo - o_hi
            fb = [((F - st["F_ref"]) * st["w"])[b].sum() / st["w"][b].sum() for b in st["bands"]]
            out.append({"name": st["name"], "F": float((F * st["w"]).sum()), "F_ref": float((st["F_ref"] * st["w"]).sum()),
                        "F_band_err": fb, "olr": float((o_lo * st["w"]).sum()),
                        "olr_ref": float((st["olr_ref"] * st["w"]).sum())})
        return out

    # sensitivities: OLR differences between neighbouring CO2 values (forcing) and between
    # neighbouring surface temperatures (the clear-sky feedback), against RRTMG
    idx = {(c["ps"], c["ts"], c["rh"], c["co2"]): i for i, c in enumerate(fit_lw)}
    pairs_co2, pairs_ts = [], []
    for (ps, ts, rh, co2), i in idx.items():
        k = co2s.index(co2)
        if k + 1 < len(co2s):
            pairs_co2.append((i, idx[(ps, ts, rh, co2s[k + 1])]))
        k = tss.index(ts)
        if k + 1 < len(tss):
            pairs_ts.append((i, idx[(ps, tss[k + 1], rh, co2)]))
    pc, pt = np.array(pairs_co2), np.array(pairs_ts)

    def lw_res(x):
        r = toyclim(lw_knobs(x), cols_all, tmp)
        olr_all = np.array([c["olr"] for c in r])
        sd = state_diag(olr_all)
        st_res = [e / args.state_scale for d in sd for e in d["F_band_err"]] + \
                 [(d["olr"] - d["olr_ref"]) / (5 * args.state_scale) for d in sd]
        # OLR differences between consecutive state sets (cold -> warm: the clear-sky
        # response; stratosphere swaps: its effect on the OLR), scale --state-scale
        st_res += [((b["olr"] - a_["olr"]) - (b["olr_ref"] - a_["olr_ref"])) / args.state_scale
                   for a_, b in zip(sd[:-1], sd[1:])]
        r = r[:len(cols_lw)]
        olr = np.array([c["olr"] for c in r])
        dn = np.array([c["lw_down_sfc"] for c in r])
        f_m = olr[pc[:, 0]] - olr[pc[:, 1]]
        f_r = ref_olr[pc[:, 0]] - ref_olr[pc[:, 1]]
        s_m = olr[pt[:, 1]] - olr[pt[:, 0]]
        s_r = ref_olr[pt[:, 1]] - ref_olr[pt[:, 0]]
        return np.concatenate([(olr - ref_olr) / 8.0, (dn - ref_dn) / 8.0,
                               (f_m - f_r) / args.forcing_scale, (s_m - s_r) / args.slope_scale, st_res])

    x0 = np.array([0.1, 23.8, 254.0, 0.2023, 0.2150, 147.11, 1.0814e4, 0.0954, 0.0, 0.0,
                   2.0, 200.0, 254.0, 1.0, 0.4, 0.0])
    lo = np.array([0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, -0.5])
    hi = np.array([5, 500, 1e6, 2, 5, 5e3, 1e6, 2, 2, 3, 50, 5e3, 1e6, 10, 0.95, 0.5])
    if args.co2_band:
        # start from the fitted P3 knobs, the band at the paper's values (Table 1) and c_ref 1 ppm
        p3 = json.loads(pathlib.Path(args.start or OUT / "calibration.json").read_text())["knobs"]
        band = list(p3.get("lw_co2_band", [10.2, 1.0, 50.0]))
        band += [band[1]] * (4 - len(band))
        x0 = np.array(list(p3["lw_rest"]) + list(p3["lw_window"]) + [p3["lw_n"], 0.0] + list(p3["lw_rest_strong"])
                      + [p3["lw_strong_share"], 0.0] + band)
        x0[[3, 7, 13]] = 0.0
        lo = np.append(lo, [3.0, 0.01, 1.0, 0.01])
        hi = np.append(hi, [30.0, 300.0, 1000.0, 400.0])
        free = np.ones(len(x0), bool)
        free[[3, 7, 9, 13, 15]] = False
    else:
        free = np.ones(len(x0), bool)
    x0 = np.clip(x0, lo, hi)

    def _expand(y):
        x = x0.copy()
        x[free] = y
        return x

    s = least_squares(lambda y: lw_res(_expand(y)), x0[free], bounds=(lo[free], hi[free]),
                      x_scale=np.maximum(np.abs(x0[free]), 0.05), max_nfev=400)
    xl = _expand(s.x)
    sdiag = state_diag(np.array([c["olr"] for c in toyclim(lw_knobs(xl), cols_all, tmp)]))
    for d in sdiag:
        print(f"state {d['name']}: F {d['F']:.3f} (RRTMG {d['F_ref']:.3f}), band errors "
              f"{np.round(d['F_band_err'], 3)}, OLR {d['olr']:.2f} (RRTMG {d['olr_ref']:.2f})", flush=True)
    print("longwave fit:", np.round(xl, 5), "cost", s.cost, flush=True)

    # ---- shortwave reference (Sun)
    def sw_set(ozone):
        out = []
        for ps in fit_ps:
            for ts in [260, 288, 300]:
                for rh in rhs:
                    ph, t, q = moist_adiabat(ts, ps, rh, half)
                    for mu0 in [0.25, 0.5, 0.75, 1.0]:
                        for alb in [0.06, 0.3]:
                            s0 = 1361.0 * mu0
                            ref = rrtmg(ph, t, q, ts, 400, albedo=alb, s0=s0, mu0=mu0, ozone=ozone)
                            out.append({"ps": ps, "ts": ts, "rh": rh, "mu0": mu0, "alb": alb, "ref": ref,
                                        "col": column(ph, t, q, ts, 400, albedo=alb, s0=s0, mu0=mu0)})
        return out

    sw0 = sw_set(False)
    print(f"shortwave: {len(sw0)} RRTMG columns; TOA down / s0 = "
          f"{sw0[0]['ref']['sw_down_top'] / sw0[0]['col']['s0']:.4f}", flush=True)
    cols_sw = [c["col"] for c in sw0]
    ref_atm = np.array([c["ref"]["sw_atm"] for c in sw0])
    ref_up = np.array([c["ref"]["sw_up_top"] for c in sw0])

    def sw_knobs(x):
        return {"rayleigh": x[0:3], "k_wv": x[3:5], "nir_weak_share": x[5], "wv_n": x[6],
                "dry_abs": x[7:9]}

    def sw_res(x, cols=cols_sw, ra=ref_atm, ru=ref_up, extra=None):
        k = sw_knobs(x)
        if extra:
            k.update(extra)
        r = toyclim(k, cols, tmp)
        atm = np.array([c["sw_atm"] for c in r])
        up = np.array([c["sw_up_top"] for c in r])
        # absorption: relative (10 % target); reflected: 5 W m-2 scale
        return np.concatenate([(atm - ra) / (0.10 * ra), (up - ru) / 5.0])

    y0 = np.array([0.12, 0.01, 0.0, 0.002, 0.1, 0.6, 0.0, 0.01, 0.05])
    s = least_squares(sw_res, y0, bounds=([0.03, 0, 0, 0, 0, 0.05, 0, 0, 0], [0.4, 0.02, 0.02, 1, 50, 0.95, 2, 2, 5]),
                      x_scale=[0.05, 0.01, 0.01, 0.002, 0.1, 0.1, 0.2, 0.01, 0.05], max_nfev=400)
    ys = s.x
    print("shortwave fit:", np.round(ys, 5), flush=True)

    # Earth ozone column (the other knobs fixed)
    sw3 = sw_set(True)
    cols3 = [c["col"] for c in sw3]
    ra3 = np.array([c["ref"]["sw_atm"] for c in sw3])
    ru3 = np.array([c["ref"]["sw_up_top"] for c in sw3])

    def oz_res(z):
        return sw_res(ys, cols3, ra3, ru3, {"ozone_tau": z[0], "ozone_p": z[1]})

    s = least_squares(oz_res, [0.03, 1000.0], bounds=([0, 100], [1, 20000]), x_scale=[0.01, 500])
    oz = s.x
    print("ozone fit:", np.round(oz, 5), flush=True)

    # ---- residuals
    knobs = {**lw_knobs(xl), **sw_knobs(ys)}
    r_lw = toyclim(knobs, [c["col"] for c in lw], tmp)
    r_sw = toyclim(knobs, cols_sw, tmp)
    r_sw3 = toyclim({**knobs, "ozone_tau": oz[0], "ozone_p": oz[1]}, cols3, tmp)
    rows = []
    for c, r in zip(lw, r_lw):
        rows.append({"ps": c["ps"], "ts": c["ts"], "rh": c["rh"], "co2": c["co2"],
                     "olr": r["olr"], "olr_ref": c["ref"]["olr"],
                     "lw_down_sfc": r["lw_down_sfc"], "lw_down_sfc_ref": c["ref"]["lw_down_sfc"]})
    swrows = []
    for tag, set_, res in [("no_ozone", sw0, r_sw), ("ozone", sw3, r_sw3)]:
        for c, r in zip(set_, res):
            swrows.append({"set": tag, "ps": c["ps"], "ts": c["ts"], "rh": c["rh"], "mu0": c["mu0"],
                           "albedo": c["alb"], "sw_atm": r["sw_atm"], "sw_atm_ref": c["ref"]["sw_atm"],
                           "sw_up_top": r["sw_up_top"], "sw_up_top_ref": c["ref"]["sw_up_top"]})

    def stats(sel):
        d1 = np.array([x["olr"] - x["olr_ref"] for x in sel])
        d2 = np.array([x["lw_down_sfc"] - x["lw_down_sfc_ref"] for x in sel])
        return np.abs(d1).max(), np.sqrt((d1 ** 2).mean()), np.abs(d2).max(), np.sqrt((d2 ** 2).mean())

    report = {"knobs": {k: (list(map(float, v)) if np.ndim(v) else float(v)) for k, v in knobs.items()},
              "earth_ozone": {"ozone_tau": float(oz[0]), "ozone_p": float(oz[1])},
              "lw_rows": rows, "sw_rows": swrows, "states": sdiag}
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "calibration.json").write_text(json.dumps(report, indent=1))

    # forcing and slope diagnostics at 1 bar, RH 0.75
    def pick(ps, rh, co2, ts):
        return next(x for x in rows if x["ps"] == ps and x["rh"] == rh and x["co2"] == co2 and x["ts"] == ts)

    lines = ["# Semi-gray radiation calibration (P3)", "",
             "Generated by `tools/calibrate_radiation.py` (RRTMG via climlab "
             f"{__import__('climlab').__version__}). Clear sky; moist adiabats to a 200 K "
             "stratosphere, L26 levels. Residual = toyclim − RRTMG.", "",
             "## Fitted knobs", "", "```toml"]
    for k, v in report["knobs"].items():
        lines.append(f"{k} = {json.dumps(v) if isinstance(v, list) else v}")
    lines += [f"# Earth: ozone_tau = {oz[0]:.5f}, ozone_p = {oz[1]:.1f}", "```", "",
              "## Longwave residuals [W m⁻²]", "",
              "| p_s | columns | OLR max | OLR rms | LW↓sfc max | LW↓sfc rms |", "|---|---|---|---|---|---|"]
    for ps in fit_ps + extra_ps:
        sel = [x for x in rows if x["ps"] == ps]
        m1, r1, m2, r2 = stats(sel)
        tag = " (fitted)" if ps in fit_ps else " (extrapolated reference)"
        lines.append(f"| {ps / 1e5:.1f} bar{tag} | {len(sel)} | {m1:.2f} | {r1:.2f} | {m2:.2f} | {r2:.2f} |")
    lines += ["", "## Shortwave residuals (Sun, 400 ppm)", "",
              "| set | atmospheric absorption: max relative error | rms relative | reflected at top: max abs [W m⁻²] |",
              "|---|---|---|---|"]
    for tag in ["no_ozone", "ozone"]:
        sel = [x for x in swrows if x["set"] == tag]
        rel = np.array([(x["sw_atm"] - x["sw_atm_ref"]) / x["sw_atm_ref"] for x in sel])
        up = np.array([x["sw_up_top"] - x["sw_up_top_ref"] for x in sel])
        lines.append(f"| {tag} | {np.abs(rel).max():.3f} | {np.sqrt((rel ** 2).mean()):.3f} | {np.abs(up).max():.2f} |")
    if 400 in co2s and 800 in co2s:
        lines += ["", "## Checks at 1 bar, RH 0.75", "",
                  "| quantity | toyclim | RRTMG |", "|---|---|---|"]
        o288, o288r = pick(1e5, 0.75, 400, 290)["olr"], pick(1e5, 0.75, 400, 290)["olr_ref"]
        lines.append(f"| OLR at T_s 290 K, 400 ppm | {o288:.1f} | {o288r:.1f} |")
        f2 = pick(1e5, 0.75, 400, 290)["olr"] - pick(1e5, 0.75, 800, 290)["olr"]
        f2r = pick(1e5, 0.75, 400, 290)["olr_ref"] - pick(1e5, 0.75, 800, 290)["olr_ref"]
        lines.append(f"| ΔOLR 400 → 800 ppm at 290 K | {f2:.2f} | {f2r:.2f} |")
        sl = (pick(1e5, 0.75, 400, 300)["olr"] - pick(1e5, 0.75, 400, 270)["olr"]) / 30
        slr = (pick(1e5, 0.75, 400, 300)["olr_ref"] - pick(1e5, 0.75, 400, 270)["olr_ref"]) / 30
        lines.append(f"| dOLR/dT_s 270–300 K [W m⁻² K⁻¹] | {sl:.2f} | {slr:.2f} |")
    (out_dir / "calibration.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))

    # figure
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(1, 3, figsize=(16, 4.8), constrained_layout=True)
    for co2, col in zip([100, 400, 2000, 10000], ["C0", "C1", "C2", "C3"]):
        sel = [x for x in rows if x["ps"] == 1e5 and x["rh"] == 0.75 and x["co2"] == co2]
        if not sel:
            continue
        ts = [x["ts"] for x in sel]
        ax[0].plot(ts, [x["olr_ref"] for x in sel], col + "--")
        ax[0].plot(ts, [x["olr"] for x in sel], col + "-", label=f"{co2} ppm")
        ax[1].plot(ts, [x["lw_down_sfc_ref"] for x in sel], col + "--")
        ax[1].plot(ts, [x["lw_down_sfc"] for x in sel], col + "-", label=f"{co2} ppm")
    ax[0].set_title("clear-sky OLR, 1 bar, RH 0.75 (solid toyclim, dashed RRTMG)")
    ax[1].set_title("surface downward longwave")
    for a_ in ax[:2]:
        a_.set_xlabel("T_s [K]")
        a_.set_ylabel("W m⁻²")
        a_.legend()
    for tag, mk in [("no_ozone", "o"), ("ozone", "x")]:
        sel = [x for x in swrows if x["set"] == tag]
        ax[2].scatter([x["sw_atm_ref"] for x in sel], [x["sw_atm"] for x in sel], marker=mk, s=12, label=tag)
    m = max(x["sw_atm_ref"] for x in swrows)
    ax[2].plot([0, m], [0, m], "k-", lw=0.5)
    ax[2].plot([0, m], [0, 1.1 * m], "k:", lw=0.5)
    ax[2].plot([0, m], [0, 0.9 * m], "k:", lw=0.5)
    ax[2].set_xlabel("RRTMG atmospheric SW absorption [W m⁻²]")
    ax[2].set_ylabel("toyclim")
    ax[2].set_title("shortwave absorption (dotted ±10 %)")
    ax[2].legend()
    fig.savefig(out_dir / "calibration.png", dpi=90)


if __name__ == "__main__":
    main()
