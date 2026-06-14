use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use super::{truncate_output, Tool};

const TIMEOUT_SECS: u64 = 120;
const MAX_OUTPUT: usize = 50_000;

pub struct BashTool {
    workdir: PathBuf,
}

impl BashTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "在工作目录下执行 shell 命令。超时 120 秒。"
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
            .context("缺少 command 参数")?;

        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.workdir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("启动 shell 失败")?;

        let output = timeout(Duration::from_secs(TIMEOUT_SECS), child.wait_with_output())
            .await
            .context("命令执行超时")?
            .context("等待命令结束失败")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);

        let mut result = format!("exit code: {code}\n");
        if !stdout.is_empty() {
            result.push_str("--- stdout ---\n");
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            result.push_str("--- stderr ---\n");
            result.push_str(&stderr);
        }

        Ok(truncate_output(&result, MAX_OUTPUT))
    }
}
