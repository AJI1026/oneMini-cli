mod index;
mod security;

pub use security::verify_signature;

use anyhow::{bail, Context, Result};
use index::{asset_sig_url, VersionsIndex};
use semver::Version;
use security::{download_and_verify, secure_http_client, VERSIONS_INDEX_URL, VERSIONS_SIG_URL};
use std::path::{Path, PathBuf};
use std::process::Command;

const BINARY: &str = "onemini";

pub struct UpdateOptions {
    pub check_only: bool,
    pub version: Option<String>,
    pub force: bool,
    pub ignore_deprecated: bool,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub async fn run(opts: UpdateOptions) -> Result<()> {
    let client = secure_http_client()?;

    let current = parse_version(current_version())?;
    let (version_key, release) = fetch_release_entry(&client, opts.version.as_deref()).await?;
    let remote = parse_version(&version_key)?;
    let tag = &release.tag;

    println!(
        "{}",
        crate::ui::status_pair("当前版本", &format!("v{current}"))
    );
    println!("{}", crate::ui::status_pair("目标版本", tag));

    if release.deprecated {
        let reason = release
            .deprecation_reason
            .as_deref()
            .unwrap_or("该版本存在已知安全问题");
        let msg = format!("警告: {tag} 已标记为弃用 — {reason}");
        if opts.ignore_deprecated {
            println!("{}", crate::ui::warn(&format!("{msg}（已使用 --ignore-deprecated 继续）")));
        } else {
            bail!(
                "{msg}\n如需继续安装，请显式添加 --ignore-deprecated（忽略弃用警告）"
            );
        }
    }

    if remote <= current && !opts.force {
        if remote == current {
            println!("{}", crate::ui::success("已是最新版本"));
        } else {
            println!(
                "{}",
                crate::ui::warn(&format!(
                    "远程版本 v{remote} 低于当前 v{current}，已跳过（可用 --force 强制安装）"
                ))
            );
        }
        return Ok(());
    }

    if opts.check_only {
        println!(
            "{}",
            crate::ui::success(&format!("有新版本可用: v{current} -> {tag}"))
        );
        println!("{}", crate::ui::dim("运行 onemini update 开始更新"));
        return Ok(());
    }

    let platform = detect_platform()?;
    println!("{}", crate::ui::dim(&format!("平台: {platform}")));

    let asset = index::VersionsIndex::asset_for_platform(&release, &platform)?;
    security::ensure_https_url(&asset.url)?;
    let sig_url = asset_sig_url(asset);
    security::ensure_https_url(&sig_url)?;

    let exe = std::env::current_exe().context("无法定位当前可执行文件")?;
    let install_dir = exe
        .parent()
        .context("可执行文件路径无效")?
        .to_path_buf();

    let tmp = tempfile_dir()?;
    let archive = tmp.join(format!("{BINARY}-{platform}.tar.gz"));
    let extract_dir = tmp.join("extract");
    std::fs::create_dir_all(&extract_dir)?;

    println!("{}", crate::ui::dim(&format!("下载: {}", asset.url)));
    println!("{}", crate::ui::dim("校验 Ed25519 签名…"));
    let archive_bytes = download_and_verify(
        &client,
        &asset.url,
        &sig_url,
        Some(&asset.sha256),
    )
    .await?;
    std::fs::write(&archive, &archive_bytes)
        .with_context(|| format!("写入失败: {}", archive.display()))?;

    extract_tar(&archive, &extract_dir)?;
    let new_bin = extract_dir.join(BINARY);
    if !new_bin.is_file() {
        bail!("压缩包中未找到 {BINARY} 二进制");
    }

    let staged = install_dir.join(format!("{BINARY}.new"));
    std::fs::copy(&new_bin, &staged)
        .with_context(|| format!("无法写入 {}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }

    replace_executable(&exe, &staged)?;
    println!(
        "{}",
        crate::ui::success(&format!("已更新到 {tag} -> {}", exe.display()))
    );
    println!(
        "{}",
        crate::ui::dim("请重新打开终端，或运行 onemini --version 确认")
    );
    Ok(())
}

async fn fetch_release_entry(
    client: &reqwest::Client,
    requested: Option<&str>,
) -> Result<(String, index::ReleaseEntry)> {
    println!("{}", crate::ui::dim("获取并校验版本索引…"));
    let index_bytes =
        download_and_verify(client, VERSIONS_INDEX_URL, VERSIONS_SIG_URL, None).await?;
    let index = VersionsIndex::parse(&index_bytes)?;

    let (version_key, entry) = index.resolve_version(requested)?;
    Ok((version_key.to_string(), entry.clone()))
}

fn parse_version(s: &str) -> Result<Version> {
    Version::parse(s).with_context(|| format!("无效版本号: {s}"))
}

fn detect_platform() -> Result<String> {
    match std::env::consts::OS {
        "macos" => match std::env::consts::ARCH {
            "aarch64" => Ok("mac-arm64".into()),
            "x86_64" => Ok("mac-x64".into()),
            other => bail!(
                "不支持的 macOS CPU 架构: {other}，请从源码编译: cargo install --path ."
            ),
        },
        "linux" if std::env::consts::ARCH == "x86_64" => Ok("linux-x64".into()),
        "linux" => bail!(
            "Linux ARM 暂未提供预编译包，请从源码编译: cargo install --path ."
        ),
        other => bail!("不支持的操作系统: {other}（仅 macOS / Linux 支持 onemini update）"),
    }
}

fn extract_tar(archive: &Path, dest: &Path) -> Result<()> {
    let status = Command::new("tar")
        .args([
            "xzf",
            archive.to_str().context("路径无效")?,
            "-C",
            dest.to_str().context("路径无效")?,
        ])
        .status()
        .context("执行 tar 失败（请确认系统已安装 tar）")?;
    if !status.success() {
        bail!("解压失败，退出码: {:?}", status.code());
    }
    Ok(())
}

fn replace_executable(target: &Path, staged: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let script = format!(
            r#"#!/bin/sh
sleep 1
mv -f "{staged}" "{target}"
chmod +x "{target}"
"#,
            staged = staged.display(),
            target = target.display()
        );
        let script_path = staged.with_extension("sh");
        std::fs::write(&script_path, script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
        }
        Command::new("sh")
            .arg(&script_path)
            .spawn()
            .context("启动更新脚本失败")?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        std::fs::rename(staged, target).with_context(|| {
            format!(
                "无法替换 {}（Windows 请手动下载 Release 覆盖）",
                target.display()
            )
        })?;
        Ok(())
    }
}

fn tempfile_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("onemini-update-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::index::normalize_version_key;

    #[test]
    fn semver_patch_bump() {
        let a = parse_version("0.1.0").unwrap();
        let b = parse_version("0.1.1").unwrap();
        assert!(b > a);
    }

    #[test]
    fn normalize_version_key_works() {
        assert_eq!(normalize_version_key("v0.1.1").unwrap(), "0.1.1");
        assert_eq!(normalize_version_key("0.1.1").unwrap(), "0.1.1");
    }
}
