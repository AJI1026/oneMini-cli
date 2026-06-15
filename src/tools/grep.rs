use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use walkdir::WalkDir;

use super::{resolve_path, truncate_output, Tool};

const MAX_MATCHES: usize = 100;

pub struct GrepTool {
    workdir: PathBuf,
}

impl GrepTool {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }

    async fn try_ripgrep(
        &self,
        pattern: &str,
        search_path: &PathBuf,
        glob_filter: Option<&str>,
    ) -> Result<String> {
        use tokio::process::Command;
        let mut cmd = Command::new("rg");
        cmd.arg("--line-number")
            .arg("--no-heading")
            .arg("--color=never")
            .arg(pattern)
            .current_dir(&self.workdir);
        if search_path.is_file() {
            cmd.arg(search_path);
        } else {
            cmd.arg(search_path);
        }
        if let Some(g) = glob_filter {
            cmd.args(["--glob", g]);
        }
        let out = cmd.output().await?;
        if !out.status.success() && out.status.code() != Some(1) {
            anyhow::bail!("rg 失败");
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if text.is_empty() {
            return Ok(String::new());
        }
        Ok(truncate_output(&text, 30_000))
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "在文件中搜索正则表达式模式。返回匹配的文件路径、行号和行内容。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "正则表达式" },
                "path": { "type": "string", "description": "搜索路径（文件或目录），默认工作目录根" },
                "glob": { "type": "string", "description": "文件名 glob 过滤，如 *.rs" }
            },
            "required": ["pattern"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        false
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let pattern_str = args["pattern"]
            .as_str()
            .context("缺少 pattern（模式）参数")?;

        let search_path = args["path"]
            .as_str()
            .map(|p| resolve_path(&self.workdir, p))
            .transpose()?
            .unwrap_or_else(|| self.workdir.clone());

        let glob_filter = args["glob"].as_str();

        if let Ok(out) = self.try_ripgrep(pattern_str, &search_path, glob_filter).await {
            if !out.is_empty() {
                return Ok(out);
            }
        }

        let re = Regex::new(pattern_str).context("无效的正则表达式")?;

        let mut matches = Vec::new();
        let walker = if search_path.is_file() {
            WalkDir::new(&search_path).max_depth(1)
        } else {
            WalkDir::new(&search_path)
        };

        'outer: for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(g) = glob_filter {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !glob::Pattern::new(g)
                    .map(|p| p.matches(name))
                    .unwrap_or(true)
                {
                    continue;
                }
            }
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    let rel = path
                        .strip_prefix(&self.workdir)
                        .unwrap_or(path)
                        .display();
                    matches.push(format!("{}:{}:{}", rel, i + 1, line));
                    if matches.len() >= MAX_MATCHES {
                        matches.push("… 匹配数已达上限".into());
                        break 'outer;
                    }
                }
            }
        }

        if matches.is_empty() {
            Ok("未找到匹配".into())
        } else {
            Ok(truncate_output(&matches.join("\n"), 30_000))
        }
    }
}
