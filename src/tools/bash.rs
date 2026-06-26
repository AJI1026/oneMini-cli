use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

use super::{truncate_output, Tool};
use crate::sandbox::SandboxRunner;

const TIMEOUT_SECS: u64 = 120;
const PREVIEW_CHARS: usize = 1200;

pub struct BashTool {
    workdir: PathBuf,
    sandbox: SandboxRunner,
}

impl BashTool {
    pub fn new(workdir: PathBuf, sandbox: SandboxRunner) -> Self {
        Self { workdir, sandbox }
    }
}

#[derive(Debug, serde::Serialize)]
struct BashResult {
    success: bool,
    exit_code: i32,
    timed_out: bool,
    command: String,
    stdout_preview: String,
    stderr_preview: String,
    failure_reason: Option<String>,
    retry_hint: Option<String>,
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "在工作目录下执行 shell 命令（OS 沙箱内）。返回结构化 JSON。超时 120 秒。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "要执行的 shell 命令" }
            },
            "required": ["command"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let command = args["command"]
            .as_str()
            .context("缺少 command（命令）参数")?;

        self.sandbox.ensure_available()?;

        let child = self
            .sandbox
            .exec(command, &self.workdir)
            .await
            .context("启动沙箱 shell 失败")?;

        let timed_out = match timeout(Duration::from_secs(TIMEOUT_SECS), child.wait_with_output()).await
        {
            Ok(result) => {
                let output = result.context("等待命令结束失败")?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                let (failure_reason, retry_hint) = if success {
                    (None, None)
                } else {
                    (
                        Some(classify_failure(code, &stderr, &stdout)),
                        Some(retry_hint_for(command, code, &stderr)),
                    )
                };

                let result = BashResult {
                    success,
                    exit_code: code,
                    timed_out: false,
                    command: command.to_string(),
                    stdout_preview: preview(&stdout),
                    stderr_preview: preview(&stderr),
                    failure_reason,
                    retry_hint,
                };
                return Ok(serde_json::to_string_pretty(&result)?);
            }
            Err(_) => BashResult {
                success: false,
                exit_code: -1,
                timed_out: true,
                command: command.to_string(),
                stdout_preview: String::new(),
                stderr_preview: String::new(),
                failure_reason: Some(timeout_failure_message(command)),
                retry_hint: Some(timeout_retry_hint(command)),
            },
        };

        Ok(serde_json::to_string_pretty(&timed_out)?)
    }
}

fn preview(text: &str) -> String {
    // 先剥离 ANSI 转义序列防止终端注入攻击，再截断
    let clean = crate::ui::strip_ansi(text);
    truncate_output(&clean, PREVIEW_CHARS)
}

fn timeout_failure_message(command: &str) -> String {
    let lower = command.to_lowercase();
    if is_gui_blocking_command(&lower) {
        format!("命令执行超时（>{TIMEOUT_SECS}s）：可能因缺少 GUI 环境阻塞（如 plt.show()）")
    } else {
        format!("命令执行超时（>{TIMEOUT_SECS}s）")
    }
}

fn timeout_retry_hint(command: &str) -> String {
    let lower = command.to_lowercase();
    if is_gui_blocking_command(&lower) {
        "将 plt.show() 改为 plt.savefig()，或设置 matplotlib 使用 Agg 后端后重试".into()
    } else {
        "缩小命令范围或拆分步骤后重试".into()
    }
}

fn is_gui_blocking_command(lower: &str) -> bool {
    lower.contains("plt.show")
        || lower.contains(".show()")
        || (lower.contains("matplotlib") && lower.contains("show"))
}

fn classify_failure(code: i32, stderr: &str, stdout: &str) -> String {
    let err = if !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    if err.is_empty() {
        format!("命令以非 0 退出（退出码: {code}）")
    } else {
        format!("退出码 {code}: {}", err.chars().take(240).collect::<String>())
    }
}

fn retry_hint_for(command: &str, code: i32, stderr: &str) -> String {
    let lower = format!("{command} {stderr}").to_lowercase();
    if lower.contains("not found") || lower.contains("no such file") {
        return "检查命令是否存在、PATH 是否正确、工作目录是否正确".into();
    }
    if lower.contains("permission denied") {
        return "检查文件权限或使用合适用户执行".into();
    }
    if lower.contains("could not compile") || lower.contains("error:") {
        return "先修复编译错误，再重新运行 cargo build/test".into();
    }
    if code == 127 {
        return "命令未找到，确认依赖已安装且命令拼写正确".into();
    }
    "根据 stderr 定位根因后，做最小修复并重试同一命令".into()
}
