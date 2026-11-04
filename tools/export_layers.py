#!/usr/bin/env python3
"""Export map layers for other tools (map makers, game engines, GIS) at the heightmap
resolution, from the output of tools/downscale.py.

Usage:
    python tools/export_layers.py out/<run> [--outdir out/<run>/layers]

Writes equirectangular PNGs (row 0 = north, column 0 = 0°E, same size as the heightmap):
- `temp_annual.png`, `temp_warmest.png`, `temp_coldest.png`: 16-bit grayscale, linear
  in °C over the range given in `layers.json`;
- `precip_annual.png`: 16-bit grayscale, linear in mm/year over the range in layers.json;
- `koppen.png`: RGB with the usual Köppen colours (ocean transparent), `koppen_codes.png`:
  8-bit class index (255 = ocean);
- `layers.json`: value ranges, class names and colours.
"""
import argparse
import json
import pathlib
import sys
import zlib
import struct

import numpy as np

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import koppen  # noqa: E402


def write_png(path, arr, bitdepth, color):
    """Minimal PNG writer (no external dependencies). arr: uint8/uint16 (H, W) or (H, W, 4)."""
    h, w = arr.shape[:2]
    if color:
        ctype, data = 6, arr.astype(np.uint8)
    else:
        ctype = 0
        data = arr.astype(">u2") if bitdepth == 16 else arr.astype(np.uint8)
    raw = b"".join(b"\x00" + data[r].tobytes() for r in range(h))

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, bitdepth, ctype, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    pathlib.Path(path).write_bytes(png)


def scale16(x, lo, hi):
    return np.clip((x - lo) / (hi - lo), 0, 1) * 65535


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run")
    ap.add_argument("--outdir", default=None)
    a = ap.parse_args()
    run = pathlib.Path(a.run)
    d = run / "downscaled"
    t = np.load(d / "tas_monthly.npy").astype(float) - 273.15
    p = np.load(d / "precip_monthly.npy").astype(float)
    codes = np.load(d / "koppen.npy")
    out = pathlib.Path(a.outdir or run / "layers")
    out.mkdir(parents=True, exist_ok=True)
    meta = {"projection": "equirectangular, row 0 = north edge, column 0 = 0°E", "shape": list(codes.shape)}
    for name, field, lo, hi in [
        ("temp_annual", t.mean(0), -60.0, 50.0),
        ("temp_warmest", t.max(0), -60.0, 50.0),
        ("temp_coldest", t.min(0), -60.0, 50.0),
        ("precip_annual", p.sum(0), 0.0, 6000.0),
    ]:
        write_png(out / f"{name}.png", scale16(field, lo, hi), 16, False)
        unit = "degC" if name.startswith("temp") else "mm/year"
        meta[name] = {"min": lo, "max": hi, "unit": unit, "encoding": "uint16 linear"}
    rgba = np.zeros(codes.shape + (4,), np.uint8)
    for k, (_, col) in enumerate(koppen.CLASSES):
        rgb = [int(col[i:i + 2], 16) for i in (1, 3, 5)]
        rgba[codes == k] = rgb + [255]
    write_png(out / "koppen.png", rgba, 8, True)
    write_png(out / "koppen_codes.png", np.where(codes >= 0, codes, 255), 8, False)
    meta["koppen"] = [{"code": k, "name": n, "color": c} for k, (n, c) in enumerate(koppen.CLASSES)]
    (out / "layers.json").write_text(json.dumps(meta, indent=2))
    print(out)


if __name__ == "__main__":
    main()
