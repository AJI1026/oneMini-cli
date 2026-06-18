#!/usr/bin/env python3
"""生成 PPTX 缩略图预览（经 PDF 中转）。"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "shared" / "office"))

from soffice import convert  # noqa: E402


def thumbnails(pptx: str, out_dir: str, dpi: int = 120) -> None:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        pdf = convert(pptx, "pdf", tmp)
        prefix = out / "slide"
        if shutil.which("pdftoppm"):
            subprocess.run(
                ["pdftoppm", "-png", "-r", str(dpi), str(pdf), str(prefix)],
                check=True,
            )
        else:
            raise RuntimeError("需要 pdftoppm (poppler)")


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("pptx")
    p.add_argument("out_dir")
    p.add_argument("--dpi", type=int, default=120)
    args = p.parse_args()
    try:
        thumbnails(args.pptx, args.out_dir, args.dpi)
        print(f"OK → {args.out_dir}")
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
