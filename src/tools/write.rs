use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use super::{resolve_path, Tool};

pub struct WriteTool {
    workdir: PathBuf,
}

impl WriteTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "创建新文件或覆盖已有文件。若文件已存在，应先使用 read 读取。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "文件路径" },
                "content": { "type": "string", "description": "写入的完整文件内容" }
            },
            "required": ["path", "content"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("缺少 path 参数")?;
        let content = args["content"]
            .as_str()
            .context("缺少 content 参数")?;

        let path = resolve_path(&self.workdir, path_str)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, content)
            .with_context(|| format!("写入文件失败: {}", path.display()))?;

        Ok(format!(
            "已写入 {} ({} 字节)",
            path.display(),
            content.len()
        ))
    }
}
