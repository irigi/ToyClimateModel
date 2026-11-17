#!/usr/bin/env python3
"""Earth reference data for the `earth_amip` scorecard and case (SPEC.md §9.5.3).

    .venv/bin/python -I tools/fetch_reference.py [--download-only] [--truncations 21 42]

Downloads public data with fixed URLs (no account) into `reference/raw/<source>/`, then
writes regridded arrays to `reference/derived/`:

- `t<N>/<field>.npy`: monthly (12, nlat, nlon) or static (nlat, nlon) float32 maps on the
  model's Gaussian grid for truncation N (rows north to south, column k at longitude
  2πk/nlon, the layout of the model's climatology output);
- `earth_heightmap.npy`: ETOPO5 elevation [m] as an equirectangular heightmap for the
  model's `geography` key (row 0 at 90°N, column 0 at 0°E; ≤ 0 is ocean);
- `SOURCES.md`: URL, download date, citation and licence of every source.

Regridding: the source is interpolated bilinearly (periodic in longitude) onto a 0.25°
grid, then box-averaged onto the Gaussian cells (cell edges from the Gaussian weights,
area-weighted), the same binning as the model's heightmap reader. Missing values (land in
the SST and sea-ice data) are first filled by repeated neighbour averaging, so that cells
with some land carry the SST of the nearby ocean. The Köppen map is categorical: each
Gaussian cell takes the majority class of its land pixels.

The script sends nothing but the HTTP requests for these files.
"""
import argparse
import datetime
import pathlib
import shutil
import urllib.request
import zipfile

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
RAW = ROOT / "reference" / "raw"
DERIVED = ROOT / "reference" / "derived"
PSL = "https://downloads.psl.noaa.gov/Datasets"

# name -> (url, file name, citation, licence)
SOURCES = {
    "gpcp": (
        f"{PSL}/gpcp/precip.mon.ltm.1991-2020.nc", "precip.mon.ltm.1991-2020.nc",
        "Adler, R.F. et al. (2018): The Global Precipitation Climatology Project (GPCP) "
        "monthly analysis (new version 2.3) and a review of 2017 global precipitation. "
        "Atmosphere 9, 138. Long-term monthly means 1991-2020 from NOAA PSL.",
        "public domain (NOAA PSL)"),
    "ncep_air2m": (
        f"{PSL}/ncep.reanalysis.derived/surface_gauss/air.2m.mon.ltm.1991-2020.nc",
        "air.2m.mon.ltm.1991-2020.nc",
        "Kalnay, E. et al. (1996): The NCEP/NCAR 40-year reanalysis project. Bull. Amer. "
        "Meteor. Soc. 77, 437-471. 2 m air temperature, long-term monthly means 1991-2020.",
        "public domain (NOAA PSL)"),
    "ncep_hgt_gauss": (
        f"{PSL}/ncep.reanalysis.derived/surface_gauss/hgt.sfc.gauss.nc", "hgt.sfc.gauss.nc",
        "Kalnay et al. (1996), NCEP/NCAR reanalysis model orography on its T62 Gaussian "
        "grid (the orography of the 2 m temperature).",
        "public domain (NOAA PSL)"),
    "ncep_slp": (
        f"{PSL}/ncep.reanalysis.derived/surface/slp.mon.ltm.1991-2020.nc",
        "slp.mon.ltm.1991-2020.nc",
        "Kalnay et al. (1996), NCEP/NCAR reanalysis sea-level pressure, long-term monthly "
        "means 1991-2020.",
        "public domain (NOAA PSL)"),
    "ncep_uwnd": (
        f"{PSL}/ncep.reanalysis.derived/pressure/uwnd.mon.ltm.1991-2020.nc",
        "uwnd.mon.ltm.1991-2020.nc",
        "Kalnay et al. (1996), NCEP/NCAR reanalysis zonal wind on pressure levels, "
        "long-term monthly means 1991-2020.",
        "public domain (NOAA PSL)"),
    "oisst_sst": (
        f"{PSL}/noaa.oisst.v2/sst.ltm.1991-2020.nc", "sst.ltm.1991-2020.nc",
        "Reynolds, R.W. et al. (2002): An improved in situ and satellite SST analysis for "
        "climate. J. Climate 15, 1609-1625. NOAA OI SST V2, long-term monthly means "
        "1991-2020.",
        "public domain (NOAA PSL)"),
    "oisst_icec": (
        f"{PSL}/noaa.oisst.v2/icec.ltm.1991-2020.nc", "icec.ltm.1991-2020.nc",
        "Reynolds et al. (2002), NOAA OI SST V2 sea-ice concentration, long-term monthly "
        "means 1991-2020.",
        "public domain (NOAA PSL)"),
    "oisst_lsmask": (
        f"{PSL}/noaa.oisst.v2/lsmask.nc", "lsmask.nc",
        "Reynolds et al. (2002), NOAA OI SST V2 land-sea mask.",
        "public domain (NOAA PSL)"),
    "snowcover": (
        f"{PSL}/snowcover/snowcover.mon.ltm.nc", "snowcover.mon.ltm.nc",
        "Robinson, D.A., Estilow, T.W. and NOAA CDR Program: NOAA Climate Data Record of "
        "Northern Hemisphere snow cover extent (Rutgers), long-term monthly means as "
        "distributed by NOAA PSL.",
        "public domain (NOAA PSL)"),
    "etopo5": (
        "https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO5/TOPO/ETOPO5/ETOPO5.DAT",
        "ETOPO5.DAT",
        "NOAA National Geophysical Data Center (1988): Data Announcement 88-MGG-02, "
        "Digital relief of the surface of the Earth (ETOPO5), 5 arc-minute grid.",
        "public domain (NOAA NCEI)"),
    "beck_kg": (
        "https://ndownloader.figshare.com/files/12407516", "Beck_KG_V1.zip",
        "Beck, H.E., Zimmermann, N.E., McVicar, T.R., Vergopolan, N., Berg, A. and Wood, "
        "E.F. (2018): Present and future Köppen-Geiger climate classification maps at 1-km "
        "resolution. Sci. Data 5, 180214. doi:10.6084/m9.figshare.6396959 (present "
        "1980-2016, 0.5° map).",
        "CC BY 4.0"),
}


