//! Release 下载安全：HTTPS-only、TLS 1.2+、Ed25519 签名校验。

use anyhow::{bail, Context, Result};
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// 编译时注入的 Release 签名公钥（Base64，32 字节 Ed25519）。
pub const SIGNING_PUBLIC_KEY_B64: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/release/signing_public_key.b64"));

/// 版本索引固定 HTTPS 地址（禁止 HTTP 回退）。
pub const VERSIONS_INDEX_URL: &str =
    "https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/release/versions.json";

pub const VERSIONS_SIG_URL: &str =
    "https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/release/versions.json.sig";

pub fn secure_http_client() -> Result<reqwest::Client> {
    crate::fs_util::secure_http_client("onemini-cli-updater", 120)
}

pub fn ensure_https_url(url: &str) -> Result<()> {
    let parsed = url
        .parse::<reqwest::Url>()
        .with_context(|| format!("无效 URL: {url}"))?;
    if parsed.scheme() != "https" {
        bail!("拒绝非 HTTPS 下载地址: {url}（不允许 HTTP 回退）");
    }
    Ok(())
}

pub async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    ensure_https_url(url)?;
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("下载失败: {url}"))?
        .error_for_status()
        .with_context(|| format!("下载失败: {url}"))?;
    let final_url = resp.url().to_string();
    ensure_https_url(&final_url)?;
    Ok(resp.bytes().await?.to_vec())
}

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<()> {
    let actual = sha256_hex(data);
    if actual.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        bail!(
            "SHA256 校验失败（仅防传输错误，不能替代签名校验）: 期望 {expected_hex}, 实际 {actual}"
        )
    }
}

fn load_verifying_key() -> Result<VerifyingKey> {
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(SIGNING_PUBLIC_KEY_B64.trim())
        .context("解析内置签名公钥失败")?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("签名公钥长度无效（期望 32 字节 Ed25519）"))?;
    VerifyingKey::from_bytes(&key_array).context("无效的 Ed25519 公钥")
}

/// 校验 `.sig` 文件（Base64 Ed25519，签名对象为 SHA256 摘要，与 Cosign sign-blob 语义一致）。
pub fn verify_signature(data: &[u8], sig_b64: &str) -> Result<()> {
    let key = load_verifying_key()?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(sig_b64.trim())
        .context("解析 .sig 文件失败（非 Base64）")?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!(".sig 长度无效（期望 64 字节 Ed25519 签名）"))?;
    let signature = Signature::from_bytes(&sig_array);
    let digest = Sha256::digest(data);
    key.verify_strict(&digest, &signature)
        .map_err(|e| anyhow::anyhow!("签名校验失败: {e}（文件可能被篡改）"))
}

pub async fn download_and_verify(
    client: &reqwest::Client,
    url: &str,
    sig_url: &str,
    expected_sha256: Option<&str>,
) -> Result<Vec<u8>> {
    ensure_https_url(url)?;
    ensure_https_url(sig_url)?;

    let data = download_bytes(client, url).await?;
    let sig = download_bytes(client, sig_url).await?;
    let sig_text = String::from_utf8(sig).context(".sig 文件不是有效 UTF-8")?;

    verify_signature(&data, &sig_text)?;

    if let Some(expected) = expected_sha256 {
        verify_sha256(&data, expected)?;
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;

    #[test]
    fn rejects_http_url() {
        assert!(ensure_https_url("http://example.com/file").is_err());
        assert!(ensure_https_url("https://example.com/file").is_ok());
    }

    #[test]
    fn verify_roundtrip_with_embedded_key_format() {
        // 仅验证 API 形态；真实签名校验依赖 release/signing_public_key.b64
        let sk = SigningKey::generate(&mut OsRng);
        let digest = Sha256::digest(b"hello");
        let sig = sk.sign(&digest);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());
        sk.verifying_key()
            .verify_strict(&digest, &sig)
            .expect("self verify");
        assert!(!sig_b64.is_empty());
    }
}
