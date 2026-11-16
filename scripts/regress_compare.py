#!/usr/bin/env python3
"""Compare two toyclim run directories for `scripts/regress --against` (SPEC.md §9.5.4).

Every file must exist on both sides and be byte-identical, except JSON sidecars
(`meta.json`, `restart.json`), which are compared after dropping the keys that legitimately
differ between builds and schema versions: the git hash, the version string and the config
echo. Scalars, grids, σ levels, planet constants and calendars in them must still agree
exactly.

Usage: regress_compare.py <ref_run_dir> <new_run_dir>     (exit 0 = identical)
"""
import json
import pathlib
import sys

VOLATILE = {"git_hash", "toyclim_version", "config"}


def files(root):
    return sorted(p.relative_to(root) for p in root.rglob("*") if p.is_file())


def strip(v):
    if isinstance(v, dict):
        return {k: strip(x) for k, x in v.items() if k not in VOLATILE}
    return v


def main():
    ref, new = (pathlib.Path(a) for a in sys.argv[1:3])
    fr, fn = files(ref), files(new)
    bad = []
    for p in sorted(set(fr) ^ set(fn)):
        bad.append(f"only in {'ref' if p in fr else 'new'}: {p}")
    for p in sorted(set(fr) & set(fn)):
        a, b = (ref / p).read_bytes(), (new / p).read_bytes()
        if p.suffix == ".json":
            if strip(json.loads(a)) != strip(json.loads(b)):
                bad.append(f"json differs: {p}")
        elif a != b:
            bad.append(f"bytes differ: {p}")
    n = len(set(fr) & set(fn))
    if bad:
        print(f"DIFFER ({len(bad)} of {n} files):")
        for line in bad[:20]:
            print("  " + line)
        if len(bad) > 20:
            print(f"  ... {len(bad) - 20} more")
        sys.exit(1)
    print(f"identical ({n} files)")


if __name__ == "__main__":
    main()
