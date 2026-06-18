#!/usr/bin/env bash
# 从 om-app-icon.svg（或已有 logo.png）生成安装包图标。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

WEB_LOGO_PNG="../oneMini-web/public/logo/logo.png"
WEB_APP_ICON="../oneMini-web/public/logo/om-app-icon.svg"
ASSETS="${ROOT}/assets"
LOGO="${ASSETS}/logo.png"

mkdir -p "${ASSETS}"

rasterize_app_icon() {
  local svg="$1"
  local png="$2"
  local size="${3:-512}"

  if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w "${size}" -h "${size}" "${svg}" -o "${png}"
    return 0
  fi

  if command -v magick >/dev/null 2>&1; then
    magick -background none -density 384 "${svg}" -resize "${size}x${size}" "${png}"
    return 0
  fi

  if command -v convert >/dev/null 2>&1; then
    convert -background none -density 384 "${svg}" -resize "${size}x${size}" "${png}"
    return 0
  fi

  if [[ "$(uname -s)" == "Darwin" ]] && command -v qlmanage >/dev/null 2>&1; then
    local tmp_dir
    tmp_dir="$(mktemp -d)"
    qlmanage -t -s "${size}" -o "${tmp_dir}" "${svg}" >/dev/null 2>&1
    local thumb="${tmp_dir}/$(basename "${svg}").png"
    if [[ -f "${thumb}" ]]; then
      mv "${thumb}" "${png}"
      rm -rf "${tmp_dir}"
      return 0
    fi
    rm -rf "${tmp_dir}"
  fi

  return 1
}

if [[ ! -f "${LOGO}" && -f "${ROOT}/${WEB_LOGO_PNG}" ]]; then
  cp "${ROOT}/${WEB_LOGO_PNG}" "${LOGO}"
  echo "==> copied logo.png from oneMini-web"
fi

if [[ ! -f "${LOGO}" && -f "${ROOT}/${WEB_APP_ICON}" ]]; then
  echo "==> rasterizing om-app-icon.svg -> assets/logo.png"
  if rasterize_app_icon "${ROOT}/${WEB_APP_ICON}" "${LOGO}" 512; then
    cp "${LOGO}" "${ROOT}/${WEB_LOGO_PNG}"
    echo "==> wrote ${ROOT}/${WEB_LOGO_PNG}"
  else
    echo "error: install rsvg-convert, ImageMagick, or run on macOS with qlmanage to rasterize om-app-icon.svg" >&2
    exit 1
  fi
fi

if [[ ! -f "${LOGO}" ]]; then
  echo "error: missing ${LOGO} (provide logo.png or om-app-icon.svg under oneMini-web/public/logo/)" >&2
  exit 1
fi

echo "==> generating onemini.ico"
cargo run --quiet --release --bin generate-icons

if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "==> generating AppIcon.icns"
  ICONSET="${ASSETS}/AppIcon.iconset"
  rm -rf "${ICONSET}"
  mkdir -p "${ICONSET}"
  for size in 16 32 128 256 512; do
    sips -z "${size}" "${size}" "${LOGO}" --out "${ICONSET}/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "${double}" "${double}" "${LOGO}" --out "${ICONSET}/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "${ICONSET}" -o "${ASSETS}/AppIcon.icns"
  rm -rf "${ICONSET}"
  echo "==> wrote ${ASSETS}/AppIcon.icns"
else
  echo "warning: AppIcon.icns requires macOS (iconutil); use committed file or run on macOS"
fi

echo "==> done"
