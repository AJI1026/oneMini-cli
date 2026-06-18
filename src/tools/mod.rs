mod bash;
mod delegate;
mod edit;
mod fetch;
mod glob_tool;
mod grep;
mod list_skills;
mod read;
mod write;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use crate::llm::ToolDefinition;
use crate::permissions::PermissionMode;

pub use bash::BashTool;
pub use delegate::DelegateTool;
pub use edit::EditTool;
pub use fetch::FetchTool;
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use list_skills::ListSkillsTool;
pub use read::ReadTool;
pub use write::WriteTool;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn requires_approval(&self, _args: &Value) -> bool;
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: crate::llm::FunctionDefinition {
                name: self.name().into(),
                description: self.description().into(),
                parameters: self.parameters_schema(),
            },
        }
    }
    async fn execute(&self, args: Value) -> Result<String>;
}

pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new(workdir: PathBuf, sandbox: crate::sandbox::SandboxRunner) -> Self {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadTool::new(workdir.clone())),
            Arc::new(ListSkillsTool::new(workdir.clone())),
            Arc::new(FetchTool::new()),
            Arc::new(WriteTool::new(workdir.clone())),
            Arc::new(EditTool::new(workdir.clone())),
            Arc::new(GrepTool::new(workdir.clone())),
            Arc::new(GlobTool::new(workdir.clone())),
            Arc::new(BashTool::new(workdir.clone(), sandbox)),
        ];
        Self { tools }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn extend(&mut self, extra: &[Arc<dyn Tool>]) {
        self.tools.extend(extra.iter().cloned());
    }

    pub fn definitions_for_mode(&self, mode: PermissionMode) -> Vec<ToolDefinition> {
        if mode.is_readonly() {
            return self
                .tools
                .iter()
                .filter(|t| matches!(t.name(), "read" | "grep" | "glob" | "fetch" | "list_skills"))
                .map(|t| t.definition())
                .collect();
        }
        self.tools.iter().map(|t| t.definition()).collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }
}

/// 将用户路径解析为工作目录内的绝对路径，拒绝越界访问。
pub fn resolve_path(workdir: &std::path::Path, user_path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(user_path);
    let resolved = if path.is_absolute() {
        path
    } else {
        workdir.join(path)
    };
    let canonical = if resolved.exists() {
        resolved.canonicalize().unwrap_or(resolved.clone())
    } else if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let parent_canon = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
            parent_canon.join(resolved.file_name().unwrap_or_default())
        } else {
            workdir
                .canonicalize()
                .unwrap_or_else(|_| workdir.to_path_buf())
                .join(user_path.trim_start_matches("./"))
        }
    } else {
        workdir
            .canonicalize()
            .unwrap_or_else(|_| workdir.to_path_buf())
            .join(user_path.trim_start_matches("./"))
    };

    let work_canon = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());

    if !canonical.starts_with(&work_canon) {
        anyhow::bail!("路径越界，拒绝访问: {user_path}");
    }
    Ok(canonical)
}

pub fn truncate_output(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!(
        "{truncated}\n\n… [输出已截断，共 {} 字符，显示前 {max_chars} 字符]",
        text.chars().count()
    )
}
