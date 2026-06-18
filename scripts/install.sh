#!/usr/bin/env bash
# OneMini-CLI installer（HTTPS + Ed25519 签名校验）
# Usage:
#   curl -fL --progress-bar https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
#   ONEMINI_VERSION=0.1.0 curl -fL --progress-bar ... | bash
#   ONEMINI_MIRROR=https://ghproxy.com curl -fL --progress-bar ... | bash

set -euo pipefail

# 与 release/signing_public_key.b64 及 onemini update 内置公钥一致（勿从网络下载公钥）
EMBEDDED_SIGNING_PUBLIC_KEY_B64="TOZSDtW7+y9gjKglkfmBIZBkaQ/i9hxHOq6ws/xAg2Q="

REPO="AJI1026/OneMini-CLI"
BINARY_NAME="onemini"
GITHUB="https://github.com/${REPO}"
RAW_BASE="${ONEMINI_RAW_BASE:-https://raw.githubusercontent.com/${REPO}/main}"
VERSIONS_INDEX="${RAW_BASE}/release/versions.json"
VERSIONS_SIG="${RAW_BASE}/release/versions.json.sig"
VERIFY_PY="${RAW_BASE}/scripts/verify_signature.py"

INSTALL_DIR="${ONEMINI_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${ONEMINI_VERSION:-}"
IGNORE_DEPRECATED="${ONEMINI_IGNORE_DEPRECATED:-0}"
CONNECT_TIMEOUT="${ONEMINI_CONNECT_TIMEOUT:-15}"
DOWNLOAD_TIMEOUT="${ONEMINI_DOWNLOAD_TIMEOUT:-600}"
MIRROR="${ONEMINI_MIRROR:-}"

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
error() { printf '\033[1;31m错误:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "缺少必需命令: $1"
}

ensure_https() {
  case "$1" in
    https://*) ;;
    *) error "拒绝非 HTTPS 地址: $1" ;;
  esac
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Darwin)
      # 统一提供 Apple Silicon 构建；Intel Mac 可通过 Rosetta 运行
      printf 'mac-arm64'
      return
      ;;
    Linux)
      case "${arch}" in
        x86_64 | amd64) printf 'linux-x64'; return ;;
        arm64 | aarch64)
          error "Linux ARM 暂未提供预编译包，请使用: cargo install --path ."
          ;;
        *) error "不支持的 CPU 架构: ${arch}" ;;
      esac
      ;;
    *) error "不支持的操作系统: ${os}（macOS/Linux 请用 install.sh；Windows 请用 install.ps1）" ;;
  esac
}

mirror_url() {
  local url="$1"
  if [[ -z "${MIRROR}" ]]; then
    printf '%s' "${url}"
    return
  fi
  case "${url}" in
    https://github.com/*|https://raw.githubusercontent.com/*)
      printf '%s/%s' "${MIRROR%/}" "${url}"
      ;;
    *)
      printf '%s' "${url}"
      ;;
  esac
}

download_failed_hint() {
  local url="$1" code="$2"
  cat >&2 <<EOF
error: 下载失败 (exit ${code})
  URL: ${url}

常见原因:
  - 无法访问 GitHub / raw.githubusercontent.com（国内网络常见，会长时间无输出后超时）
  - Release 尚未发布或版本索引未更新

可尝试:
  1. 使用代理或 VPN 后重试
  2. 镜像加速: ONEMINI_MIRROR=https://ghproxy.com curl -fsSL .../install.sh | bash
  3. 自定义 raw 源: ONEMINI_RAW_BASE=https://your-mirror/.../main bash install.sh
  4. 源码安装: git clone ${GITHUB}.git && cd OneMini-CLI && cargo install --path .
EOF
  exit 1
}

download() {
  local url="$1" dest="$2"
  local label="${3:-${url##*/}}"
  url="$(mirror_url "${url}")"
  ensure_https "${url}"

  if command -v curl >/dev/null 2>&1; then
    local curl_common=(
      -fL
      --proto '=https'
      --tlsv1.2
      --connect-timeout "${CONNECT_TIMEOUT}"
      --max-time "${DOWNLOAD_TIMEOUT}"
      --retry 3
      --retry-delay 2
      -o "${dest}"
    )
    info "正在下载 ${label}"
    if [[ "${ONEMINI_QUIET:-0}" == "1" ]]; then
      curl -sS "${curl_common[@]}" "${url}" || download_failed_hint "${url}" "$?"
    elif [[ -t 2 ]]; then
      curl --progress-bar "${curl_common[@]}" "${url}" || download_failed_hint "${url}" "$?"
      printf '\n'
    else
      curl -# "${curl_common[@]}" "${url}" || download_failed_hint "${url}" "$?"
      printf '\n'
    fi
  elif command -v wget >/dev/null 2>&1; then
    info "正在下载 ${label}"
    if [[ "${ONEMINI_QUIET:-0}" == "1" ]]; then
      wget -qO "${dest}" "${url}" || download_failed_hint "${url}" "$?"
    else
      wget --show-progress -O "${dest}" "${url}" || download_failed_hint "${url}" "$?"
    fi
  else
    error "需要 curl 或 wget 才能下载"
  fi
}

verify_blob() {
  local file="$1" sig_file="$2" pubkey_file="$3" verify_py="$4"
  python3 "${verify_py}" --file "${file}" --sig "${sig_file}" --pubkey "${pubkey_file}"
}

