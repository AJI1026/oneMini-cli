#!/usr/bin/env python3
"""向现有 PPTX 追加空白或复制版式的幻灯片。"""

from __future__ import annotations

import argparse
from copy import deepcopy

from pptx import Presentation


def add_slide(path: str, output: str, layout_index: int = 1, copies: int = 1) -> None:
    prs = Presentation(path)
    layout = prs.slide_layouts[min(layout_index, len(prs.slide_layouts) - 1)]
    for _ in range(copies):
        prs.slides.add_slide(layout)
    prs.save(output)


def duplicate_slide(path: str, output: str, slide_index: int) -> None:
    prs = Presentation(path)
    idx = slide_index - 1
    if idx < 0 or idx >= len(prs.slides):
        raise IndexError(f"幻灯片索引越界: {slide_index}")
    source = prs.slides[idx]
    blank = prs.slides.add_slide(source.slide_layout)
    for shp in source.shapes:
        new_el = deepcopy(shp.element)
        blank.shapes._spTree.insert_element_before(new_el, "p:extLst")
    prs.save(output)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("output")
    p.add_argument("--layout", type=int, default=1)
    p.add_argument("--count", type=int, default=1)
    p.add_argument("--duplicate", type=int, help="复制指定页（1-based）")
    args = p.parse_args()
    if args.duplicate:
        duplicate_slide(args.input, args.output, args.duplicate)
    else:
        add_slide(args.input, args.output, args.layout, args.count)
    print(f"OK → {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
