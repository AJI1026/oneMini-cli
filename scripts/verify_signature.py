#!/usr/bin/env python3
"""Verify OneMini release Ed25519 signatures (SHA256 digest as message)."""

from __future__ import annotations

import argparse
import base64
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

OPENSSL_CANDIDATES = (
    "openssl",
    "/opt/homebrew/opt/openssl@3/bin/openssl",
    "/opt/homebrew/opt/openssl/bin/openssl",
    "/opt/homebrew/bin/openssl",
    "/usr/local/opt/openssl@3/bin/openssl",
    "/usr/local/opt/openssl/bin/openssl",
    "/usr/local/bin/openssl",
)


def ed25519_spki_pem(raw_pubkey: bytes) -> str:
    if len(raw_pubkey) != 32:
        raise ValueError("Ed25519 public key must be 32 bytes")
    spki = (
        bytes([0x30, 0x2A, 0x30, 0x05, 0x06, 0x03, 0x2B, 0x65, 0x70, 0x03, 0x21, 0x00])
        + raw_pubkey
    )
    b64 = base64.standard_b64encode(spki).decode("ascii")
    lines = "\n".join(b64[i : i + 64] for i in range(0, len(b64), 64))
    return f"-----BEGIN PUBLIC KEY-----\n{lines}\n-----END PUBLIC KEY-----\n"


def _find_openssl() -> str | None:
    seen: set[str] = set()
    for candidate in OPENSSL_CANDIDATES:
        path = shutil.which(candidate) if os.path.basename(candidate) == candidate else candidate
        if not path or path in seen or not os.path.isfile(path):
            continue
        seen.add(path)
        try:
            proc = subprocess.run(
                [path, "version"],
                capture_output=True,
                text=True,
                check=False,
            )
        except OSError:
            continue
        version = (proc.stdout or proc.stderr or "").strip()
        if "LibreSSL" in version:
            continue
        return path
    return None


def _verify_with_cryptography(pubkey: bytes, digest: bytes, signature: bytes) -> None:
    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

    key = Ed25519PublicKey.from_public_bytes(pubkey)
    try:
        key.verify(signature, digest)
    except InvalidSignature as exc:
        raise RuntimeError("invalid signature") from exc


def _verify_with_openssl(
    openssl: str, pubkey: bytes, digest: bytes, signature: bytes
) -> None:
    pem = ed25519_spki_pem(pubkey)
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        pem_path = tmp_path / "pub.pem"
        digest_path = tmp_path / "digest.bin"
        sig_path = tmp_path / "sig.bin"
        pem_path.write_text(pem, encoding="utf-8")
        digest_path.write_bytes(digest)
        sig_path.write_bytes(signature)

        proc = subprocess.run(
            [
                openssl,
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                str(pem_path),
                "-in",
                str(digest_path),
                "-sigfile",
                str(sig_path),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        if proc.returncode != 0:
            stderr = (proc.stderr or proc.stdout or "").strip()
            raise RuntimeError(stderr or "invalid signature")


def verify_signature(data: bytes, sig_b64: str, pubkey_b64: str) -> None:
    digest = hashlib.sha256(data).digest()
    signature = base64.standard_b64decode(sig_b64.strip())
    if len(signature) != 64:
        raise ValueError(".sig must decode to 64 bytes")
    pubkey = base64.standard_b64decode(pubkey_b64.strip())
    if len(pubkey) != 32:
        raise ValueError("Ed25519 public key must be 32 bytes")

    errors: list[str] = []

    try:
        _verify_with_cryptography(pubkey, digest, signature)
        return
    except ImportError:
        errors.append("cryptography 未安装")
    except RuntimeError as exc:
        errors.append(f"cryptography: {exc}")
    except Exception as exc:  # noqa: BLE001 - aggregate verify backends
        errors.append(f"cryptography: {exc}")

    openssl = _find_openssl()
    if openssl:
        try:
            _verify_with_openssl(openssl, pubkey, digest, signature)
            return
        except RuntimeError as exc:
            errors.append(f"openssl ({openssl}): {exc}")
        except Exception as exc:  # noqa: BLE001 - aggregate verify backends
            errors.append(f"openssl ({openssl}): {exc}")
    else:
        errors.append(
            "未找到支持 Ed25519 的 OpenSSL（macOS 自带 LibreSSL 不支持）；"
            "可执行: brew install openssl，或 pip3 install cryptography"
        )

    raise RuntimeError("signature verification failed: " + "; ".join(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", required=True, help="File to verify")
    parser.add_argument("--sig", required=True, help="Path to .sig file")
    parser.add_argument("--pubkey", required=True, help="Path to signing_public_key.b64")
    args = parser.parse_args()

    data = Path(args.file).read_bytes()
    sig_b64 = Path(args.sig).read_text(encoding="utf-8")
    pubkey_b64 = Path(args.pubkey).read_text(encoding="utf-8")
    verify_signature(data, sig_b64, pubkey_b64)
    print(f"signature ok: {args.file}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - CLI tool
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
