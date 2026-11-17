"""Model vs reference maps for an earth_amip run (figures for the scorecard).

    python tools/earth_maps.py out/p2/earth_amip_t42 docs/figures/P2/earth_amip_t42.png

Rows: annual precipitation (model, GPCP), annual land 2 m temperature (model lapse-rate
corrected to the reference orography, NCEP), Köppen main group (model, Beck et al. 2018),
zonal means of precipitation, 2 m temperature and 200 hPa wind (DJF, JJA).
Needs `tools/scorecard.py --suite earth_amip` to have written climatology/koppen_earth_amip.npy.
"""

import json
import pathlib
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.colors import BoundaryNorm, ListedColormap  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent


def main():
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    meta = json.loads((run / "meta.json").read_text())
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"]))
    c = run / "climatology"
    ref = ROOT / "reference" / "derived" / f"t{meta['truncation']}"
    R = lambda k: np.load(ref / f"{k}.npy").astype(float)  # noqa: E731
    M = lambda k: np.load(c / f"monthly_{k}.npy").astype(float)  # noqa: E731
    land = R("land") >= 0.5
    orog_f = sorted(run.glob("fields/orog_*.npy"))
    zmod = np.load(orog_f[0]) if orog_f else R("orog")
    tas_m = M("tas").mean(0) + 0.0065 * (zmod - R("orog_tas_ref"))
    tas_r = R("tas").mean(0)
    pm, pr = M("precip").mean(0) * 86400, R("pr").mean(0)
    # Köppen main group 1..5 = A B C D E (Beck codes 1-3 A, 4-7 B, 8-16 C, 17-28 D, 29-30 E)
    grp = np.array([0, 1, 1, 1, 2, 2, 2, 2] + [3] * 9 + [4] * 12 + [5] * 2)
    km = grp[np.load(c / "koppen_earth_amip.npy")]
    kr = grp[R("koppen_beck").astype(int)]
    km = np.where(land, km, 0)
    kr = np.where(land, kr, 0)

    fig, ax = plt.subplots(4, 2, figsize=(13, 15), constrained_layout=True)
    ext = dict(extent=(lon[0], lon[-1], lat[-1], lat[0]), aspect="auto", interpolation="nearest")

    def show(a, f, title, **kw):
        im = a.imshow(f, **ext, **kw)
        a.set_title(title)
        a.contour(lon, lat, land.astype(float), [0.5], colors="k", linewidths=0.5)
        return im

    pk = dict(cmap="YlGnBu", vmin=0, vmax=12)
    show(ax[0, 0], pm, "model precipitation [mm/day]", **pk)
    im = show(ax[0, 1], pr, "GPCP precipitation [mm/day]", **pk)
    fig.colorbar(im, ax=ax[0, :], shrink=0.8)
    tk = dict(cmap="RdYlBu_r", vmin=230, vmax=305)
    show(ax[1, 0], np.where(land, tas_m, np.nan), "model land 2 m T, lapse-corrected [K]", **tk)
    im = show(ax[1, 1], np.where(land, tas_r, np.nan), "NCEP land 2 m T [K]", **tk)
    fig.colorbar(im, ax=ax[1, :], shrink=0.8)
    kc = ListedColormap(["#d9eaf7", "#1a7a1a", "#e8b04a", "#7fbf3f", "#7a5fb0", "#9fb6c8"])
    kn = BoundaryNorm(np.arange(-0.5, 6.5), 6)
    show(ax[2, 0], km, "model Köppen main group", cmap=kc, norm=kn)
    im = show(ax[2, 1], kr, "Beck et al. 2018 Köppen main group", cmap=kc, norm=kn)
    cb = fig.colorbar(im, ax=ax[2, :], shrink=0.8, ticks=range(6))
    cb.ax.set_yticklabels(["sea", "A", "B", "C", "D", "E"])

    a = ax[3, 0]
    a.plot(lat, pm.mean(1), "C0", label="model precip [mm/day]")
    a.plot(lat, pr.mean(1), "C0--", label="GPCP")
    a.set_xlabel("latitude")
    a.legend(loc="upper left")
    b = a.twinx()
    b.plot(lat, M("tas").mean(0).mean(1), "C3", label="model 2 m T [K]")
    b.plot(lat, R("tas").mean(0).mean(1), "C3--", label="NCEP")
    b.legend(loc="upper right")
    a.set_title("zonal means, annual")
    a = ax[3, 1]
    for ms, nm, col in [([11, 0, 1], "DJF", "C0"), ([5, 6, 7], "JJA", "C1")]:
        a.plot(lat, M("u200")[ms].mean(0).mean(1), col, label=f"model {nm}")
        a.plot(lat, R("ua200")[ms].mean(0).mean(1), col + "--", label=f"NCEP {nm}")
    a.axhline(0, color="k", lw=0.5)
    a.set_title("zonal-mean u at 200 hPa [m/s]")
    a.set_xlabel("latitude")
    a.legend()
    for a in ax[:3].ravel():
        a.set_xlabel("longitude")
        a.set_ylabel("latitude")
    fig.suptitle(f"{run.name}: model vs reference (annual means unless stated)")
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=90)
    print(out)


if __name__ == "__main__":
    main()
