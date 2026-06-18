#!/usr/bin/env bash
# 确保 install.sh / install.ps1 内置公钥与 release/signing_public_key.b64 一致。
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

extract_sh() {
  sed -n 's/^EMBEDDED_SIGNING_PUBLIC_KEY_B64="\(.*\)"/\1/p' "${INSTALL_SH}" | head -1
}

extract_ps1() {
  sed -n 's/^\$EmbeddedSigningPublicKeyB64 = "\(.*\)"/\1/p' "${INSTALL_PS1}" | head -1
}

sh_key="$(extract_sh)"
ps1_key="$(extract_ps1)"
failed=0

if [[ "${sh_key}" != "${canonical}" ]]; then
  echo "error: scripts/install.sh embedded pubkey mismatch" >&2
  echo "  expected: ${canonical}" >&2
  echo "  actual:   ${sh_key:-<missing>}" >&2
  failed=1
fi

if [[ "${ps1_key}" != "${canonical}" ]]; then
  echo "error: scripts/install.ps1 embedded pubkey mismatch" >&2
  echo "  expected: ${canonical}" >&2
  echo "  actual:   ${ps1_key:-<missing>}" >&2
  failed=1
fi

if [[ "${failed}" -ne 0 ]]; then
  echo "hint: run ./scripts/sync-embedded-pubkey.sh" >&2
  exit 1
fi

echo "embedded signing public keys are in sync"
