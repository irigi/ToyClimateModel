#!/usr/bin/env python3
"""Köppen–Geiger climate classification from a run's monthly climatology.

Usage:
    python tools/koppen.py out/<run> [-o map.png] [--temp tas] [--precip precip]
        [--land-only]

Needs `[climatology] monthly = true` with maps of near-surface air temperature (`tas`,
K) and precipitation (`precip`, kg m-2 s-1). Rules follow Peel, Finlayson & McMahon
(2007, HESS 11, 1633–1644), with one generalisation for other planets: "summer" is the
half of the calendar year (consecutive months) with the highest mean temperature at
each grid cell, instead of fixed Earth months. Precipitation is converted to mm per
calendar month using the calendar's year length.

Writes the class map (PNG) and `climatology/koppen.npy` (int codes, see CLASSES) and
prints the area fraction of each class.
"""
import argparse
import json
import pathlib

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from matplotlib.colors import ListedColormap  # noqa: E402

# Codes and the usual colours (Peel et al. 2007 Fig.)
CLASSES = [
    ("Af", "#0000ff"), ("Am", "#0078ff"), ("Aw", "#46aafa"),
    ("BWh", "#ff0000"), ("BWk", "#ff9696"), ("BSh", "#f5a500"), ("BSk", "#ffdc64"),
    ("Csa", "#ffff00"), ("Csb", "#c8c800"), ("Csc", "#969600"),
    ("Cwa", "#96ff96"), ("Cwb", "#64c864"), ("Cwc", "#329632"),
    ("Cfa", "#c8ff50"), ("Cfb", "#64ff50"), ("Cfc", "#32c800"),
    ("Dsa", "#ff00ff"), ("Dsb", "#c800c8"), ("Dsc", "#963296"), ("Dsd", "#966496"),
    ("Dwa", "#aaafff"), ("Dwb", "#5a78dc"), ("Dwc", "#4b50b4"), ("Dwd", "#320087"),
    ("Dfa", "#00ffff"), ("Dfb", "#37c8ff"), ("Dfc", "#007d7d"), ("Dfd", "#00465f"),
    ("ET", "#b2b2b2"), ("EF", "#666666"),
]
CODE = {c: i for i, (c, _) in enumerate(CLASSES)}


def classify(T, P):
    """T: monthly mean temperature [°C], P: monthly precipitation [mm], shape (12,)."""
    nm = len(T)
    half = nm // 2
    # summer = the `half` consecutive months with the highest mean temperature
    best, start = -1e9, 0
    for s in range(nm):
        idx = [(s + i) % nm for i in range(half)]
        m = T[idx].mean()
        if m > best:
            best, start = m, s
    summer = np.zeros(nm, bool)
    summer[[(start + i) % nm for i in range(half)]] = True
    winter = ~summer
    MAT, MAP = T.mean(), P.sum()
    Thot, Tcold = T.max(), T.min()
    Tmon10 = int((T > 10).sum())
    Pdry = P.min()
    Psdry, Pswet = P[summer].min(), P[summer].max()
    Pwdry, Pwwet = P[winter].min(), P[winter].max()
    Psum, Pwin = P[summer].sum(), P[winter].sum()
    if Pwin >= 0.7 * MAP:
        Pth = 2 * MAT
    elif Psum >= 0.7 * MAP:
        Pth = 2 * MAT + 28
    else:
        Pth = 2 * MAT + 14
    if Thot < 10:
        return "ET" if Thot > 0 else "EF"
    if MAP < 10 * Pth:
        return ("BW" if MAP < 5 * Pth else "BS") + ("h" if MAT >= 18 else "k")
    if Tcold >= 18:
        if Pdry >= 60:
            return "Af"
        return "Am" if Pdry >= 100 - MAP / 25 else "Aw"
    first = "C" if Tcold > 0 else "D"
    if Psdry < 40 and Psdry < Pwwet / 3:
        second = "s"
    elif Pwdry < Pswet / 10:
        second = "w"
    else:
        second = "f"
    if Thot >= 22:
        third = "a"
    elif Tmon10 >= 4:
        third = "b"
    elif first == "D" and Tcold < -38:
        third = "d"
    else:
        third = "c"
    return first + second + third


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("-o", "--output", default=None)
    ap.add_argument("--temp", default="tas")
    ap.add_argument("--precip", default="precip")
    ap.add_argument("--mask", default=None, help=".npy land mask on the model grid (1 = land) to grey out oceans")
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    meta = json.loads((run / "meta.json").read_text())
    c = run / "climatology"
    cal = meta["calendar"]
    T = np.load(c / f"monthly_{a.temp}.npy") - 273.15
    month_s = cal["year"] / len(cal["months"])
    P = np.load(c / f"monthly_{a.precip}.npy") * month_s  # kg m-2 = mm per month
    nm, nlat, nlon = T.shape
    codes = np.zeros((nlat, nlon), int)
    for j in range(nlat):
        for i in range(nlon):
            codes[j, i] = CODE[classify(T[:, j, i], P[:, j, i])]
    np.save(c / "koppen.npy", codes)
    w = np.asarray(meta["grid"]["weights"])[:, None] * np.ones((1, nlon))
    mask = np.ones_like(codes, bool) if a.mask is None else np.load(a.mask) > 0.5
    tot = (w * mask).sum()
    print("class  area fraction")
    for k, (name, _) in enumerate(CLASSES):
        f = (w * (codes == k) * mask).sum() / tot
        if f > 0:
            print(f"{name:<5}  {f:.3f}")
    lat = np.degrees(np.asarray(meta["grid"]["lat"]))
    lon = np.degrees(np.asarray(meta["grid"]["lon"]))
    cmap = ListedColormap([col for _, col in CLASSES])
    fig, ax = plt.subplots(figsize=(11, 5.2), constrained_layout=True)
    shown = np.where(mask, codes, np.nan)
    ax.pcolormesh(lon, lat, shown, cmap=cmap, vmin=-0.5, vmax=len(CLASSES) - 0.5, shading="nearest")
    present = sorted(set(codes[mask].ravel()))
    handles = [plt.Rectangle((0, 0), 1, 1, color=CLASSES[k][1]) for k in present]
    ax.legend(handles, [CLASSES[k][0] for k in present], loc="center left", bbox_to_anchor=(1.0, 0.5), fontsize=8)
    ax.set_xlabel("longitude [°E]")
    ax.set_ylabel("latitude [°]")
    ax.set_title(f"Köppen–Geiger classes: {run.name} (T{meta['truncation']})", fontsize=10)
    out = a.output or str(run / "koppen.png")
    fig.savefig(out, dpi=110)
    print(out)


if __name__ == "__main__":
    main()
