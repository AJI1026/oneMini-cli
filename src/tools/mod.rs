mod bash;
mod delegate;
mod edit;
mod fetch;
mod glob_tool;
mod grep;
mod list_skills;
mod read;
mod write;

use anyhow::{Context, Result};
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

/// 将用户路径解析为工作目录内的绝对路径，拒绝越界访问和符号链接攻击。
pub fn resolve_path(workdir: &std::path::Path, user_path: &str) -> Result<PathBuf> {
    let work_canon = workdir
        .canonicalize()
        .with_context(|| format!("无法解析工作目录: {}", workdir.display()))?;

    let path = PathBuf::from(user_path);
    let resolved = if path.is_absolute() {
        path
    } else {
        work_canon.join(path)
    };

    // 标准化路径（解析 . 和 ..）
    let normalized = normalize_path(&resolved);

    // 检查标准化后的路径是否还在工作目录内
    if !normalized.starts_with(&work_canon) {
        anyhow::bail!("路径越界，拒绝访问: {user_path}");
    }

    // 如果路径存在，必须通过 canonicalize 验证（防止符号链接逃逸）
    if normalized.exists() {
        let canonical = normalized
            .canonicalize()
            .with_context(|| format!("路径解析失败: {user_path}"))?;
        if !canonical.starts_with(&work_canon) {
            anyhow::bail!("路径越界（符号链接逃逸），拒绝访问: {user_path}");
        }
        // 检查路径组件中是否有符号链接指向工作目录之外
        check_symlink_attack(&canonical, &work_canon)
            .context("符号链接安全检查失败")?;
        return Ok(canonical);
    }

    // 路径不存在——验证父目录可解析且不越界
    if let Some(parent) = normalized.parent() {
        if parent.exists() {
            let parent_canon = parent
                .canonicalize()
                .with_context(|| format!("父目录解析失败: {}", parent.display()))?;
            if !parent_canon.starts_with(&work_canon) {
                anyhow::bail!("父目录越界，拒绝访问: {user_path}");
            }
            let file_name = normalized
                .file_name()
                .with_context(|| format!("路径缺少文件名: {user_path}"))?;
            return Ok(parent_canon.join(file_name));
        }
    }

    // 父目录也不存在——使用规范化的工作目录路径
    Ok(work_canon.join(
        user_path
            .trim_start_matches("./")
            .trim_start_matches("../"),
    ))
}

/// 标准化路径，解析 . 和 .. 但不依赖文件系统（防止 TOCTOU）
fn normalize_path(path: &std::path::Path) -> PathBuf {
    use std::path::Component;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                if components.pop().is_none() {
                    // .. 溢出到根目录，保留
                    components.push(component);
                }
            }
            other => components.push(other),
        }
    }
    components.iter().collect()
}

/// 检查路径的每个父目录是否包含指向工作目录外的符号链接
fn check_symlink_attack(path: &std::path::Path, workdir: &std::path::Path) -> Result<()> {
    let mut current = path.to_path_buf();
    // 从路径末端开始，检查每个祖先目录
    while let Some(parent) = current.parent() {
        if parent == workdir || Some(parent) == workdir.parent() {
            break;
        }
        if parent.is_symlink() {
            let target = std::fs::read_link(parent)
                .with_context(|| format!("读取符号链接失败: {}", parent.display()))?;
            if !target.is_absolute() {
                let absolute_target = parent.parent().unwrap_or(parent).join(&target);
                if let Ok(canon_target) = absolute_target.canonicalize() {
                    if !canon_target.starts_with(workdir) {
                        anyhow::bail!(
                            "检测到符号链接逃逸: {} -> {}（指向工作目录外）",
                            parent.display(),
                            target.display()
                        );
                    }
                }
            } else if !target.starts_with(workdir) {
                anyhow::bail!(
                    "检测到符号链接逃逸: {} -> {}（指向工作目录外）",
                    parent.display(),
                    target.display()
                );
            }
        }
        current = parent.to_path_buf();
    }
    Ok(())
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
