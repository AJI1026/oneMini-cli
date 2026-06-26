use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use std::fs;
use std::path::Path;

const NONCE_LEN: usize = 12;
const KEYCHAIN_PLACEHOLDER: &str = "keychain:onemini";
/// PBKDF2 迭代次数（OWASP 2023 推荐值：600,000 次 SHA-256）
const PBKDF2_ROUNDS: u32 = 600_000;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

pub fn encrypt_bytes(plaintext: &[u8]) -> Result<Vec<u8>> {
    let (key, salt) = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).context("初始化加密器失败")?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| anyhow::anyhow!("生成 nonce 失败: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("加密失败: {e}"))?;
    // 格式: salt(16) + nonce(12) + ciphertext
    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let min_len = SALT_LEN + NONCE_LEN;
    if data.len() < min_len {
        anyhow::bail!("加密数据过短");
    }
    let (salt, rest) = data.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let key = derive_key_with_salt(salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key).context("初始化解密器失败")?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("解密失败（密钥或数据损坏）: {e}"))
}

fn derive_key() -> Result<([u8; KEY_LEN], [u8; SALT_LEN])> {
    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt).map_err(|e| anyhow::anyhow!("生成盐失败: {e}"))?;
    let key = derive_key_with_salt(&salt)?;
    Ok((key, salt))
}

fn derive_key_with_salt(salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let config_dir = crate::config::Config::config_dir()?;
    let machine_id_path = config_dir.join(".machine_id");
    let machine_id = if machine_id_path.exists() {
        fs::read_to_string(&machine_id_path)?.trim().to_string()
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        crate::fs_util::write_private(&machine_id_path, &id)?;
        id
    };

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let username = whoami::username();

    // 构造密码材料
    let mut password = Vec::new();
    password.extend_from_slice(hostname.as_bytes());
    password.extend_from_slice(username.as_bytes());
    password.extend_from_slice(config_dir.display().to_string().as_bytes());
    password.extend_from_slice(machine_id.as_bytes());

    // 使用 PBKDF2-HMAC-SHA256（pbkdf2 crate），替代原始 SHA-256
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(&password, salt, PBKDF2_ROUNDS, &mut key);
    // 安全清零密码缓冲区
    password.fill(0);
    Ok(key)
}

pub fn write_encrypted(path: &Path, plaintext: &[u8]) -> Result<()> {
    let enc = encrypt_bytes(plaintext)?;
    crate::fs_util::write_private(path, enc)
}

pub fn read_encrypted(path: &Path) -> Result<Vec<u8>> {
    let data = fs::read(path).with_context(|| format!("读取加密文件失败: {}", path.display()))?;
    decrypt_bytes(&data)
}

pub fn is_keychain_placeholder(value: &str) -> bool {
    value == KEYCHAIN_PLACEHOLDER
}

pub const API_KEY_KEYCHAIN_PLACEHOLDER: &str = KEYCHAIN_PLACEHOLDER;

#[cfg(feature = "keychain")]
pub fn store_api_key_in_keychain(key: &str) -> Result<()> {
    let entry = keyring::Entry::new("onemini-cli", "api_key")?;
    entry.set_password(key).context("写入系统钥匙串失败")?;
    Ok(())
}

#[cfg(feature = "keychain")]
pub fn load_api_key_from_keychain() -> Result<String> {
    let entry = keyring::Entry::new("onemini-cli", "api_key")?;
    entry.get_password().context("从系统钥匙串读取 API 密钥失败")
}

#[cfg(not(feature = "keychain"))]
pub fn store_api_key_in_keychain(_key: &str) -> Result<()> {
    Ok(())
}

#[cfg(not(feature = "keychain"))]
pub fn load_api_key_from_keychain() -> Result<String> {
    anyhow::bail!("未编译钥匙串支持")
}
