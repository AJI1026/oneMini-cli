use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use super::{resolve_path, truncate_output, Tool};

const MAX_LINES: usize = 2000;

pub struct ReadTool {
    workdir: PathBuf,
}

impl ReadTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "读取文件内容。支持可选的 offset（起始行，1-based）和 limit（行数）。单次最多读取 2000 行。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "相对于工作目录的文件路径" },
                "offset": { "type": "integer", "description": "起始行号（1-based）" },
                "limit": { "type": "integer", "description": "读取行数" }
            },
            "required": ["path"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("缺少 path 参数")?;
        let path = resolve_path(&self.workdir, path_str)?;

        if !path.is_file() {
            anyhow::bail!("不是文件或不存在: {path_str}");
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("读取文件失败: {}", path.display()))?;

        let offset = args["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = args["limit"].as_u64().map(|n| n as usize).unwrap_or(MAX_LINES);

        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1);
        let end = (start + limit).min(lines.len());

        if start >= lines.len() {
            return Ok(format!("文件共 {} 行，offset 超出范围", lines.len()));
        }

        let mut out = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_no = start + i + 1;
            out.push_str(&format!("{line_no:6}|{line}\n"));
        }

        if lines.len() > end {
            out.push_str(&format!(
                "\n… 文件共 {} 行，已显示 {}–{} 行",
                lines.len(),
                start + 1,
                end
            ));
        }

        Ok(truncate_output(&out, 50_000))
    }
}
