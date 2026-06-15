use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub event: String,
    #[serde(default)]
    pub tool: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    #[serde(default)]
    pub hooks: Vec<HookDef>,
}

pub struct HookRunner {
    config: HookConfig,
}

impl HookRunner {
    pub fn load() -> Result<Self> {
        let path = crate::config::Config::config_dir()?.join("hooks.toml");
        let config = if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("读取 hooks 配置失败: {}", path.display()))?;
            toml::from_str(&text).unwrap_or_default()
        } else {
            HookConfig::default()
        };
        Ok(Self { config })
    }

    pub fn run_pre_tool(&self, tool: &str, args: &Value) -> Result<()> {
        self.run_matching("PreToolUse", Some(tool), args)
    }

    pub fn run_post_tool(&self, tool: &str, args: &Value, result: &str) -> Result<()> {
        let mut enriched = args.clone();
        if let Some(obj) = enriched.as_object_mut() {
            obj.insert("_result".into(), Value::String(result.to_string()));
        }
        self.run_matching("PostToolUse", Some(tool), &enriched)
    }

    pub fn run_notification(&self, message: &str) -> Result<()> {
        let args = serde_json::json!({ "message": message });
        self.run_matching("Notification", None, &args)
    }

    fn run_matching(&self, event: &str, tool: Option<&str>, args: &Value) -> Result<()> {
        for hook in &self.config.hooks {
            if hook.event != event {
                continue;
            }
            if let Some(ref t) = hook.tool {
                if tool != Some(t.as_str()) {
                    continue;
                }
            }
            self.execute_hook(&hook.command, args)?;
        }
        Ok(())
    }

    fn execute_hook(&self, command: &str, args: &Value) -> Result<()> {
        let args_json = args.to_string();
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .status()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .status()
        }
        .with_context(|| format!("执行 hook 失败: {command}"))?;
        if !status.success() {
            anyhow::bail!("hook 退出码 {:?}", status.code());
        }
        Ok(())
    }
}
