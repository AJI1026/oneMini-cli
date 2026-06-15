use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct GitManager {
    workdir: PathBuf,
    last_checkpoint: Option<String>,
}

impl GitManager {
    pub fn new(workdir: PathBuf) -> Self {
        Self {
            workdir,
            last_checkpoint: None,
        }
    }

    pub fn is_repo(&self) -> bool {
        self.workdir.join(".git").exists()
            || Command::new("git")
                .args(["rev-parse", "--git-dir"])
                .current_dir(&self.workdir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    pub fn last_checkpoint(&self) -> Option<&str> {
        self.last_checkpoint.as_deref()
    }

    pub fn create_checkpoint(&mut self, message: &str) -> Result<String> {
        if !self.is_repo() {
            anyhow::bail!("不是 git 仓库");
        }
        let status = self.git_output(&["status", "--porcelain"])?;
        if status.trim().is_empty() {
            let hash = self.git_output(&["rev-parse", "HEAD"])?;
            self.last_checkpoint = Some(hash.trim().to_string());
            return Ok(self.last_checkpoint.clone().unwrap());
        }
        self.git_run(&["add", "-A"])?;
        self.git_run(&["commit", "-m", message, "--no-verify"])?;
        let hash = self.git_output(&["rev-parse", "HEAD"])?;
        let hash = hash.trim().to_string();
        self.last_checkpoint = Some(hash.clone());
        Ok(hash)
    }

    pub fn diff_preview(&self, paths: &[&str]) -> Result<String> {
        if !self.is_repo() {
            return Ok(String::new());
        }
        let mut args = vec!["diff", "--stat"];
        args.extend(paths);
        Ok(self.git_output(&args).unwrap_or_default())
    }

    pub fn diff_staged_preview(&self) -> Result<String> {
        if !self.is_repo() {
            return Ok(String::new());
        }
        Ok(self
            .git_output(&["diff", "--cached", "--stat"])
            .unwrap_or_default())
    }

    pub fn rollback_checkpoint(&self) -> Result<()> {
        let hash = self
            .last_checkpoint
            .as_deref()
            .context("没有可回滚的检查点")?;
        self.git_run(&["reset", "--hard", hash])?;
        Ok(())
    }

    pub fn suggest_commit_message(changed_files: &[String]) -> String {
        if changed_files.is_empty() {
            return "chore: onemini checkpoint".into();
        }
        if changed_files.len() == 1 {
            return format!("chore: update {}", changed_files[0]);
        }
        format!(
            "chore: update {} files ({})",
            changed_files.len(),
            changed_files
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn git_run(&self, args: &[&str]) -> Result<()> {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.workdir)
            .status()
            .with_context(|| format!("git {} 失败", args.join(" ")))?;
        if !status.success() {
            anyhow::bail!("git {} 退出码 {:?}", args.join(" "), status.code());
        }
        Ok(())
    }

    fn git_output(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.workdir)
            .output()
            .with_context(|| format!("git {} 失败", args.join(" ")))?;
        if !out.status.success() {
            anyhow::bail!(
                "git {} 失败: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}
