#!/usr/bin/env python3
"""提取 PDF 文本（pypdf / pdfplumber）。"""

from __future__ import annotations

import argparse
import sys


def extract(input_pdf: str, output: str | None, use_plumber: bool) -> str:
    chunks = []
    if use_plumber:
        import pdfplumber

        with pdfplumber.open(input_pdf) as pdf:
            for i, page in enumerate(pdf.pages, 1):
                text = page.extract_text() or ""
                chunks.append(f"--- Page {i} ---\n{text}")
    else:
        from pypdf import PdfReader

        reader = PdfReader(input_pdf)
        for i, page in enumerate(reader.pages, 1):
            text = page.extract_text() or ""
            chunks.append(f"--- Page {i} ---\n{text}")
    body = "\n\n".join(chunks)
    if output:
        open(output, "w", encoding="utf-8").write(body)
    return body


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("-o", "--output")
    p.add_argument("--layout", action="store_true", help="使用 pdfplumber 保留布局")
    args = p.parse_args()
    text = extract(args.input, args.output, args.layout)
    if not args.output:
        sys.stdout.write(text)
    else:
        print(f"OK → {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
