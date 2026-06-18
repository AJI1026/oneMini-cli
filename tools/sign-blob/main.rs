//! 维护者工具：生成 Ed25519 密钥、签名 blob、更新 versions.json.sig。
//!
//! ```bash
//! cargo run --bin onemini-sign -- keygen --out-dir release
//! ONEMINI_SIGNING_KEY=<base64-secret> cargo run --bin onemini-sign -- sign --file release/versions.json
//! ```

use anyhow::{Context, Result};
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("keygen") => cmd_keygen(&args[2..]),
        Some("sign") => cmd_sign(&args[2..]),
        Some("verify") => cmd_verify(&args[2..]),
        _ => {
            eprintln!(
                "用法:\n  \
                 onemini-sign keygen --out-dir release\n  \
                 onemini-sign sign --file PATH [--key-env ONEMINI_SIGNING_KEY]\n  \
                 onemini-sign verify --file PATH --sig PATH [--pubkey PATH]"
            );
            std::process::exit(1);
        }
    }
}

fn cmd_keygen(args: &[String]) -> Result<()> {
    let out_dir = arg_value(args, "--out-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("release"));
    fs::create_dir_all(&out_dir)?;

    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let secret_b64 = base64::engine::general_purpose::STANDARD.encode(signing_key.to_bytes());
    let public_b64 =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());

    let secret_path = out_dir.join("signing_secret_key.b64");
    let public_path = out_dir.join("signing_public_key.b64");
    fs::write(&public_path, format!("{public_b64}\n"))?;
    fs::write(&secret_path, format!("{secret_b64}\n"))?;

    println!("公钥已写入: {}", public_path.display());
    println!("私钥已写入: {}（勿提交到 Git）", secret_path.display());
    println!("请将 signing_public_key.b64 提交到仓库，私钥存入 GitHub Secret ONEMINI_SIGNING_KEY");
    println!("然后运行: ./scripts/sync-embedded-pubkey.sh（同步 install.sh / install.ps1 内置公钥）");
    Ok(())
}

fn cmd_sign(args: &[String]) -> Result<()> {
    let file = arg_value(args, "--file").context("缺少 --file")?;
    let key_env = arg_value(args, "--key-env").unwrap_or_else(|| "ONEMINI_SIGNING_KEY".to_string());
    let secret_b64 = std::env::var(&key_env)
        .with_context(|| format!("未设置环境变量 {key_env}（或 --key-env 指定其他变量）"))?;
    let signing_key = load_signing_key(&secret_b64)?;
    sign_file(&file, &signing_key)
}

fn cmd_verify(args: &[String]) -> Result<()> {
    let file = arg_value(args, "--file").context("缺少 --file")?;
    let sig = arg_value(args, "--sig")
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(format!("{file}.sig")));
    let pubkey_path = arg_value(args, "--pubkey")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("release/signing_public_key.b64"));

    let data = fs::read(&file).with_context(|| format!("读取失败: {file}"))?;
    let sig_text = fs::read_to_string(&sig).with_context(|| format!("读取失败: {}", sig.display()))?;
    let pubkey_b64 = fs::read_to_string(&pubkey_path)
        .with_context(|| format!("读取公钥失败: {}", pubkey_path.display()))?;

    verify_blob(&data, &sig_text, &pubkey_b64)?;
    println!("签名校验通过: {file}");
    Ok(())
}

fn sign_file(path: &str, signing_key: &SigningKey) -> Result<()> {
    let data = fs::read(path).with_context(|| format!("读取失败: {path}"))?;
    let digest = Sha256::digest(&data);
    let signature = signing_key.sign(&digest);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    let sig_path = format!("{path}.sig");
    fs::write(&sig_path, format!("{sig_b64}\n"))?;
    println!("已签名: {sig_path}");
    Ok(())
}

fn load_signing_key(secret_b64: &str) -> Result<SigningKey> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(secret_b64.trim())
        .context("私钥 Base64 解码失败")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("私钥长度无效（期望 32 字节 Ed25519 seed）"))?;
    Ok(SigningKey::from_bytes(&arr))
}

fn verify_blob(data: &[u8], sig_b64: &str, pubkey_b64: &str) -> Result<()> {
    use ed25519_dalek::{Signature, VerifyingKey};
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64.trim())
        .context("公钥 Base64 解码失败")?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("公钥长度无效"))?;
    let key = VerifyingKey::from_bytes(&key_array).context("无效公钥")?;

    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(sig_b64.trim())
        .context("签名 Base64 解码失败")?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("签名长度无效"))?;
    let signature = Signature::from_bytes(&sig_array);
    let digest = Sha256::digest(data);
    key.verify_strict(&digest, &signature)
        .map_err(|e| anyhow::anyhow!("签名校验失败: {e}"))
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