def download(names, force=False):
    for name in names:
        url, fname, _, _ = SOURCES[name]
        d = RAW / name
        d.mkdir(parents=True, exist_ok=True)
        f = d / fname
        if f.exists() and not force:
            continue
        print(f"download {url}")
        tmp = f.with_suffix(f.suffix + ".part")
        with urllib.request.urlopen(url, timeout=120) as r, open(tmp, "wb") as out:
            shutil.copyfileobj(r, out)
        tmp.rename(f)
        (d / "DOWNLOADED").write_text(datetime.date.today().isoformat() + "\n")


def raw(name):
    return RAW / name / SOURCES[name][1]


# ---------------------------------------------------------------------------------------
# grids and regridding


def gaussian_grid(trunc):
    """(lat [rad, north to south], lon [rad], mu edges north to south) of the model grid."""
    nlat = {21: 32, 42: 64, 63: 96, 85: 128}[trunc]
    nlon = 2 * nlat
    x, w = np.polynomial.legendre.leggauss(nlat)
    mu = x[::-1]
    w = w[::-1]
    edges = np.concatenate([[1.0], 1.0 - np.cumsum(w)])
    edges[-1] = -1.0
    return np.arcsin(mu), 2 * np.pi * np.arange(nlon) / nlon, edges


FINE = 0.25  # degrees


def fine_grid():
    lat = 90 - FINE * (np.arange(int(180 / FINE)) + 0.5)
    lon = FINE * (np.arange(int(360 / FINE)) + 0.5)
    return lat, lon


def to_fine(field, lat, lon):
    """Bilinear interpolation of field[..., lat, lon] (lon in degrees, any start, regular)
    onto the 0.25° grid; latitudes outside the source range take the edge row."""
    lat = np.asarray(lat, float)
    lon = np.mod(np.asarray(lon, float), 360.0)
    order = np.argsort(lon)
    lon = lon[order]
    field = field[..., order]
    if lat[0] > lat[-1]:
        lat = lat[::-1]
        field = field[..., ::-1, :]
    flat, flon = fine_grid()
    # latitude weights
    jy = np.clip(np.searchsorted(lat, flat) - 1, 0, len(lat) - 2)
    ty = np.clip((flat - lat[jy]) / (lat[jy + 1] - lat[jy]), 0.0, 1.0)
    # periodic longitude
    lonx = np.concatenate([lon, [lon[0] + 360.0]])
    fx = np.concatenate([field, field[..., :1]], axis=-1)
    flon_w = np.where(flon < lonx[0], flon + 360.0, flon)
    ix = np.clip(np.searchsorted(lonx, flon_w) - 1, 0, len(lonx) - 2)
    tx = (flon_w - lonx[ix]) / (lonx[ix + 1] - lonx[ix])
    a = fx[..., jy, :]
    rows = a[..., :, ix] * (1 - tx) + a[..., :, ix + 1] * tx
    b = fx[..., jy + 1, :]
    rows1 = b[..., :, ix] * (1 - tx) + b[..., :, ix + 1] * tx
    return rows * (1 - ty)[:, None] + rows1 * ty[:, None]


