#!/usr/bin/env python3
"""校验 Office 文件结构（解包后 XML 或成品文件）。"""

from __future__ import annotations

import argparse
import sys
import tempfile
import zipfile
from pathlib import Path

from validators.docx import validate_docx_tree
from validators.pptx import validate_pptx_tree


def validate_path(target: Path) -> list[str]:
    errors: list[str] = []
    if target.is_file():
        suffix = target.suffix.lower()
        if suffix not in {".docx", ".pptx", ".xlsx", ".xlsm"}:
            return [f"不支持的扩展名: {suffix}"]
        try:
            with zipfile.ZipFile(target, "r") as zf:
                zf.testzip()
        except zipfile.BadZipFile:
            return ["ZIP 结构损坏"]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with zipfile.ZipFile(target, "r") as zf:
                zf.extractall(root)
            if suffix == ".docx":
                errors.extend(validate_docx_tree(root))
            elif suffix == ".pptx":
                errors.extend(validate_pptx_tree(root))
        return errors

    if target.is_dir():
        if (target / "word").is_dir():
            errors.extend(validate_docx_tree(target))
        elif (target / "ppt").is_dir():
            errors.extend(validate_pptx_tree(target))
        else:
            errors.append("无法识别解包目录类型")
    else:
        errors.append(f"路径不存在: {target}")
    return errors


def main() -> int:
    p = argparse.ArgumentParser(description="校验 Office 文件")
    p.add_argument("path")
    args = p.parse_args()
    errors = validate_path(Path(args.path))
    if errors:
        print("VALIDATION FAILED")
        for e in errors:
            print(f"  - {e}")
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
