#!/usr/bin/env python3
"""提取 PDF 可填写表单字段信息，输出 JSON。"""

from __future__ import annotations

import argparse
import json
import sys

from pypdf import PdfReader


def get_field_info(reader: PdfReader) -> list[dict]:
    fields = reader.get_fields() or {}
    out = []
    for field_id, field in fields.items():
        ft = field.get("/FT")
        type_name = str(ft) if ft else "unknown"
        page = 1
        for i, pg in enumerate(reader.pages):
            annots = pg.get("/Annots") or []
            for annot_ref in annots:
                annot = annot_ref.get_object()
                if annot.get("/T") == field_id:
                    page = i + 1
        entry = {
            "field_id": field_id,
            "type": _normalize_type(type_name),
            "page": page,
        }
        if entry["type"] == "checkbox":
            states = field.get("/Opt") or ["/Yes", "/Off"]
            entry["checked_value"] = str(states[0]).lstrip("/")
            entry["unchecked_value"] = "Off"
        elif entry["type"] == "choice":
            opts = field.get("/Opt") or []
            entry["choice_options"] = [{"value": str(o).lstrip("/")} for o in opts]
        out.append(entry)
    return out


def _normalize_type(raw: str) -> str:
    raw = raw.lower()
    if "btn" in raw:
        return "checkbox"
    if "ch" in raw:
        return "choice"
    if "tx" in raw:
        return "text"
    return "unknown"


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("pdf")
    p.add_argument("-o", "--output")
    args = p.parse_args()
    reader = PdfReader(args.pdf)
    data = get_field_info(reader)
    text = json.dumps(data, ensure_ascii=False, indent=2)
    if args.output:
        open(args.output, "w", encoding="utf-8").write(text)
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
