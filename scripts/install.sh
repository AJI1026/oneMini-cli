#!/usr/bin/env bash
# OneMini-CLI installer（HTTPS + Ed25519 签名校验）
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
#   ONEMINI_VERSION=0.1.0 curl -fsSL ... | bash

set -euo pipefail

REPO="AJI1026/OneMini-CLI"
BINARY_NAME="onemini"
GITHUB="https://github.com/${REPO}"
RAW_BASE="https://raw.githubusercontent.com/${REPO}/main"
VERSIONS_INDEX="${RAW_BASE}/release/versions.json"
VERSIONS_SIG="${RAW_BASE}/release/versions.json.sig"
PUBKEY_URL="${RAW_BASE}/release/signing_public_key.b64"
VERIFY_PY="${RAW_BASE}/scripts/verify_signature.py"

INSTALL_DIR="${ONEMINI_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${ONEMINI_VERSION:-}"
IGNORE_DEPRECATED="${ONEMINI_IGNORE_DEPRECATED:-0}"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "missing required command: $1"
}

ensure_https() {
  case "$1" in
    https://*) ;;
    *) error "refusing non-HTTPS URL: $1" ;;
  esac
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    *) error "unsupported OS: ${os} (macOS and Linux only; Windows: download from ${GITHUB}/releases)" ;;
  esac

  case "${arch}" in
    x86_64 | amd64)   arch="x86_64" ;;
    arm64 | aarch64)  arch="aarch64" ;;
    *) error "unsupported architecture: ${arch}" ;;
  esac

  printf '%s-%s' "${arch}" "${os}"
}

download() {
  local url="$1" dest="$2"
  ensure_https "${url}"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --proto '=https' --tlsv1.2 --retry 3 --retry-delay 1 -o "${dest}" "${url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${dest}" "${url}"
  else
    error "need curl or wget to download"
  fi
}

verify_blob() {
  local file="$1" sig_file="$2" pubkey_file="$3" verify_py="$4"
  python3 "${verify_py}" --file "${file}" --sig "${sig_file}" --pubkey "${pubkey_file}"
}

main() {
  need_cmd uname
  need_cmd tar
  need_cmd install
  need_cmd python3
  need_cmd openssl

  local target tmpdir index_json
  target="$(detect_target)"
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' EXIT

  info "detected platform: ${target}"
  info "fetching signed versions.json"

  download "${VERSIONS_INDEX}" "${tmpdir}/versions.json"
  download "${VERSIONS_SIG}" "${tmpdir}/versions.json.sig"
  download "${PUBKEY_URL}" "${tmpdir}/signing_public_key.b64"
  download "${VERIFY_PY}" "${tmpdir}/verify_signature.py"

  info "verifying versions.json signature"
  verify_blob "${tmpdir}/versions.json" "${tmpdir}/versions.json.sig" \
    "${tmpdir}/signing_public_key.b64" "${tmpdir}/verify_signature.py"

  index_json="${tmpdir}/versions.json"
  read -r archive_url sig_url sha256 deprecated reason <<<"$(python3 - "${index_json}" "${target}" "${VERSION}" <<'PY'
import json, sys
index_path, platform, req = sys.argv[1:4]
with open(index_path, encoding="utf-8") as f:
    index = json.load(f)
version = req.strip() if req.strip() else index["latest"]
entry = index["releases"][version]
url = entry["assets"][platform]["url"]
sig = entry["assets"][platform].get("sig_url") or (url + ".sig")
sha = entry["assets"][platform]["sha256"]
dep = "1" if entry.get("deprecated") else "0"
reason = entry.get("deprecation_reason") or "该版本存在已知安全问题"
print(url, sig, sha, dep, reason)
PY
)"

  if [[ "${deprecated}" == "1" && "${IGNORE_DEPRECATED}" != "1" ]]; then
    error "version is deprecated: ${reason}. Set ONEMINI_IGNORE_DEPRECATED=1 to continue"
  elif [[ "${deprecated}" == "1" ]]; then
    warn "installing deprecated version (${reason})"
  fi

  ensure_https "${archive_url}"
  ensure_https "${sig_url}"

  info "downloading ${archive_url}"
  download "${archive_url}" "${tmpdir}/archive"
  download "${sig_url}" "${tmpdir}/archive.sig"

  info "verifying release artifact signature"
  verify_blob "${tmpdir}/archive" "${tmpdir}/archive.sig" \
    "${tmpdir}/signing_public_key.b64" "${tmpdir}/verify_signature.py"

  local actual expected
  expected="${sha256}"
  if command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "${tmpdir}/archive" | awk '{print $1}')"
  else
    actual="$(sha256sum "${tmpdir}/archive" | awk '{print $1}')"
  fi
  [[ "${expected}" == "${actual}" ]] || error "SHA256 mismatch (expected ${expected}, got ${actual})"
  info "SHA256 verified"

  case "${archive_url}" in
    *.tar.gz)
      tar xzf "${tmpdir}/archive" -C "${tmpdir}"
      ;;
    *.zip)
      need_cmd unzip
      unzip -q "${tmpdir}/archive" -d "${tmpdir}"
      ;;
    *)
      error "unsupported archive format: ${archive_url}"
      ;;
  esac

  mkdir -p "${INSTALL_DIR}"
  install -m 755 "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  info "installed ${BINARY_NAME} -> ${INSTALL_DIR}/${BINARY_NAME}"

  if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    warn "${INSTALL_DIR} is not in PATH"
    echo "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
  fi

  if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
    info "run: ${BINARY_NAME} --help"
  fi
}

main "$@"
