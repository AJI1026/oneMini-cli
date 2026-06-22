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

# 规范为绝对路径，避免在临时目录内 zip 时相对路径指向错误位置
if [[ "${DIST}" != /* ]]; then
  DIST="${ROOT}/${DIST}"
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
need_cmd unzip
need_cmd zip

cd "${ROOT}"

sign_file() {
  cargo run --quiet --release --bin onemini-sign -- sign --file "$1"
}

embed_inner_signatures() {
  local win_zip="${DIST}/onemini-win-x64.zip"
  if [[ -f "${win_zip}" ]]; then
    echo "==> embedding onemini.exe.sig into Windows zip"
    local tmp
    tmp="$(mktemp -d)"
    unzip -q "${win_zip}" -d "${tmp}"
    sign_file "${tmp}/onemini.exe"
    rm -f "${win_zip}"
    (cd "${tmp}" && zip -qr "${win_zip}" .)
    rm -rf "${tmp}"
  fi

  local mac_bundle="${DIST}/onemini-mac-arm64-bundle.zip"
  if [[ -f "${mac_bundle}" ]]; then
    echo "==> embedding onemini.sig into macOS app bundle"
    local tmp
    tmp="$(mktemp -d)"
    unzip -q "${mac_bundle}" -d "${tmp}"
    local binary="${tmp}/OneMini.app/Contents/MacOS/onemini"
    local resources="${tmp}/OneMini.app/Contents/Resources"
    if [[ ! -f "${binary}" ]]; then
      echo "error: missing ${binary}" >&2
      exit 1
    fi
    sign_file "${binary}"
    mv "${binary}.sig" "${resources}/onemini.sig"
    rm -f "${mac_bundle}"
    (cd "${tmp}" && zip -qr "${mac_bundle}" OneMini.app)
    rm -rf "${tmp}"
  fi
}

embed_inner_signatures

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
    "onemini-mac-arm64.tar.gz": ("mac-arm64", "archive"),
    "onemini-mac-arm64-bundle.zip": ("mac-arm64", "bundle"),
    "onemini-mac-x64.tar.gz": ("mac-x64", "archive"),
    "onemini-linux-x64.tar.gz": ("linux-x64", "archive"),
    "onemini-win-x64.zip": ("win-x64", "archive"),
}

def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def asset_entry(path: str, fname: str) -> dict:
    return {
        "url": f"{base}/{fname}",
        "sha256": sha256(path),
        "sig_url": f"{base}/{fname}.sig",
    }

assets = {}
for fname, (platform, kind) in mapping.items():
    path = os.path.join(dist, fname)
    if not os.path.isfile(path):
        continue
    entry = asset_entry(path, fname)
    if kind == "archive":
        assets[platform] = entry
    elif kind == "bundle":
        assets.setdefault(platform, {})
        assets[platform]["bundle"] = entry

if not assets:
    raise SystemExit("no mapped artifacts found")

for platform, entry in list(assets.items()):
    if "url" not in entry:
        raise SystemExit(f"missing archive for platform: {platform}")

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
