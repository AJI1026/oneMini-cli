use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

pub struct GitWorktree {
    pub path: PathBuf,
    pub branch: String,
    repo_root: PathBuf,
}

impl GitWorktree {
    pub fn create(repo_root: &Path, prefix: &str) -> Result<Self> {
        let root = repo_root
            .canonicalize()
            .unwrap_or_else(|_| repo_root.to_path_buf());
        let id = Uuid::new_v4().to_string();
        let short = &id[..8];
        let branch = format!("onemini/{prefix}-{short}");
        let worktrees_dir = root
            .parent()
            .map(|p| p.join(".onemini-worktrees"))
            .unwrap_or_else(|| root.join("../.onemini-worktrees"));
        std::fs::create_dir_all(&worktrees_dir)?;
        let wt_path = worktrees_dir.join(short);

        let status = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                wt_path.to_str().context("路径无效")?,
            ])
            .current_dir(&root)
            .status()
            .context("git worktree add 失败")?;
        if !status.success() {
            anyhow::bail!("git worktree add 失败，退出码 {:?}", status.code());
        }

        Ok(Self {
            path: wt_path,
            branch,
            repo_root: root,
        })
    }

    pub fn remove(self) -> Result<()> {
        let _ = Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                self.path.to_str().context("路径无效")?,
            ])
            .current_dir(&self.repo_root)
            .status();
        let _ = Command::new("git")
            .args(["branch", "-D", &self.branch])
            .current_dir(&self.repo_root)
            .status();
        Ok(())
    }
}
