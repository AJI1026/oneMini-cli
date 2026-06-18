#!/usr/bin/env python3
"""解包 Office 文件 (docx/pptx/xlsx) 为目录，便于编辑 XML。"""

from __future__ import annotations

import argparse
import sys
import zipfile
from pathlib import Path
from xml.dom import minidom

from helpers.merge_runs import merge_runs
from helpers.simplify_redlines import simplify_redlines

OFFICE_SUFFIXES = {".docx", ".pptx", ".xlsx", ".xlsm"}


def pretty_xml(path: Path) -> None:
    try:
        raw = path.read_text(encoding="utf-8")
        dom = minidom.parseString(raw.encode("utf-8"))
        path.write_text(dom.toprettyxml(indent="  ", encoding=None), encoding="utf-8")
    except Exception:
        pass


def unpack(
    input_file: str,
    output_dir: str,
    *,
    merge_runs_flag: bool = True,
    simplify_redlines_flag: bool = True,
) -> str:
    src = Path(input_file)
    out = Path(output_dir)
    if not src.is_file():
        raise FileNotFoundError(input_file)
    if src.suffix.lower() not in OFFICE_SUFFIXES:
        raise ValueError(f"不支持的格式: {src.suffix}")

    out.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(src, "r") as zf:
        zf.extractall(out)

    xml_files = list(out.rglob("*.xml")) + list(out.rglob("*.rels"))
    for xf in xml_files:
        pretty_xml(xf)

    msg = f"已解包 {src.name}（{len(xml_files)} 个 XML）"

    if src.suffix.lower() == ".docx":
        if simplify_redlines_flag:
            n, _ = simplify_redlines(str(out))
            msg += f"，简化修订 {n} 处"
        if merge_runs_flag:
            n, _ = merge_runs(str(out))
            msg += f"，合并 run {n} 处"

    return msg


def main() -> int:
    p = argparse.ArgumentParser(description="解包 Office 文件")
    p.add_argument("input")
    p.add_argument("output_dir")
    p.add_argument("--merge-runs", choices=("true", "false"), default="true")
    p.add_argument("--simplify-redlines", choices=("true", "false"), default="true")
    args = p.parse_args()
    try:
        print(
            unpack(
                args.input,
                args.output_dir,
                merge_runs_flag=args.merge_runs == "true",
                simplify_redlines_flag=args.simplify_redlines == "true",
            )
        )
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
