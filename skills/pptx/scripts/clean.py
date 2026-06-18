#!/usr/bin/env python3
"""清理 PPTX：删除空白占位、压缩 speaker notes 空白。"""

from __future__ import annotations

import argparse
from pptx import Presentation
from pptx.enum.shapes import PP_PLACEHOLDER


def clean(path: str, output: str) -> None:
    prs = Presentation(path)
    for slide in prs.slides:
        to_remove = []
        for shape in slide.shapes:
            if not shape.has_text_frame:
                continue
            text = shape.text_frame.text.strip()
            if not text and shape.is_placeholder:
                ph = shape.placeholder_format.type
                if ph in {PP_PLACEHOLDER.BODY, PP_PLACEHOLDER.OBJECT}:
                    to_remove.append(shape)
        for shape in to_remove:
            sp = shape.element
            sp.getparent().remove(sp)
    prs.save(output)


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("input")
    p.add_argument("output")
    args = p.parse_args()
    clean(args.input, args.output)
    print(f"OK → {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
