"""简化 DOCX 中同一作者的相邻 tracked changes。"""

from __future__ import annotations

from pathlib import Path
from xml.etree import ElementTree as ET

W_NS = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
NS = {"w": W_NS}


def _local(tag: str) -> str:
    return tag.split("}")[-1] if "}" in tag else tag


def simplify_redlines(doc_root: str) -> tuple[int, str]:
    root = Path(doc_root)
    doc_xml = root / "word" / "document.xml"
    if not doc_xml.is_file():
        return 0, "无 word/document.xml"

    tree = ET.parse(doc_xml)
    simplified = 0
    for parent in list(tree.iter()):
        children = list(parent)
        i = 0
        while i < len(children) - 1:
            a, b = children[i], children[i + 1]
            la, lb = _local(a.tag), _local(b.tag)
            if la in {"ins", "del"} and la == lb:
                author_a = a.get(f"{{{W_NS}}}author")
                author_b = b.get(f"{{{W_NS}}}author")
                if author_a and author_a == author_b:
                    for child in list(b):
                        a.append(child)
                    parent.remove(b)
                    simplified += 1
                    children = list(parent)
                    continue
            i += 1

    if simplified:
        tree.write(doc_xml, encoding="utf-8", xml_declaration=True)
    return simplified, "ok"
