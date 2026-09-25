#!/usr/bin/env python3
"""Zip the MV3 extension with manifest.json at the archive root.

Does not click Edge Allow and does not start a browser.
"""
from __future__ import annotations

import argparse
import zipfile
from pathlib import Path


def zip_extension(src: Path, dest: Path) -> int:
    src = src.resolve()
    manifest = src / "manifest.json"
    if not manifest.is_file():
        raise SystemExit(f"manifest.json missing: {src}")
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        dest.unlink()
    count = 0
    with zipfile.ZipFile(dest, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(src.rglob("*")):
            if not path.is_file():
                continue
            if "__pycache__" in path.parts or path.suffix == ".bak" or "tests" in path.parts:
                continue
            zf.write(path, path.relative_to(src).as_posix())
            count += 1
    with zipfile.ZipFile(dest) as zf:
        names = zf.namelist()
        if "manifest.json" not in names:
            raise SystemExit("zip root is missing manifest.json")
    return count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--dest", type=Path, required=True)
    args = parser.parse_args()
    count = zip_extension(args.source, args.dest)
    print(f"LENS_ZIP_OK {args.dest} files={count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
