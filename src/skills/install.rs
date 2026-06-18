//! 从 GitHub 安装技能目录（含 SKILL.md、scripts/、辅助文件）

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::catalog::{self, CatalogEntry, ANTHROPIC_REPO};

#[derive(Debug, Deserialize)]
struct GitHubContent {
    name: String,
    path: String,
    #[serde(rename = "type")]
    kind: String,
    download_url: Option<String>,
}

pub fn install_skills(ids: &[String], dest_root: &Path) -> Result<Vec<String>> {
    install_skills_with_options(ids, dest_root, InstallOptions::default())
}

#[derive(Debug, Clone, Copy)]
pub struct InstallOptions {
    pub quiet: bool,
}

/// 下载进度（stderr 单行刷新）
pub struct DownloadReporter {
    pub quiet: bool,
    pub label: String,
    pub count: usize,
}

impl DownloadReporter {
    pub fn file_downloaded(&mut self, remote_path: &str) {
        self.count += 1;
        if self.quiet {
            return;
        }
        let frame = crate::ui::spinner_frame(self.count);
        let short = remote_path.rsplit('/').next().unwrap_or(remote_path);
        let _ = write!(
            std::io::stderr(),
            "\r  {} {} · 已下载 {} 个文件 · {}",
            frame, self.label, self.count, short
        );
        let _ = std::io::stderr().flush();
    }

    pub fn finish(&self) {
        if !self.quiet && self.count > 0 {
            eprintln!();
        }
    }
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self { quiet: false }
    }
}

pub fn install_skills_with_options(
    ids: &[String],
    dest_root: &Path,
    opts: InstallOptions,
) -> Result<Vec<String>> {
    fs::create_dir_all(dest_root)?;
    let mut installed = Vec::new();
    for id in ids {
        let entry = catalog::find(id).with_context(|| format!("未知技能: {id}"))?;
        install_one(entry, dest_root, opts)?;
        installed.push(id.clone());
    }
    Ok(installed)
}

pub fn design_bundle_installed(dest: &Path) -> bool {
    dest.join(catalog::DESIGN_BUNDLE_MARKER).is_file()
        || dest.join("frontend-design/SKILL.md").is_file()
}

fn write_design_bundle_marker(dest: &Path) -> Result<()> {
    fs::write(
        dest.join(catalog::DESIGN_BUNDLE_MARKER),
        "anthropics/skills design bundle\n",
    )
    .context("无法写入技能预装标记")
}

/// 首次配置后预装常用设计技能；网络失败时不阻断配置流程。
pub fn ensure_default_design_skills(quiet: bool) -> Result<()> {
    if std::env::var("ONEMINI_SKIP_SKILL_BOOTSTRAP").ok().as_deref() == Some("1") {
        return Ok(());
    }

    let dest = user_skills_dir()?;
    if design_bundle_installed(&dest) {
        return Ok(());
    }

    if !quiet {
        println!();
        println!("{}", crate::ui::section_title("预装设计技能"));
        println!(
            "{}",
            crate::ui::dim(
                "正在从 anthropics/skills 安装 frontend-design、canvas-design 等常用技能…"
            )
        );
    }

    let ids = catalog::DEFAULT_DESIGN_SKILL_IDS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    match install_skills_with_options(&ids, &dest, InstallOptions { quiet }) {
        Ok(installed) => {
            write_design_bundle_marker(&dest)?;
            if !quiet {
                println!(
                    "{}",
                    crate::ui::success(&format!(
                        "已预装 {} 个设计技能（会话内可用 /frontend-design 等）",
                        installed.len()
                    ))
                );
                println!(
                    "{}",
                    crate::ui::dim("文档技能会在首次启动 onemini 时自动下载")
                );
            }
        }
        Err(e) => {
            if !quiet {
                println!(
                    "{}",
                    crate::ui::warn(&format!(
                        "设计技能预装跳过（可稍后运行 onemini skills install design）: {e}"
                    ))
                );
            }
        }
    }
    Ok(())
}

fn install_one(entry: &CatalogEntry, dest_root: &Path, opts: InstallOptions) -> Result<()> {
    let (owner, repo) = ANTHROPIC_REPO;
    let skill_root = format!("skills/{}", entry.id);
    let dest = dest_root.join(entry.id);

    if dest.exists() {
        fs::remove_dir_all(&dest)
            .with_context(|| format!("无法清理旧目录: {}", dest.display()))?;
    }
    fs::create_dir_all(&dest)?;

    block_on_async(download_tree(owner, repo, &skill_root, &dest, None))?;

    if !opts.quiet {
        println!(
            "==> 已安装技能 `{}` -> {}",
            entry.id,
            dest.display()
        );
        if entry.license == "proprietary" {
            println!(
                "    注意: 该技能为 Anthropic 专有/源可用许可，请阅读 {}/LICENSE.txt",
                dest.display()
            );
        }
        if dest.join("scripts").is_dir() {
            println!("    含 scripts/ 目录，执行前请安装 SKILL.md 中列出的 Python 依赖");
        }
    }
    Ok(())
}

/// 在同步上下文中执行异步下载；若已在 Tokio 运行时内则用 block_in_place，避免嵌套 Runtime。
fn block_on_async<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Runtime::new()?.block_on(future)
    }
}

pub async fn download_tree(
    owner: &str,
    repo: &str,
    remote_path: &str,
    local_dir: &Path,
    reporter: Option<Rc<RefCell<DownloadReporter>>>,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("onemini-cli")
        .build()?;
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/contents/{remote_path}?ref=main"
    );
    let items: Vec<GitHubContent> = client
        .get(&url)
        .send()
        .await
        .context("请求 GitHub API 失败（检查网络）")?
        .error_for_status()
        .context(format!("GitHub 上不存在路径: {remote_path}"))?
        .json()
        .await
        .context("解析 GitHub API 响应失败")?;

    for item in items {
        let local = local_dir.join(&item.name);
        if item.kind == "dir" {
            fs::create_dir_all(&local)?;
            Box::pin(download_tree(
                owner,
                repo,
                &item.path,
                &local,
                reporter.clone(),
            ))
            .await?;
        } else if item.kind == "file" {
            let Some(download_url) = item.download_url else {
                continue;
            };
            if let Some(r) = &reporter {
                r.borrow_mut().file_downloaded(&item.path);
            }
            let bytes = client
                .get(download_url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            fs::write(&local, &bytes)
                .with_context(|| format!("写入失败: {}", local.display()))?;
        }
    }
    Ok(())
}

pub fn user_skills_dir() -> Result<PathBuf> {
    Ok(crate::config::Config::config_dir()?.join("skills"))
}

pub fn parse_install_args(args: &[String]) -> Result<Vec<String>> {
    if args.is_empty() {
        bail!("请指定技能 id，例如: onemini skills install pdf frontend-design");
    }
    let mut ids = Vec::new();
    for arg in args {
        if arg == "docs" || arg == "documents" {
            ids.extend(
                ["docx", "pdf", "pptx", "xlsx"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        } else if arg == "design" {
            ids.extend(
                catalog::DEFAULT_DESIGN_SKILL_IDS
                    .iter()
                    .map(|s| s.to_string()),
            );
        } else {
            ids.push(arg.clone());
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}
