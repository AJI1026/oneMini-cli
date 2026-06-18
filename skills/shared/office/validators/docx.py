from __future__ import annotations

from pathlib import Path

from .base import require_files


def validate_docx_tree(root: Path) -> list[str]:
    errors = require_files(
        root,
        [
            "[Content_Types].xml",
            "word/document.xml",
            "_rels/.rels",
            "word/_rels/document.xml.rels",
        ],
    )
    doc = root / "word" / "document.xml"
    if doc.is_file():
        text = doc.read_text(encoding="utf-8", errors="replace")
        if "<w:document" not in text and "wordprocessingml" not in text:
            errors.append("word/document.xml 不像有效 DOCX 正文")
    return errors
