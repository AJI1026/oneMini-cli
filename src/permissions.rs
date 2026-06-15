use anyhow::{Context, Result};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionRules {
    #[serde(default)]
    pub always_allow: Vec<String>,
    #[serde(default)]
    pub always_deny: Vec<String>,
    #[serde(default)]
    pub bash_allow_patterns: Vec<String>,
    #[serde(default)]
    pub bash_deny_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny(String),
    Ask(String),
}

pub struct PermissionManager {
    rules: PermissionRules,
    path: PathBuf,
}

impl PermissionManager {
    pub fn load() -> Result<Self> {
        let path = crate::config::Config::config_dir()?.join("permissions.toml");
        let rules = if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("读取权限配置失败: {}", path.display()))?;
            toml::from_str(&text).unwrap_or_default()
        } else {
            PermissionRules {
                always_allow: vec![
                    "read".into(),
                    "grep".into(),
                    "glob".into(),
                ],
                ..Default::default()
            }
        };
        Ok(Self { rules, path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn evaluate(&self, tool: &str, detail: &str) -> PermissionDecision {
        if self.rules.always_deny.iter().any(|t| t == tool) {
            return PermissionDecision::Deny(format!("工具 {tool} 已被始终拒绝（always_deny）"));
        }
        if self.rules.always_allow.iter().any(|t| t == tool) {
            return PermissionDecision::Allow;
        }

        if tool == "bash" {
            for pat in &self.rules.bash_deny_patterns {
                if pattern_match(pat, detail) {
                    return PermissionDecision::Deny(format!("命令匹配拒绝规则: {pat}"));
                }
            }
            for pat in &self.rules.bash_allow_patterns {
                if pattern_match(pat, detail) {
                    return PermissionDecision::Allow;
                }
            }
        }

        PermissionDecision::Ask(String::new())
    }
}

fn pattern_match(pattern: &str, text: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches(text))
        .unwrap_or_else(|_| text.contains(pattern))
}
