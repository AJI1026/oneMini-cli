use anyhow::{bail, Context, Result};
use semver::Version;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "AJI1026/OneMini-CLI";
const BINARY: &str = "onemini";
const USER_AGENT: &str = "onemini-cli-updater";

pub struct UpdateOptions {
    pub check_only: bool,
    pub version: Option<String>,
    pub force: bool,
}

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub async fn run(opts: UpdateOptions) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let current = parse_version(current_version())?;
    let tag = resolve_target_tag(&client, opts.version.as_deref()).await?;
    let remote = parse_version(tag.trim_start_matches('v'))?;

    println!(
        "{}",
        crate::ui::status_pair("当前版本", &format!("v{current}"))
    );
    println!(
        "{}",
        crate::ui::status_pair("目标版本", &tag)
    );

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

    let exe = std::env::current_exe().context("无法定位当前可执行文件")?;
    let install_dir = exe
        .parent()
        .context("可执行文件路径无效")?
        .to_path_buf();

    let tmp = tempfile_dir()?;
    let archive = tmp.join(format!("{BINARY}-{platform}.tar.gz"));
    let extract_dir = tmp.join("extract");
    std::fs::create_dir_all(&extract_dir)?;

    let base = format!("https://github.com/{REPO}/releases/download/{tag}");
    let archive_url = format!("{base}/{BINARY}-{platform}.tar.gz");

    println!("{}", crate::ui::dim(&format!("下载: {archive_url}")));
    download_file(&client, &archive_url, &archive).await?;

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

async fn resolve_target_tag(client: &reqwest::Client, version: Option<&str>) -> Result<String> {
    if let Some(v) = version {
        let tag = normalize_tag(v);
        let url = format!("https://api.github.com/repos/{REPO}/releases/tags/{tag}");
        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("查询 release {tag} 失败"))?;
        if !resp.status().is_success() {
            bail!("GitHub 上不存在 release: {tag}");
        }
        let info: ReleaseInfo = resp.json().await.context("解析 release 信息失败")?;
        return Ok(info.tag_name);
    }

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = client
        .get(&url)
        .send()
        .await
        .context("查询 latest release 失败")?;
    if !resp.status().is_success() {
        bail!("无法获取 latest release（HTTP {}）", resp.status());
    }
    let info: ReleaseInfo = resp.json().await.context("解析 latest release 失败")?;
    Ok(info.tag_name)
}

fn normalize_tag(v: &str) -> String {
    if v.starts_with('v') {
        v.to_string()
    } else {
        format!("v{v}")
    }
}

fn parse_version(s: &str) -> Result<Version> {
    Version::parse(s).with_context(|| format!("无效版本号: {s}"))
}

fn detect_platform() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let os_tag = match os {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => bail!("不支持的操作系统: {other}（仅 macOS / Linux 支持 onemini update）"),
    };
    let arch_tag = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => bail!("不支持架构: {other}"),
    };
    Ok(format!("{arch_tag}-{os_tag}"))
}

async fn download_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<()> {
    let bytes = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("下载失败: {url}"))?
        .error_for_status()
        .with_context(|| format!("下载失败: {url}"))?
        .bytes()
        .await?;
    std::fs::write(dest, bytes).with_context(|| format!("写入失败: {}", dest.display()))?;
    Ok(())
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
        bail!("解压失败，exit code: {:?}", status.code());
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

    #[test]
    fn normalize_tag_adds_v() {
        assert_eq!(normalize_tag("0.1.1"), "v0.1.1");
        assert_eq!(normalize_tag("v0.1.1"), "v0.1.1");
    }

    #[test]
    fn semver_patch_bump() {
        let a = parse_version("0.1.0").unwrap();
        let b = parse_version("0.1.1").unwrap();
        assert!(b > a);
    }
}
