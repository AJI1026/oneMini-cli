#!/usr/bin/env python3
"""PDF 转 PNG/JPEG（pdftoppm 或 pdf2image）。"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


def convert(input_pdf: str, out_dir: str, dpi: int, fmt: str) -> None:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    prefix = out / "page"
    if shutil.which("pdftoppm"):
        flag = "-png" if fmt == "png" else "-jpeg"
        subprocess.run(
            ["pdftoppm", flag, "-r", str(dpi), input_pdf, str(prefix)],
            check=True,
        )
        return
    try:
        from pdf2image import convert_from_path
    except ImportError as e:
        raise RuntimeError("需要 poppler (pdftoppm) 或 pip install pdf2image") from e
    images = convert_from_path(input_pdf, dpi=dpi)
    for i, img in enumerate(images, 1):
        img.save(out / f"page_{i:03d}.{fmt}")


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("out_dir")
    p.add_argument("--dpi", type=int, default=150)
    p.add_argument("--format", choices=("png", "jpg"), default="png")
    args = p.parse_args()
    try:
        convert(args.input, args.out_dir, args.dpi, args.format)
        print(f"OK → {args.out_dir}")
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
