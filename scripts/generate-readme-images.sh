#!/usr/bin/env bash
# 从 docs/assets/*.svg 生成 README 用 PNG（GitHub README 不支持相对路径 SVG）。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ASSETS="${ROOT}/docs/assets"

rasterize() {
  local svg="$1"
  local png="$2"
  local width="$3"

  if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w "${width}" "${ASSETS}/${svg}" -o "${ASSETS}/${png}"
    return 0
  fi

  if command -v magick >/dev/null 2>&1; then
    magick -background none -density 384 "${ASSETS}/${svg}" -resize "${width}x" "${ASSETS}/${png}"
    return 0
  fi

  if command -v npx >/dev/null 2>&1; then
    npx --yes @resvg/resvg-js-cli --fit-width "${width}" "${ASSETS}/${svg}" "${ASSETS}/${png}" >/dev/null
    return 0
  fi

  echo "error: install rsvg-convert, ImageMagick, or Node.js (npx) to rasterize README images" >&2
  return 1
}

echo "==> docs/assets/hero.svg -> hero.png"
rasterize hero.svg hero.png 1280
echo "==> docs/assets/session.svg -> session.png"
rasterize session.svg session.png 1400
echo "==> docs/assets/themes.svg -> themes.png"
rasterize themes.svg themes.png 1400
echo "==> done"
