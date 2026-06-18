#!/usr/bin/env bash
# 从 assets/logo.png（与 oneMini-web/public/logo/logo.png 同源）生成安装包图标。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT}"

WEB_LOGO="../oneMini-web/public/logo/logo.png"
ASSETS="${ROOT}/assets"
LOGO="${ASSETS}/logo.png"

mkdir -p "${ASSETS}"

if [[ ! -f "${LOGO}" && -f "${ROOT}/${WEB_LOGO}" ]]; then
  cp "${ROOT}/${WEB_LOGO}" "${LOGO}"
  echo "==> copied logo from oneMini-web"
fi

if [[ ! -f "${LOGO}" ]]; then
  echo "error: missing ${LOGO} (copy from oneMini-web/public/logo/logo.png)" >&2
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
