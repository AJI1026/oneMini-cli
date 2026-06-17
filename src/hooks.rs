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

#[derive(Debug, Clone)]
pub enum HookOutcome {
    Continue,
    Deny(String),
    Modified(Value),
}

pub struct HookRunner {
    user_config: HookConfig,
    managed_hooks_only: bool,
    fail_open: bool,
}

impl HookRunner {
    pub fn load(managed: &crate::managed::ManagedSettings) -> Result<Self> {
        let path = crate::config::Config::config_dir()?.join("hooks.toml");
        let user_config = if path.exists() && !managed.allow_managed_hooks_only {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("读取 hooks 配置失败: {}", path.display()))?;
            toml::from_str(&text).unwrap_or_default()
        } else {
            HookConfig::default()
        };
        Ok(Self {
            user_config,
            managed_hooks_only: managed.allow_managed_hooks_only,
            fail_open: managed.hook_fail_open,
        })
    }

    pub fn run_pre_tool(&self, tool: &str, args: &Value) -> Result<HookOutcome> {
        self.run_matching("PreToolUse", Some(tool), args)
    }

    pub fn run_post_tool(&self, tool: &str, args: &Value, result: &str) -> Result<()> {
        let mut enriched = args.clone();
        if let Some(obj) = enriched.as_object_mut() {
            obj.insert("_result".into(), Value::String(result.to_string()));
        }
        match self.run_matching("PostToolUse", Some(tool), &enriched) {
            Ok(HookOutcome::Continue) => Ok(()),
            Ok(HookOutcome::Deny(reason)) => anyhow::bail!("PostToolUse hook 拒绝: {reason}"),
            Ok(HookOutcome::Modified(_)) => Ok(()),
            Err(e) => {
                if self.fail_open {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn run_notification(&self, message: &str) -> Result<()> {
        let args = serde_json::json!({ "message": message });
        let _ = self.run_matching("Notification", None, &args);
        Ok(())
    }

    fn run_matching(&self, event: &str, tool: Option<&str>, args: &Value) -> Result<HookOutcome> {
        let mut current_args = args.clone();
        for hook in &self.user_config.hooks {
            if hook.event != event {
                continue;
            }
            if let Some(ref t) = hook.tool {
                if tool != Some(t.as_str()) {
                    continue;
                }
            }
            match self.execute_hook(&hook.command, &current_args)? {
                HookOutcome::Deny(reason) => return Ok(HookOutcome::Deny(reason)),
                HookOutcome::Modified(v) => current_args = v,
                HookOutcome::Continue => {}
            }
        }
        if current_args != *args {
            Ok(HookOutcome::Modified(current_args))
        } else {
            Ok(HookOutcome::Continue)
        }
    }

    fn execute_hook(&self, command: &str, args: &Value) -> Result<HookOutcome> {
        let args_json = args.to_string();
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", command])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .output()
        }
        .with_context(|| format!("执行 hook 失败: {command}"))?;

        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if code == 2 {
            let reason = if stdout.is_empty() {
                format!("hook 退出码 2: {command}")
            } else {
                stdout.clone()
            };
            return Ok(HookOutcome::Deny(reason));
        }

        if code != 0 {
            if self.fail_open {
                return Ok(HookOutcome::Continue);
            }
            anyhow::bail!("hook 退出码 {code}: {}", String::from_utf8_lossy(&output.stderr));
        }

        if let Ok(v) = serde_json::from_str::<Value>(&stdout) {
            if let Some(decision) = v.get("decision").and_then(|d| d.as_str()) {
                return match decision {
                    "deny" => Ok(HookOutcome::Deny(
                        v.get("reason")
                            .and_then(|r| r.as_str())
                            .unwrap_or("hook 拒绝")
                            .into(),
                    )),
                    "allow" => Ok(HookOutcome::Continue),
                    "ask" => Ok(HookOutcome::Continue),
                    _ => Ok(HookOutcome::Continue),
                };
            }
            if let Some(modified) = v.get("modified_input") {
                return Ok(HookOutcome::Modified(modified.clone()));
            }
        }

        Ok(HookOutcome::Continue)
    }
}