def bin_index(src_lat_deg, src_lon_deg, trunc):
    """Gaussian cell (j, k) of every pixel of an equirectangular grid, and its area weight."""
    _, glon, edges = gaussian_grid(trunc)
    nlat, nlon = len(edges) - 1, len(glon)
    mu = np.sin(np.radians(src_lat_deg))
    j = np.clip(np.searchsorted(-edges, -mu, side="right") - 1, 0, nlat - 1)
    dl = 2 * np.pi / nlon
    k = (np.floor((np.radians(src_lon_deg) + 0.5 * dl) / dl).astype(int)) % nlon
    w = np.cos(np.radians(src_lat_deg))
    return j, k, w, nlat, nlon


def box_average(fine, trunc, lat=None, lon=None, mask=None):
    """Area-weighted mean of fine[..., lat, lon] over each Gaussian cell. `mask` (lat, lon)
    selects contributing pixels; cells with none get NaN."""
    if lat is None:
        lat, lon = fine_grid()
    j, k, w, nlat, nlon = bin_index(lat, lon, trunc)
    cell = (j[:, None] * nlon + k[None, :]).ravel()
    ww = np.broadcast_to(w[:, None], (len(lat), len(lon))).ravel()
    if mask is not None:
        ww = ww * mask.ravel()
    wsum = np.bincount(cell, ww, nlat * nlon)
    lead = fine.shape[:-2]
    flat = fine.reshape(-1, len(lat) * len(lon))
    out = np.array([np.bincount(cell, ww * f, nlat * nlon) for f in flat])
    with np.errstate(invalid="ignore", divide="ignore"):
        out = out / wsum
    return out.reshape(lead + (nlat, nlon))


def majority(classes, lat, lon, trunc, nclass):
    """Majority class (ignoring 0 = no data) of the pixels in each Gaussian cell; 0 where
    a cell has no classified pixel. Also returns the classified-area fraction."""
    j, k, w, nlat, nlon = bin_index(lat, lon, trunc)
    cell = (j[:, None] * nlon + k[None, :]).ravel()
    ww = np.broadcast_to(w[:, None], classes.shape).ravel()
    c = classes.ravel().astype(int)
    hist = np.zeros((nlat * nlon, nclass + 1))
    np.add.at(hist, (cell, c), ww)
    total = hist.sum(axis=1)
    best = hist[:, 1:].argmax(axis=1) + 1
    best[hist[:, 1:].sum(axis=1) == 0] = 0
    with np.errstate(invalid="ignore", divide="ignore"):
        frac = hist[:, 1:].sum(axis=1) / total
    return best.reshape(nlat, nlon), frac.reshape(nlat, nlon)


def fill_missing(field, iters=2000):
    """Fill NaNs of field[..., lat, lon] by repeated averaging of the 4 neighbours (periodic
    in longitude); known values are kept."""
    f = field.copy()
    miss = np.isnan(f)
    if not miss.any():
        return f
    f[miss] = np.nanmean(field)
    for _ in range(iters):
        up = np.concatenate([f[..., :1, :], f[..., :-1, :]], axis=-2)
        dn = np.concatenate([f[..., 1:, :], f[..., -1:, :]], axis=-2)
        avg = 0.25 * (up + dn + np.roll(f, 1, -1) + np.roll(f, -1, -1))
        f[miss] = avg[miss]
    return f


# ---------------------------------------------------------------------------------------
# readers


def nc(name, var):
    import netCDF4

    with netCDF4.Dataset(raw(name)) as d:
        v = d.variables[var]
        v.set_auto_maskandscale(True)
        data = np.ma.filled(v[:].astype(np.float64), np.nan)
        lat = np.asarray(d.variables["lat"][:], float)
        lon = np.asarray(d.variables["lon"][:], float)
        extra = {}
        if "level" in d.variables:
            extra["level"] = np.asarray(d.variables["level"][:], float)
        units = getattr(v, "units", "")
    return data, lat, lon, extra, units


def etopo5():
    """ETOPO5 as (2160, 4320) float32, row 0 at 90°N, column 0 at 0°E (big-endian int16)."""
    h = np.fromfile(raw("etopo5"), dtype=">i2").reshape(2160, 4320).astype(np.float32)
    lat = 90 - (np.arange(2160) + 0.5) / 12.0
    lon = (np.arange(4320) + 0.5) / 12.0
    return h, lat, lon


def beck_kg():
    """Beck et al. (2018) present-day 0.5° map: (classes 0..30, lat, lon), 0 = sea."""
    from PIL import Image

    z = zipfile.ZipFile(raw("beck_kg"))
    name = [n for n in z.namelist() if n.endswith("present_0p5.tif")][0]
    d = RAW / "beck_kg" / "extract"
    d.mkdir(exist_ok=True)
    tif = d / pathlib.Path(name).name
    if not tif.exists():
        tif.write_bytes(z.read(name))
    a = np.array(Image.open(tif))
    nlat, nlon = a.shape
    lat = 90 - (np.arange(nlat) + 0.5) * 180.0 / nlat
    lon = -180 + (np.arange(nlon) + 0.5) * 360.0 / nlon
    # roll so that column 0 starts at 0°E
    shift = nlon // 2
    return np.roll(a, -shift, axis=1), lat, np.mod(np.roll(lon, -shift), 360.0)


