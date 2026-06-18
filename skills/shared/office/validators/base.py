"""Office XML 校验基类。"""

from __future__ import annotations

from pathlib import Path


def require_files(root: Path, paths: list[str]) -> list[str]:
    errors = []
    for rel in paths:
        if not (root / rel).is_file():
            errors.append(f"缺少文件: {rel}")
    return errors
