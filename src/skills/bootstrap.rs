//! 启动时自动下载文档技能脚本与 Python 依赖。

use anyhow::Result;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use super::{bundled_skills_root, BUNDLED_DOC_SKILL_IDS};
use super::install::DownloadReporter;

const DOC_SKILLS_MARKER: &str = ".onemini-doc-skills-v1";
const DOC_PYDEPS_MARKER: &str = ".onemini-doc-pydeps-v1";
const ONEMINI_REPO: (&str, &str) = ("AJI1026", "OneMini-CLI");

const DOC_SKILL_REMOTE_PATHS: &[&str] = &[
    "skills/shared",
    "skills/pdf",
    "skills/docx",
    "skills/pptx",
    "skills/xlsx",
];

/// 用户数据目录中的技能根（install.sh / 启动下载写入此处）
pub fn document_skills_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| {
            crate::config::Config::config_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
        })
        .join("onemini")
        .join("skills")
}

pub fn document_skills_ready(root: &Path) -> bool {
    root.join("shared/office/unpack.py").is_file()
        && BUNDLED_DOC_SKILL_IDS
            .iter()
            .all(|id| root.join(id).join("SKILL.md").is_file())
}

/// 启动时确保文档技能脚本已就绪；`interactive` 为 true 时显示提示与进度。
pub fn ensure_document_skills(interactive: bool) -> Result<()> {
    if std::env::var("ONEMINI_SKIP_SKILL_BOOTSTRAP").ok().as_deref() == Some("1") {
        return Ok(());
    }

    if let Some(root) = bundled_skills_root() {
        if document_skills_ready(&root) {
            return Ok(());
        }
    }

    let dest = document_skills_data_dir();
    if dest.join(DOC_SKILLS_MARKER).is_file() && document_skills_ready(&dest) {
        return ensure_python_deps(&dest, interactive);
    }

    if interactive {
        println!();
        println!("{}", crate::ui::section_title("文档技能"));
        println!(
            "{}",
            crate::ui::dim(
                "正在下载 pdf / docx / pptx / xlsx 技能脚本（首次启动或升级后）…"
            )
        );
    }

    fs::create_dir_all(&dest)?;
    let reporter = Rc::new(RefCell::new(DownloadReporter {
        quiet: !interactive,
        label: "文档技能".to_string(),
        count: 0,
    }));

    let rt = tokio::runtime::Runtime::new()?;
    let (owner, repo) = ONEMINI_REPO;
    let total = DOC_SKILL_REMOTE_PATHS.len();

    for (i, remote) in DOC_SKILL_REMOTE_PATHS.iter().enumerate() {
        if interactive {
            let name = remote.strip_prefix("skills/").unwrap_or(remote);
            println!(
                "{}",
                crate::ui::dim(&format!("包 {}/{}: {name}", i + 1, total))
            );
        }
        let rel = remote.strip_prefix("skills/").unwrap_or(remote);
        let local = dest.join(rel);
        rt.block_on(super::install::download_tree(
            owner,
            repo,
            remote,
            &local,
            Some(reporter.clone()),
        ))?;
    }
    let file_count = reporter.borrow().count;
    reporter.borrow().finish();

    if let Some(req_src) = bundled_skills_root().map(|r| r.join("requirements-docs.txt")) {
        if req_src.is_file() {
            fs::copy(&req_src, dest.join("requirements-docs.txt"))?;
        }
    }
    if !dest.join("requirements-docs.txt").is_file() {
        fs::write(
            dest.join("requirements-docs.txt"),
            include_str!("../../skills/requirements-docs.txt"),
        )?;
    }

    fs::write(
        dest.join(DOC_SKILLS_MARKER),
        "onemini document skills bundle v1\n",
    )?;

    if interactive {
        println!(
            "{}",
            crate::ui::success(&format!(
                "已下载 {} 个文件，文档技能脚本已就绪",
                file_count
            ))
        );
    }

    ensure_python_deps(&dest, interactive)
}

fn ensure_python_deps(skills_root: &Path, interactive: bool) -> Result<()> {
    if std::env::var("ONEMINI_SKIP_PYTHON_DEPS").ok().as_deref() == Some("1") {
        return Ok(());
    }

    let marker = skills_root.join(DOC_PYDEPS_MARKER);
    if marker.is_file() {
        return Ok(());
    }

    let req = skills_root.join("requirements-docs.txt");
    if !req.is_file() {
        return Ok(());
    }

    let python = match which_python() {
        Some(p) => p,
        None => {
            if interactive {
                println!(
                    "{}",
                    crate::ui::warn(
                        "未找到 python3，跳过文档技能 Python 依赖安装（可稍后手动 pip install -r requirements-docs.txt）"
                    )
                );
            }
            return Ok(());
        }
    };

    if interactive {
        println!(
            "{}",
            crate::ui::dim("正在安装文档技能 Python 依赖（pip）…")
        );
    }

    let mut cmd = Command::new(&python);
    cmd.args([
        "-m",
        "pip",
        "install",
        "-r",
        req.to_str().unwrap_or("requirements-docs.txt"),
        "--disable-pip-version-check",
    ]);
    if interactive {
        cmd.arg("--progress-bar").arg("on");
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
    } else {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }

    match cmd.status() {
        Ok(s) if s.success() => {
            fs::write(&marker, "ok\n")?;
            if interactive {
                println!("{}", crate::ui::success("Python 依赖安装完成"));
            }
        }
        Ok(s) => {
            if interactive {
                println!(
                    "{}",
                    crate::ui::warn(&format!(
                        "pip 安装退出码 {}，可稍后手动: pip install -r {}",
                        s.code().unwrap_or(-1),
                        req.display()
                    ))
                );
            }
        }
        Err(e) => {
            if interactive {
                println!(
                    "{}",
                    crate::ui::warn(&format!("无法运行 pip: {e}"))
                );
            }
        }
    }
    Ok(())
}

fn which_python() -> Option<PathBuf> {
    for name in ["python3", "python"] {
        if let Ok(out) = Command::new(name).arg("--version").output() {
            if out.status.success() {
                return Some(PathBuf::from(name));
            }
        }
    }
    None
}