# ---------------------------------------------------------------------------------------
# derived fields


def derive(truncs):
    DERIVED.mkdir(parents=True, exist_ok=True)
    h, hlat, hlon = etopo5()
    np.save(DERIVED / "earth_heightmap.npy", h)
    print(f"earth_heightmap.npy {h.shape}, land fraction "
          f"{np.average((h > 0).mean(axis=1), weights=np.cos(np.radians(hlat))):.3f}")

    kg, klat, klon = beck_kg()

    sea = nc("oisst_lsmask", "mask")[0][0] > 0.5  # OISST: 1 = ocean

    def fine(name, var, scale=1.0, offset=0.0, level=None, fill=False, ocean_only=False):
        data, lat, lon, extra, _ = nc(name, var)
        if level is not None:
            data = data[:, list(extra["level"]).index(level)]
        data = data * scale + offset
        if ocean_only:
            data = np.where(sea, data, np.nan)
        if fill:
            data = fill_missing(data)
        return to_fine(data, lat, lon)

    # Rutgers snow cover covers 0.5-89.5°N only: extend with NaN rows (no data) to 89.5°S
    # so the southern hemisphere is missing rather than copied from the edge row.
    snc, slat, slon, _, _ = nc("snowcover", "snowcover")
    if slat[0] < slat[-1]:
        snc, slat = snc[:, ::-1], slat[::-1]
    sh = -(np.arange(len(slat)) + 0.5)
    snc = np.concatenate([snc, np.full_like(snc, np.nan)], axis=1) / 100.0
    slat = np.concatenate([slat, sh])

    fields = {
        "pr": fine("gpcp", "precip"),  # mm/day
        "tas": fine("ncep_air2m", "air"),  # K
        "psl": fine("ncep_slp", "slp", scale=100.0),  # hPa -> Pa
        "ua200": fine("ncep_uwnd", "uwnd", level=200.0),  # m/s
        "tos": fine("oisst_sst", "sst", offset=273.15, fill=True, ocean_only=True),  # K
        # % -> fraction; land is missing in the source and filled from the nearby ocean
        "sic": np.clip(fine("oisst_icec", "icec", scale=0.01, fill=True, ocean_only=True),
                       0.0, 1.0),
        "snc": to_fine(snc, slat, slon),  # fraction, NaN south of ~0.5°N
    }
    hgt, glat, glon, _, _ = nc("ncep_hgt_gauss", "hgt")
    fields["orog_tas_ref"] = to_fine(hgt[0] if hgt.ndim == 3 else hgt, glat, glon)

    for t in truncs:
        d = DERIVED / f"t{t}"
        d.mkdir(exist_ok=True)
        for k, v in fields.items():
            g = box_average(v, t).astype(np.float32)
            np.save(d / f"{k}.npy", g)
        # topography and land fraction from ETOPO5 (as the model computes them)
        elev = box_average(np.maximum(h, 0.0), t, hlat, hlon)
        land = box_average((h > 0).astype(np.float64), t, hlat, hlon)
        np.save(d / "orog.npy", elev.astype(np.float32))
        np.save(d / "land.npy", land.astype(np.float32))
        cls, frac = majority(kg, klat, klon, t, 30)
        np.save(d / "koppen_beck.npy", cls.astype(np.int8))
        np.save(d / "koppen_beck_landfrac.npy", frac.astype(np.float32))
        print(f"t{t}: {', '.join(sorted(fields))}, orog, land, koppen_beck")
    write_sources()


def write_sources():
    lines = ["# Reference data sources (written by tools/fetch_reference.py)", "",
             "| source | URL | downloaded | citation | licence |", "|---|---|---|---|---|"]
    for name, (url, _, cit, lic) in SOURCES.items():
        stamp = RAW / name / "DOWNLOADED"
        date = stamp.read_text().strip() if stamp.exists() else "?"
        lines.append(f"| {name} | {url} | {date} | {cit} | {lic} |")
    (DERIVED / "SOURCES.md").write_text("\n".join(lines) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--download-only", action="store_true")
    ap.add_argument("--force", action="store_true", help="download again")
    ap.add_argument("--truncations", type=int, nargs="+", default=[21, 42])
    a = ap.parse_args()
    download(SOURCES, a.force)
    if not a.download_only:
        derive(a.truncations)


if __name__ == "__main__":
    main()
