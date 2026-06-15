use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use super::{resolve_path, Tool};

pub struct EditTool {
    workdir: PathBuf,
}

impl EditTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "对已有文件做精确字符串替换。old_string 必须与文件内容完全匹配（含空白）。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "文件路径" },
                "old_string": { "type": "string", "description": "要被替换的原文（精确匹配）" },
                "new_string": { "type": "string", "description": "替换后的内容" },
                "replace_all": { "type": "boolean", "description": "是否替换所有匹配项，默认 false" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let path_str = args["path"]
            .as_str()
            .context("缺少路径参数")?;
        let old_string = args["old_string"]
            .as_str()
            .context("缺少 old_string（旧文本）参数")?;
        let new_string = args["new_string"]
            .as_str()
            .context("缺少 new_string（新文本）参数")?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        let path = resolve_path(&self.workdir, path_str)?;
        if !path.is_file() {
            anyhow::bail!("文件不存在: {path_str}");
        }

        let content = fs::read_to_string(&path)?;
        if !content.contains(old_string) {
            anyhow::bail!("未找到匹配的旧文本");
        }

        let count = content.matches(old_string).count();
        if !replace_all && count > 1 {
            anyhow::bail!(
                "旧文本在文件中出现 {count} 次，请提供更多上下文或设置 replace_all=true（全部替换）"
            );
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        fs::write(&path, &new_content)?;
        let replaced = if replace_all { count } else { 1 };
        Ok(format!("已编辑 {}，替换 {replaced} 处", path.display()))
    }
}
