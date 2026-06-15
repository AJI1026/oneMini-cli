#!/usr/bin/env bash
# OneMini-CLI installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
#   ONEMINI_VERSION=v0.1.0 curl -fsSL ... | bash

set -euo pipefail

REPO="AJI1026/OneMini-CLI"
BINARY_NAME="onemini"
GITHUB="https://github.com/${REPO}"

# Install directory (default: ~/.local/bin)
INSTALL_DIR="${ONEMINI_INSTALL_DIR:-${HOME}/.local/bin}"

# Version tag, e.g. v0.1.0; empty means latest release
VERSION="${ONEMINI_VERSION:-}"

# Set to 1 to verify SHA256 checksum after download
VERIFY="${ONEMINI_VERIFY:-0}"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "missing required command: $1"
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    *) error "unsupported OS: ${os} (macOS and Linux only; Windows users: download from ${GITHUB}/releases)" ;;
  esac

  case "${arch}" in
    x86_64 | amd64)   arch="x86_64" ;;
    arm64 | aarch64)  arch="aarch64" ;;
    *) error "unsupported architecture: ${arch}" ;;
  esac

  printf '%s-%s' "${arch}" "${os}"
}

resolve_version_path() {
  if [[ -n "${VERSION}" ]]; then
    # Accept "0.1.0" or "v0.1.0"
    [[ "${VERSION}" == v* ]] || VERSION="v${VERSION}"
    printf '%s' "${VERSION}"
    return
  fi

  if command -v curl >/dev/null 2>&1; then
    local tag
    tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n1)"
    [[ -n "${tag}" ]] || error "could not resolve latest release version"
    VERSION="${tag}"
    printf '%s' "${tag}"
    return
  fi

  warn "could not query GitHub API; falling back to /releases/latest redirect"
  printf 'latest'
}

download() {
  local url="$1" dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 3 --retry-delay 1 -o "${dest}" "${url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${dest}" "${url}"
  else
    error "need curl or wget to download"
  fi
}

verify_checksum() {
  local archive="$1" checksum_file="$2"
  need_cmd shasum
  local expected actual
  expected="$(awk '{print $1}' "${checksum_file}")"
  actual="$(shasum -a 256 "${archive}" | awk '{print $1}')"
  [[ "${expected}" == "${actual}" ]] || error "checksum mismatch (expected ${expected}, got ${actual})"
  info "checksum verified"
}

main() {
  need_cmd uname
  need_cmd tar
  need_cmd install

  local target archive version_tag tmpdir
  target="$(detect_target)"
  archive="${BINARY_NAME}-${target}.tar.gz"

  info "detected platform: ${target}"

  version_tag="$(resolve_version_path)"
  if [[ -n "${VERSION}" ]]; then
    info "installing version: ${VERSION}"
  else
    info "installing latest release"
  fi

  local archive_url checksum_url
  if [[ "${version_tag}" == "latest" ]]; then
    archive_url="${GITHUB}/releases/latest/download/${archive}"
    checksum_url="${GITHUB}/releases/latest/download/${archive}.sha256"
  else
    archive_url="${GITHUB}/releases/download/${version_tag}/${archive}"
    checksum_url="${GITHUB}/releases/download/${version_tag}/${archive}.sha256"
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' EXIT

  info "downloading ${archive_url}"
  download "${archive_url}" "${tmpdir}/${archive}"

  if [[ "${VERIFY}" == "1" ]]; then
    info "verifying checksum"
    download "${checksum_url}" "${tmpdir}/${archive}.sha256"
    verify_checksum "${tmpdir}/${archive}" "${tmpdir}/${archive}.sha256"
  fi

  tar xzf "${tmpdir}/${archive}" -C "${tmpdir}"

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
