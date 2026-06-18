#!/usr/bin/env bash
# 将 release/signing_public_key.b64 同步到 install.sh / install.ps1 内置公钥。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUBKEY_FILE="${ROOT}/release/signing_public_key.b64"
INSTALL_SH="${ROOT}/scripts/install.sh"
INSTALL_PS1="${ROOT}/scripts/install.ps1"

if [[ ! -f "${PUBKEY_FILE}" ]]; then
  echo "error: missing ${PUBKEY_FILE}" >&2
  exit 1
fi

canonical="$(tr -d '\n\r' < "${PUBKEY_FILE}")"
if [[ -z "${canonical}" ]]; then
  echo "error: ${PUBKEY_FILE} is empty" >&2
  exit 1
fi

tmp_sh="$(mktemp)"
tmp_ps1="$(mktemp)"
trap 'rm -f "${tmp_sh}" "${tmp_ps1}"' EXIT

awk -v key="${canonical}" '
  /^EMBEDDED_SIGNING_PUBLIC_KEY_B64="/ {
    print "EMBEDDED_SIGNING_PUBLIC_KEY_B64=\"" key "\""
    next
  }
  { print }
' "${INSTALL_SH}" > "${tmp_sh}"

awk -v key="${canonical}" '
  /^\$EmbeddedSigningPublicKeyB64 = "/ {
    print "$EmbeddedSigningPublicKeyB64 = \"" key "\""
    next
  }
  { print }
' "${INSTALL_PS1}" > "${tmp_ps1}"

mv "${tmp_sh}" "${INSTALL_SH}"
mv "${tmp_ps1}" "${INSTALL_PS1}"
chmod +x "${ROOT}/scripts/check-embedded-pubkey.sh" "${ROOT}/scripts/sync-embedded-pubkey.sh"

echo "synced embedded pubkey to:"
echo "  ${INSTALL_SH}"
echo "  ${INSTALL_PS1}"

"${ROOT}/scripts/check-embedded-pubkey.sh"
