use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;

use super::Tool;

pub struct DelegateTool {
    workdir: PathBuf,
    use_worktree: bool,
}

impl DelegateTool {
    pub fn new(workdir: PathBuf, use_worktree: bool) -> Self {
        Self {
            workdir,
            use_worktree,
        }
    }
}

#[async_trait]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "将子任务委派给独立只读 Agent（可选 git worktree 隔离）。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "子任务描述" },
                "context": { "type": "string", "description": "可选额外上下文" },
                "isolated": { "type": "boolean", "description": "是否在 git worktree 中隔离执行" }
            },
            "required": ["task"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let task = args["task"]
            .as_str()
            .context("缺少 task（任务）参数")?;
        let context = args["context"].as_str().unwrap_or("");
        let isolated = args["isolated"].as_bool().unwrap_or(self.use_worktree);
        let prompt = if context.is_empty() {
            task.to_string()
        } else {
            format!("{task}\n\n上下文:\n{context}")
        };
        if isolated {
            crate::subagent::run_subtask_isolated(self.workdir.clone(), &prompt).await
        } else {
            crate::subagent::run_subtask(self.workdir.clone(), &prompt).await
        }
    }
}
