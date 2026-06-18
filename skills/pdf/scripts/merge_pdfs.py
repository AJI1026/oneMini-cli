#!/usr/bin/env python3
"""合并多个 PDF。"""

from __future__ import annotations

import argparse
from pypdf import PdfReader, PdfWriter


def merge(inputs: list[str], output: str) -> None:
    writer = PdfWriter()
    for path in inputs:
        reader = PdfReader(path)
        for page in reader.pages:
            writer.add_page(page)
    with open(output, "wb") as f:
        writer.write(f)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("inputs", nargs="+")
    p.add_argument("-o", "--output", required=True)
    args = p.parse_args()
    merge(args.inputs, args.output)
    print(f"OK → {args.output} ({len(args.inputs)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
