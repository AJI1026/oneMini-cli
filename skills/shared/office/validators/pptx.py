from __future__ import annotations

from pathlib import Path

from .base import require_files


def validate_pptx_tree(root: Path) -> list[str]:
    errors = require_files(
        root,
        [
            "[Content_Types].xml",
            "ppt/presentation.xml",
            "_rels/.rels",
            "ppt/_rels/presentation.xml.rels",
        ],
    )
    pres = root / "ppt" / "presentation.xml"
    if pres.is_file():
        text = pres.read_text(encoding="utf-8", errors="replace")
        if "presentation" not in text:
            errors.append("ppt/presentation.xml 无效")
    return errors
