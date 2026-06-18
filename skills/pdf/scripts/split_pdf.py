#!/usr/bin/env python3
"""拆分 PDF：按页输出或指定范围。"""

from __future__ import annotations

import argparse
from pathlib import Path

from pypdf import PdfReader, PdfWriter


def split(input_pdf: str, out_dir: str, pages: str | None = None) -> None:
    reader = PdfReader(input_pdf)
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    indices = range(len(reader.pages))
    if pages:
        indices = [int(x.strip()) - 1 for x in pages.split(",") if x.strip()]
    for i in indices:
        writer = PdfWriter()
        writer.add_page(reader.pages[i])
        target = out / f"page_{i + 1:03d}.pdf"
        with open(target, "wb") as f:
            writer.write(f)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("out_dir")
    p.add_argument("--pages", help="如 1,3,5")
    args = p.parse_args()
    split(args.input, args.out_dir, args.pages)
    print(f"OK → {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
