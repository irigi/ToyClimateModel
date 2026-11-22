"""RRTMG single-column radiative–convective equilibrium on the L26 levels, for comparison with
the toyclim SCM RCE (cases/bench/p3_rce/rce_{400,800}.toml; P4 question 11, stratospheric
check).

    .rrtmg/bin/python -I tools/rce_rrtmg.py [--co2 400 800] [--out out/p4/q11/rce/rrtmg.json]

The setup follows rce_400.toml where climlab allows: L26 σ levels at p_s = 1000 hPa; Sun,
insolation 340.25 W m⁻² at cos zenith 0.5; a 2 m slab surface; planetary albedo 0.3 (here by
the surface albedo, found by secant iteration at the first CO₂ amount and then held); fixed
relative humidity 0.7 below the stratosphere (climlab FixedRelativeHumidity, minimum
q = 5e-6); convective adjustment to the moist adiabat (climlab "MALR"); climlab's default
ozone profile and trace gases; time step 1 day; 6 years, then checked for equilibrium.
Differences from the toyclim SCM: convective adjustment instead of SBM + boundary layer, and
RRTMG's ozone profile instead of the semi-gray ozone layer. The comparison of interest is
the change of the stratospheric temperature with CO₂, not its absolute value.
"""

import argparse
import json
import pathlib
import sys
import warnings

import numpy as np

warnings.filterwarnings("ignore")
ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from calibrate_radiation import l26_half  # noqa: E402


def run(co2, albedo, years=6.0):
    import climlab
    from climlab.domain.axis import Axis
    from climlab.domain.domain import Atmosphere, SlabOcean
    from climlab.domain.field import Field
    from climlab.radiation.water_vapor import FixedRelativeHumidity

    ph = l26_half() * 1000.0
    lev = Axis(axis_type="lev", bounds=ph)
    atm = Atmosphere(axes=lev)
    sfc = SlabOcean(axes={"depth": Axis(axis_type="depth", bounds=[0, 2.0])})
    n = len(ph) - 1
    p = 0.5 * (ph[1:] + ph[:-1])
    t0 = np.maximum(200.0, 288.0 * (p / 1000.0) ** 0.19)
    state = {"Tatm": Field(t0, domain=atm), "Ts": Field([288.0], domain=sfc)}
    h2o = FixedRelativeHumidity(state=state, relative_humidity=0.7, timestep=86400.0, name="h2o")
    conv = climlab.convection.ConvectiveAdjustment(state=state, adj_lapse_rate="MALR", timestep=86400.0, name="conv")
    rad = climlab.radiation.RRTMG(state=state, specific_humidity=h2o.q, albedo=albedo, insolation=340.25,
                                  coszen=0.5, timestep=86400.0, name="rad")
    rad.absorber_vmr["CO2"] = co2 * 1e-6
    model = climlab.couple([rad, h2o, conv], name="rce")
    model.integrate_years(years, verbose=False)
    d = rad.diagnostics
    asr = float(np.asarray(d["ASR"]).ravel()[0])
    olr = float(np.asarray(d["OLR"]).ravel()[0])
    up = float(np.asarray(d["SW_flux_up"]).ravel()[0])
    dn = float(np.asarray(d["SW_flux_down"]).ravel()[0])
    return {"co2": co2, "surface_albedo": albedo, "ts": float(np.asarray(model.Ts).ravel()[0]),
            "p_hpa": p.tolist(), "t": np.asarray(model.Tatm).ravel().tolist(),
            "planetary_albedo": up / dn, "toa_imbalance": asr - olr, "n": n}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--co2", type=float, nargs="+", default=[400.0, 800.0])
    ap.add_argument("--out", default=str(ROOT / "out/p4/q11/rce/rrtmg.json"))
    a = ap.parse_args()
    # planetary albedo 0.3 at the first CO2 amount (secant on the surface albedo)
    x0, x1 = 0.2, 0.25
    f0 = run(a.co2[0], x0, 2.0)["planetary_albedo"] - 0.3
    for _ in range(6):
        f1 = run(a.co2[0], x1, 2.0)["planetary_albedo"] - 0.3
        if abs(f1) < 5e-4:
            break
        x0, x1, f0 = x1, x1 - f1 * (x1 - x0) / (f1 - f0), f1
    res = [run(c, x1) for c in a.co2]
    for r in res:
        print(f"{r['co2']:g} ppm: T_s {r['ts']:.2f} K, planetary albedo {r['planetary_albedo']:.3f}, "
              f"TOA imbalance {r['toa_imbalance']:+.3f} W m-2 (surface albedo {r['surface_albedo']:.3f})")
    pathlib.Path(a.out).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(a.out).write_text(json.dumps(res, indent=1))
    lo, hi = res[0], res[-1]
    for k, p in enumerate(lo["p_hpa"]):
        if p < 300:
            print(f"{p:7.1f} hPa  {lo['t'][k]:6.1f} -> {hi['t'][k]:6.1f}  dT {hi['t'][k] - lo['t'][k]:+5.1f}")


if __name__ == "__main__":
    main()
