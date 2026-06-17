use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const NONCE_LEN: usize = 12;
const KEYCHAIN_PLACEHOLDER: &str = "keychain:onemini";

pub fn encrypt_bytes(plaintext: &[u8]) -> Result<Vec<u8>> {
    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).context("初始化加密器失败")?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| anyhow::anyhow!("生成 nonce 失败: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("加密失败: {e}"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_bytes(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN {
        anyhow::bail!("加密数据过短");
    }
    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).context("初始化解密器失败")?;
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("解密失败（密钥或数据损坏）: {e}"))
}

fn derive_key() -> Result<[u8; 32]> {
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
    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    hasher.update(username.as_bytes());
    hasher.update(config_dir.display().to_string().as_bytes());
    hasher.update(machine_id.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
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
