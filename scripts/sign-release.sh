#!/usr/bin/env bash
# 生成 release/versions.json 并对所有 Release 产物签名。
# 在 CI 中调用；维护者需设置 ONEMINI_SIGNING_KEY（Base64 Ed25519 私钥）。

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="${1:-}"
TAG="${2:-}"

if [[ -z "${DIST}" || -z "${TAG}" ]]; then
  echo "usage: $0 <dist-dir> <tag>  # e.g. dist v0.1.1" >&2
  exit 1
fi

if [[ -z "${ONEMINI_SIGNING_KEY:-}" ]]; then
  echo "error: ONEMINI_SIGNING_KEY not set" >&2
  exit 1
fi

VERSION="${TAG#v}"
REPO="AJI1026/OneMini-CLI"
BASE="https://github.com/${REPO}/releases/download/${TAG}"
VERSIONS_JSON="${ROOT}/release/versions.json"

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }
}

need_cmd cargo
need_cmd python3

cd "${ROOT}"

sign_file() {
  cargo run --quiet --release --bin onemini-sign -- sign --file "$1"
}

platform_for() {
  case "$1" in
    onemini-x86_64-apple-darwin.tar.gz) echo "x86_64-apple-darwin" ;;
    onemini-aarch64-apple-darwin.tar.gz) echo "aarch64-apple-darwin" ;;
    onemini-x86_64-unknown-linux-gnu.tar.gz) echo "x86_64-unknown-linux-gnu" ;;
    onemini-aarch64-unknown-linux-gnu.tar.gz) echo "aarch64-unknown-linux-gnu" ;;
    onemini-x86_64-pc-windows-msvc.zip) echo "x86_64-pc-windows-msvc" ;;
    *) echo "" ;;
  esac
}

shopt -s nullglob
signed_any=0
for archive in "${DIST}"/onemini-*.{tar.gz,zip}; do
  [[ -f "${archive}" ]] || continue
  echo "==> signing $(basename "${archive}")"
  sign_file "${archive}"
  signed_any=1
done

if [[ "${signed_any}" -eq 0 ]]; then
  echo "error: no release artifacts in ${DIST}" >&2
  exit 1
fi

python3 - "${DIST}" "${TAG}" "${VERSION}" "${BASE}" "${VERSIONS_JSON}" <<'PY'
import hashlib
import json
import os
import sys

dist, tag, version, base, index_path = sys.argv[1:6]

mapping = {
    "onemini-x86_64-apple-darwin.tar.gz": "x86_64-apple-darwin",
    "onemini-aarch64-apple-darwin.tar.gz": "aarch64-apple-darwin",
    "onemini-x86_64-unknown-linux-gnu.tar.gz": "x86_64-unknown-linux-gnu",
    "onemini-aarch64-unknown-linux-gnu.tar.gz": "aarch64-unknown-linux-gnu",
    "onemini-x86_64-pc-windows-msvc.zip": "x86_64-pc-windows-msvc",
}

def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

assets = {}
for fname, platform in mapping.items():
    path = os.path.join(dist, fname)
    if not os.path.isfile(path):
        continue
    assets[platform] = {
        "url": f"{base}/{fname}",
        "sha256": sha256(path),
        "sig_url": f"{base}/{fname}.sig",
    }

if not assets:
    raise SystemExit("no mapped artifacts found")

if os.path.isfile(index_path):
    with open(index_path, encoding="utf-8") as f:
        index = json.load(f)
else:
    index = {"schema_version": 1, "latest": version, "releases": {}}

index["schema_version"] = 1
index["latest"] = version
index.setdefault("releases", {})
index["releases"][version] = {
    "tag": tag,
    "deprecated": False,
    "deprecation_reason": None,
    "assets": assets,
}

os.makedirs(os.path.dirname(index_path), exist_ok=True)
with open(index_path, "w", encoding="utf-8") as f:
    json.dump(index, f, indent=2)
    f.write("\n")
print(f"wrote {index_path}")
PY

echo "==> signing versions.json"
sign_file "${VERSIONS_JSON}"
echo "==> done"
