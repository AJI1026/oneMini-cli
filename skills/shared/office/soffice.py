"""LibreOffice headless 辅助（转换、宏环境）。"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path


def find_soffice() -> str | None:
    for name in ("soffice", "libreoffice"):
        path = shutil.which(name)
        if path:
            return path
    if platform.system() == "Darwin":
        app = Path("/Applications/LibreOffice.app/Contents/MacOS/soffice")
        if app.is_file():
            return str(app)
    return None


def get_soffice_env() -> dict[str, str]:
    """沙箱/CI 下限制 Unix socket 时使用独立 UserInstallation。"""
    env = os.environ.copy()
    if "SAL_USE_VCLPLUGIN" not in env:
        env["SAL_USE_VCLPLUGIN"] = "gen"
    if platform.system() == "Linux" and not os.environ.get("USERINSTALLATION"):
        tmp = tempfile.mkdtemp(prefix="onemini-lo-")
        env["UserInstallation"] = Path(tmp).as_uri()
    return env


def convert(input_path: str, output_format: str, out_dir: str | None = None) -> Path:
    soffice = find_soffice()
    if not soffice:
        raise RuntimeError("未找到 LibreOffice (soffice)。请安装 LibreOffice。")
    src = Path(input_path).resolve()
    if not src.is_file():
        raise FileNotFoundError(input_path)
    dest_dir = Path(out_dir or src.parent)
    dest_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        soffice,
        "--headless",
        "--norestore",
        "--convert-to",
        output_format,
        "--outdir",
        str(dest_dir),
        str(src),
    ]
    result = subprocess.run(
        cmd, capture_output=True, text=True, env=get_soffice_env(), check=False
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr or result.stdout or "LibreOffice 转换失败")
    ext = output_format.split(":")[0]
    out = dest_dir / f"{src.stem}.{ext}"
    if not out.is_file():
        matches = list(dest_dir.glob(f"{src.stem}.*"))
        if len(matches) == 1:
            return matches[0]
        raise RuntimeError(f"未找到转换输出: {out}")
    return out


if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser(description="LibreOffice 格式转换")
    p.add_argument("input")
    p.add_argument("--to", required=True, help="如 pdf, docx, png")
    p.add_argument("--outdir")
    args = p.parse_args()
    out = convert(args.input, args.to, args.outdir)
    print(out)
