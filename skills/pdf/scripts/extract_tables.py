#!/usr/bin/env python3
"""提取 PDF 表格为 CSV / JSON。"""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

import pdfplumber


def extract_tables(input_pdf: str, out_dir: str, fmt: str) -> None:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    all_rows = []
    with pdfplumber.open(input_pdf) as pdf:
        for pi, page in enumerate(pdf.pages, 1):
            for ti, table in enumerate(page.extract_tables() or [], 1):
                if not table:
                    continue
                name = f"p{pi}_t{ti}"
                if fmt == "csv":
                    path = out / f"{name}.csv"
                    with open(path, "w", newline="", encoding="utf-8") as f:
                        csv.writer(f).writerows(table)
                all_rows.append({"page": pi, "table": ti, "rows": table})
    if fmt == "json":
        (out / "tables.json").write_text(
            json.dumps(all_rows, ensure_ascii=False, indent=2), encoding="utf-8"
        )


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("out_dir")
    p.add_argument("--format", choices=("csv", "json"), default="csv")
    args = p.parse_args()
    extract_tables(args.input, args.out_dir, args.format)
    print(f"OK → {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
