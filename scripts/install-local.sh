#!/usr/bin/env bash
# OneMini-CLI local installer (manual tar.gz / DMG download — no signature verification)
# Usage:
#   ./install-local.sh
#   ./install-local.sh --skip-path

set -euo pipefail

BINARY_NAME="onemini"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKIP_PATH="${ONEMINI_SKIP_PATH:-0}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-path) SKIP_PATH=1; shift ;;
    -h|--help)
      echo "usage: $0 [--skip-path]"
      exit 0
      ;;
    *) echo "error: unknown argument: $1" >&2; exit 1 ;;
  esac
done

BIN="${SCRIPT_DIR}/${BINARY_NAME}"
if [[ ! -f "${BIN}" ]]; then
  echo "error: ${BINARY_NAME} not found in ${SCRIPT_DIR}" >&2
  exit 1
fi

chmod +x "${BIN}"
echo "==> local install from ${BIN}"
echo "warning: this script does not verify Ed25519 signatures; use install.sh for full verification"

if [[ "${SKIP_PATH}" == "1" ]]; then
  export ONEMINI_SKIP_PATH=1
fi

"${BIN}" install
