use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::permissions::PermissionRulesFile;

#[derive(Debug, Clone, Default)]
pub struct ManagedSettings {
    pub disable_bypass_permissions: bool,
    pub allow_managed_rules_only: bool,
    pub allow_managed_hooks_only: bool,
    pub rules: PermissionRulesFile,
    pub hook_fail_open: bool,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct ManagedFile {
    #[serde(default)]
    disable_bypass_permissions: bool,
    #[serde(default)]
    allow_managed_rules_only: bool,
    #[serde(default)]
    allow_managed_hooks_only: bool,
    #[serde(default)]
    hook_fail_open: bool,
    #[serde(default)]
    rules: PermissionRulesFile,
}

impl ManagedSettings {
    pub fn load() -> Result<Self> {
        let path = resolve_managed_path();
        if let Some(path) = path {
            if path.exists() {
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("读取托管策略失败: {}", path.display()))?;
                let file: ManagedFile = toml::from_str(&text)
                    .with_context(|| format!("解析托管策略失败: {}", path.display()))?;
                let mut rules = file.rules;
                rules.migrate_legacy();
                return Ok(Self {
                    disable_bypass_permissions: file.disable_bypass_permissions,
                    allow_managed_rules_only: file.allow_managed_rules_only,
                    allow_managed_hooks_only: file.allow_managed_hooks_only,
                    hook_fail_open: file.hook_fail_open,
                    rules,
                    source: Some(path),
                });
            }
        }
        Ok(Self::default())
    }
}

fn resolve_managed_path() -> Option<PathBuf> {
    if let Ok(p) = env::var("ONEMINI_MANAGED_SETTINGS") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    #[cfg(target_os = "macos")]
    {
        return Some(PathBuf::from(
            "/Library/Application Support/onemini/managed.toml",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        return Some(PathBuf::from("/etc/onemini/managed.toml"));
    }
    #[cfg(target_os = "windows")]
    {
        return Some(PathBuf::from(r"C:\ProgramData\onemini\managed.toml"));
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}
