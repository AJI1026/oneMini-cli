#!/usr/bin/env python3
"""用 LibreOffice 接受 DOCX 全部修订并输出干净副本。"""

from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
from pathlib import Path

# 允许从技能目录调用
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "shared" / "office"))

from soffice import convert, find_soffice  # noqa: E402


def accept_changes(input_docx: str, output_docx: str) -> None:
    if not find_soffice():
        raise RuntimeError("需要 LibreOffice 才能接受修订")
    src = Path(input_docx)
    out = Path(output_docx)
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp) / src.name
        shutil.copy2(src, work)
        # 转 ODT 再转 DOCX 可丢弃多数修订标记（LibreOffice 行为）
        odt = convert(str(work), "odt", tmp)
        convert(str(odt), "docx", str(out.parent))
        produced = out.parent / f"{odt.stem}.docx"
        if produced.is_file() and produced != out:
            produced.replace(out)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("output")
    args = p.parse_args()
    try:
        accept_changes(args.input, args.output)
        print(f"OK → {args.output}")
        return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
