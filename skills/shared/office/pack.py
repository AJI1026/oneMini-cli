#!/usr/bin/env python3
"""将解包目录重新打包为 Office 文件。"""

from __future__ import annotations

import argparse
import sys
import zipfile
from pathlib import Path


def pack(input_dir: str, output_file: str) -> str:
    src = Path(input_dir)
    out = Path(output_file)
    if not src.is_dir():
        raise NotADirectoryError(input_dir)

    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(src.rglob("*")):
            if path.is_file():
                zf.write(path, path.relative_to(src).as_posix())
    return f"已打包 → {out}"


def main() -> int:
    p = argparse.ArgumentParser(description="打包 Office 目录")
    p.add_argument("input_dir")
    p.add_argument("output_file")
    args = p.parse_args()
    try:
        print(pack(args.input_dir, args.output_file))
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
