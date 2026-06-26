use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SandboxConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default = "default_true")]
    pub auto_allow_sandboxed_bash: bool,
    #[serde(default)]
    pub extra_read_paths: Vec<String>,
    #[serde(default)]
    pub extra_write_paths: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl SandboxConfig {
    pub fn effective_auto_allow(&self) -> bool {
        self.auto_allow_sandboxed_bash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxBackend {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Bubblewrap,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    SandboxExec,
    None,
}

#[derive(Debug, Clone)]
pub struct SandboxRunner {
    config: SandboxConfig,
    backend: SandboxBackend,
}

impl SandboxRunner {
    pub fn new(config: &SandboxConfig) -> Self {
        Self {
            config: config.clone(),
            backend: probe_backend(),
        }
    }

    pub fn backend(&self) -> SandboxBackend {
        self.backend
    }

    pub fn ensure_available(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.backend == SandboxBackend::None {
            anyhow::bail!(
                "沙箱已启用但当前平台无可用沙箱后端。\n\
                 Linux: 请安装 bubblewrap (bwrap)\n\
                 macOS: 需要 sandbox-exec\n\
                 Windows: 请使用 WSL 或在配置中设置 sandbox.enabled = false"
            );
        }
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn auto_allow_sandboxed_bash(&self) -> bool {
        self.config.effective_auto_allow()
    }

    pub async fn exec(&self, command: &str, workdir: &Path) -> Result<tokio::process::Child> {
        self.ensure_available()?;
        if !self.config.enabled {
            // 沙箱关闭时检查危险命令模式，防止 prompt injection 导致的 RCE
            check_dangerous_command(command)?;
            return Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(workdir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("启动 shell 失败");
        }

        match self.backend {
            SandboxBackend::Bubblewrap => self.exec_bwrap(command, workdir).await,
            SandboxBackend::SandboxExec => self.exec_sandbox_exec(command, workdir).await,
            SandboxBackend::None => {
                anyhow::bail!("沙箱不可用，已拒绝执行 bash")
            }
        }
    }

    async fn exec_bwrap(&self, command: &str, workdir: &Path) -> Result<tokio::process::Child> {
        let work = workdir
            .canonicalize()
            .with_context(|| format!("无法解析工作目录: {}", workdir.display()))?;
        let mut args = vec![
            "--die-with-parent".to_string(),
            "--unshare-all".to_string(),
            "--new-session".to_string(),
            "--ro-bind".to_string(),
            "/".to_string(),
            "/".to_string(),
            "--bind".to_string(),
            work.display().to_string(),
            work.display().to_string(),
            "--chdir".to_string(),
            work.display().to_string(),
            // bwrap --dev 创建新的最小 devtmpfs（非绑定主机 /dev），安全且兼容
            "--dev".to_string(),
            "/dev".to_string(),
            "--proc".to_string(),
            "/proc".to_string(),
            "--tmpfs".to_string(),
            "/tmp".to_string(),
        ];
        if !self.config.allow_network {
            args.push("--unshare-net".to_string());
        }
        for p in &self.config.extra_read_paths {
            if let Ok(canon) = PathBuf::from(p).canonicalize() {
                args.push("--ro-bind".to_string());
                args.push(canon.display().to_string());
                args.push(canon.display().to_string());
            }
        }
        for p in &self.config.extra_write_paths {
            if let Ok(canon) = PathBuf::from(p).canonicalize() {
                args.push("--bind".to_string());
                args.push(canon.display().to_string());
                args.push(canon.display().to_string());
            }
        }
        args.push("--".to_string());
        args.push("sh".to_string());
        args.push("-c".to_string());
        args.push(command.to_string());

        Command::new("bwrap")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("启动 bwrap 沙箱失败（请确认已安装 bubblewrap）")
    }

    async fn exec_sandbox_exec(&self, command: &str, workdir: &Path) -> Result<tokio::process::Child> {
        let work = workdir
            .canonicalize()
            .with_context(|| format!("无法解析工作目录: {}", workdir.display()))?;
        let mut profile = format!(
            "(version 1)\n(deny default)\n(allow process*)\n(allow sysctl-read)\n(allow file-read*)\n(allow file-write* (subpath \"{}\"))\n(allow file-write* (subpath \"/tmp\"))\n(allow file-write* (subpath \"/var/folders\"))\n",
            work.display()
        );
        if self.config.allow_network {
            profile.push_str("(allow network-outbound)\n");
        }
        // 使用 tempfile 创建安全临时文件（权限 0o600），避免 /tmp 下的 TOCTOU 竞争
        let mut tmp = tempfile::Builder::new()
            .prefix("onemini-sandbox-")
            .suffix(".sb")
            .tempfile()
            .context("创建 sandbox profile 临时文件失败")?;
        tmp.write_all(profile.as_bytes())
            .context("写入 sandbox profile 失败")?;
        let profile_path = tmp.path().to_path_buf();

        let child = Command::new("sandbox-exec")
            .arg("-f")
            .arg(&profile_path)
            .arg("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&work)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("启动 sandbox-exec 失败")?;

        // tempfile 超出作用域后自动删除
        drop(tmp);
        Ok(child)
    }
}

/// 高危命令模式——即使沙箱关闭也必须拦截
const DANGEROUS_COMMAND_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "mkfs.",
    "dd if=/dev/zero of=",
    ":(){ :|:& };:",
    "> /dev/sda",
    "| sh",
    "| bash",
    "| zsh",
    "curl ",
    "wget ",
    "chmod -R 000",
    "chown -R ",
];

fn check_dangerous_command(command: &str) -> Result<()> {
    let lower = command.to_lowercase();
    for pattern in DANGEROUS_COMMAND_PATTERNS {
        if lower.contains(pattern) {
            bail!(
                "[安全拦截] 沙箱关闭状态下禁止执行危险命令（匹配模式: {}）",
                pattern
            );
        }
    }
    Ok(())
}

pub fn probe_backend() -> SandboxBackend {
    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("bwrap")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return SandboxBackend::Bubblewrap;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if std::process::Command::new("sandbox-exec")
            .arg("-h")
            .output()
            .is_ok()
        {
            return SandboxBackend::SandboxExec;
        }
    }
    SandboxBackend::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_variant() {
        let _ = probe_backend();
    }

    #[test]
    fn detects_dangerous_commands() {
        assert!(check_dangerous_command("rm -rf /").is_err());
        assert!(check_dangerous_command("ls -la").is_ok());
        assert!(check_dangerous_command("curl http://evil.sh").is_err());
        assert!(check_dangerous_command("echo hello").is_ok());
    }
}