PATH_MARKER_BEGIN="# >>> onemini >>>"
PATH_MARKER_END="# <<< onemini <<<"

detect_shell_profile() {
  local shell_name="${SHELL:-}"
  shell_name="$(basename "${shell_name}")"

  case "${shell_name}" in
    zsh)
      printf '%s\n' "${HOME}/.zshrc"
      ;;
    bash)
      if [[ "$(uname -s)" == "Darwin" && -f "${HOME}/.bash_profile" ]]; then
        printf '%s\n' "${HOME}/.bash_profile"
      else
        printf '%s\n' "${HOME}/.bashrc"
      fi
      ;;
    fish)
      printf '%s\n' "${HOME}/.config/fish/config.fish"
      ;;
    *)
      if [[ -f "${HOME}/.profile" ]]; then
        printf '%s\n' "${HOME}/.profile"
      elif [[ -f "${HOME}/.bashrc" ]]; then
        printf '%s\n' "${HOME}/.bashrc"
      else
        printf '%s\n' "${HOME}/.profile"
      fi
      ;;
  esac
}

path_block_for_shell() {
  local shell_name="${SHELL:-}"
  shell_name="$(basename "${shell_name}")"

  case "${shell_name}" in
    fish)
      cat <<EOF
${PATH_MARKER_BEGIN}
fish_add_path -a "${INSTALL_DIR}"
${PATH_MARKER_END}
EOF
      ;;
    *)
      cat <<EOF
${PATH_MARKER_BEGIN}
export PATH="${INSTALL_DIR}:\$PATH"
${PATH_MARKER_END}
EOF
      ;;
  esac
}

ensure_path_in_shell() {
  if [[ "${ONEMINI_SKIP_PATH:-0}" == "1" ]]; then
    warn "已设置 ONEMINI_SKIP_PATH=1，跳过自动配置 PATH"
    return
  fi

  if [[ ":${PATH}:" == *":${INSTALL_DIR}:"* ]]; then
    info "${INSTALL_DIR} 已在 PATH 中"
    return
  fi

  local profile block
  profile="$(detect_shell_profile)"
  mkdir -p "$(dirname "${profile}")"

  if [[ -f "${profile}" ]] && grep -qF "${PATH_MARKER_BEGIN}" "${profile}"; then
    info "onemini PATH 配置块已存在于 ${profile}"
  else
    block="$(path_block_for_shell)"
    if [[ -f "${profile}" ]]; then
      printf '\n%s\n' "${block}" >> "${profile}"
    else
      printf '%s\n' "${block}" > "${profile}"
    fi
    info "已将 ${INSTALL_DIR} 写入 ${profile} 的 PATH"
  fi

  export PATH="${INSTALL_DIR}:${PATH}"
  warn "请重新打开终端，或运行: source ${profile}"
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

  info "检测到平台: ${target}"
  info "正在获取已签名的 versions.json"

  download "${VERSIONS_INDEX}" "${tmpdir}/versions.json"
  download "${VERSIONS_SIG}" "${tmpdir}/versions.json.sig"
  download "${VERIFY_PY}" "${tmpdir}/verify_signature.py"
  printf '%s\n' "${EMBEDDED_SIGNING_PUBLIC_KEY_B64}" > "${tmpdir}/signing_public_key.b64"

  info "正在校验 versions.json 签名"
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
    error "该版本已弃用: ${reason}。如需继续，请设置 ONEMINI_IGNORE_DEPRECATED=1"
  elif [[ "${deprecated}" == "1" ]]; then
    warn "正在安装已弃用版本（${reason}）"
  fi

  ensure_https "${archive_url}"
  ensure_https "${sig_url}"

  download "${archive_url}" "${tmpdir}/archive" "${BINARY_NAME} (${target})"
  download "${sig_url}" "${tmpdir}/archive.sig" "signature"

  info "正在校验发布包签名"
  verify_blob "${tmpdir}/archive" "${tmpdir}/archive.sig" \
    "${tmpdir}/signing_public_key.b64" "${tmpdir}/verify_signature.py"

  local actual expected
  expected="${sha256}"
  if command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "${tmpdir}/archive" | awk '{print $1}')"
  else
    actual="$(sha256sum "${tmpdir}/archive" | awk '{print $1}')"
  fi
  [[ "${expected}" == "${actual}" ]] || error "SHA256 校验失败（期望 ${expected}，实际 ${actual}）"
  info "SHA256 校验通过"

  case "${archive_url}" in
    *.tar.gz)
      tar xzf "${tmpdir}/archive" -C "${tmpdir}"
      ;;
    *.zip)
      need_cmd unzip
      unzip -q "${tmpdir}/archive" -d "${tmpdir}"
      ;;
    *)
      error "不支持的压缩包格式: ${archive_url}"
      ;;
  esac

  mkdir -p "${INSTALL_DIR}"
  install -m 755 "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  SKILLS_DIR="${ONEMINI_SKILLS_DIR:-${HOME}/.local/share/onemini/skills}"
  if [[ -d "${tmpdir}/skills" ]]; then
    mkdir -p "${SKILLS_DIR}"
    cp -R "${tmpdir}/skills/." "${SKILLS_DIR}/"
    info "已安装文档技能脚本 -> ${SKILLS_DIR}"
  fi

  info "已安装 ${BINARY_NAME} -> ${INSTALL_DIR}/${BINARY_NAME}"

  ensure_path_in_shell

  if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
    info "运行: ${BINARY_NAME} --help"
  fi
}

main "$@"
