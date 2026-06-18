#!/usr/bin/env python3
"""根据 JSON 填写 PDF 可交互表单。"""

from __future__ import annotations

import argparse
import json
import sys

from pypdf import PdfReader, PdfWriter

from extract_form_field_info import get_field_info


def fill_fields(pdf_in: str, fields_json: str, pdf_out: str) -> None:
    with open(fields_json, encoding="utf-8") as f:
        values = json.load(f)
    reader = PdfReader(pdf_in)
    known = {x["field_id"]: x for x in get_field_info(reader)}
    by_page: dict[int, dict[str, str]] = {}
    for item in values:
        fid = item["field_id"]
        if fid not in known:
            raise ValueError(f"未知字段: {fid}")
        meta = known[fid]
        page = item.get("page", meta["page"])
        if page != meta["page"]:
            raise ValueError(f"字段 {fid} 页码应为 {meta['page']}")
        if "value" in item:
            by_page.setdefault(page, {})[fid] = str(item["value"])

    writer = PdfWriter(clone_from=reader)
    for page_num, field_map in by_page.items():
        writer.update_page_form_field_values(
            writer.pages[page_num - 1], field_map, auto_regenerate=False
        )
    writer.set_need_appearances_writer(True)
    with open(pdf_out, "wb") as f:
        writer.write(f)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("pdf_in")
    p.add_argument("fields_json")
    p.add_argument("pdf_out")
    args = p.parse_args()
    try:
        fill_fields(args.pdf_in, args.fields_json, args.pdf_out)
        print(f"OK → {args.pdf_out}")
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
