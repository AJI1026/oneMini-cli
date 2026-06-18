"""合并 DOCX 中相邻、格式相同的 w:r run。"""

from __future__ import annotations

import re
from pathlib import Path
from xml.etree import ElementTree as ET

W_NS = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
NS = {"w": W_NS}


def _local(tag: str) -> str:
    return tag.split("}")[-1] if "}" in tag else tag


def run_signature(run: ET.Element) -> tuple:
    texts = []
    for node in run.iter():
        if _local(node.tag) == "t" and node.text:
            texts.append(node.text)
    rpr = run.find("w:rPr", NS)
    rpr_xml = ET.tostring(rpr, encoding="unicode") if rpr is not None else ""
    return (rpr_xml, "".join(texts))


def merge_runs(doc_root: str) -> tuple[int, str]:
    root = Path(doc_root)
    doc_xml = root / "word" / "document.xml"
    if not doc_xml.is_file():
        return 0, "无 word/document.xml"

    tree = ET.parse(doc_xml)
    merged = 0
    for parent in tree.iter():
        children = list(parent)
        if not children:
            continue
        i = 0
        while i < len(children) - 1:
            a, b = children[i], children[i + 1]
            if _local(a.tag) == "r" and _local(b.tag) == "r":
                if run_signature(a) == run_signature(b):
                    for node in b.iter():
                        if _local(node.tag) == "t" and node.text:
                            ta = a.find(".//w:t", NS)
                            if ta is not None:
                                ta.text = (ta.text or "") + node.text
                            merged += 1
                            break
                    parent.remove(b)
                    children = list(parent)
                    continue
            i += 1

    if merged:
        xml = ET.tostring(tree.getroot(), encoding="unicode")
        doc_xml.write_text(re.sub(r"ns\d:", "w:", xml), encoding="utf-8")
    return merged, "ok"
