use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::SystemTime;

use super::{resolve_path, truncate_output, Tool};

const MAX_RESULTS: usize = 200;

pub struct GlobTool {
    workdir: PathBuf,
}

impl GlobTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "按 glob 模式查找文件，按修改时间倒序返回。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "glob 模式，如 **/*.rs" },
                "path": { "type": "string", "description": "搜索根目录，默认工作目录" }
            },
            "required": ["pattern"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let pattern = args["pattern"]
            .as_str()
            .context("缺少 pattern（模式）参数")?;
        let base = args["path"]
            .as_str()
            .map(|p| resolve_path(&self.workdir, p))
            .transpose()?
            .unwrap_or_else(|| self.workdir.clone());

        let full_pattern = if pattern.contains('/') || pattern.starts_with("**") {
            base.join(pattern)
        } else {
            base.join("**").join(pattern)
        };

        let pattern_str = full_pattern.to_string_lossy();
        let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in glob::glob(&pattern_str).context("无效的 glob 模式")? {
            let path = entry?;
            if path.is_file() {
                let mtime = path
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                entries.push((path, mtime));
            }
        }

        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(MAX_RESULTS);

        if entries.is_empty() {
            return Ok("未找到匹配文件".into());
        }

        let lines: Vec<String> = entries
            .iter()
            .map(|(p, _)| {
                p.strip_prefix(&self.workdir)
                    .unwrap_or(p)
                    .display()
                    .to_string()
            })
            .collect();

        Ok(truncate_output(&lines.join("\n"), 20_000))
    }
}
