use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::process::{Command, Output};
use std::sync::mpsc;
use std::time::Duration;

const HOOK_TIMEOUT_SECS: u64 = 30;

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
    Ask(String),
    Modified(Value),
}

pub struct HookRunner {
    hooks: Vec<HookDef>,
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

        let mut hooks = managed.hooks.hooks.clone();
        if !managed.allow_managed_hooks_only {
            hooks.extend(user_config.hooks);
        }

        Ok(Self {
            hooks,
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
            Ok(HookOutcome::Continue) | Ok(HookOutcome::Ask(_)) => Ok(()),
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
        for hook in &self.hooks {
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
                HookOutcome::Ask(reason) => return Ok(HookOutcome::Ask(reason)),
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
        // 验证 hook 命令路径：仅限于 ~/.onemini/hooks/ 目录下的脚本
        validate_hook_command(command)?;
        let args_json = args.to_string();
        match run_command_with_timeout(command, &args_json, HOOK_TIMEOUT_SECS) {
            Ok(output) => parse_hook_output(command, &output, self.fail_open),
            Err(e) => {
                if self.fail_open {
                    Ok(HookOutcome::Continue)
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// 验证 hook 命令路径安全性
fn validate_hook_command(command: &str) -> Result<()> {
    // 允许系统命令（仅白名单）
    const ALLOWED_SYSTEM_COMMANDS: &[&str] = &[
        "grep", "awk", "sed", "cat", "echo", "printf", "test", "[",
        "python3", "python", "node", "deno", "bash", "sh", "zsh",
        "jq", "yq", "git",
    ];

    let trimmed = command.trim();
    // 尝试解析命令的第一部分（路径或命令名）
    let first_part = trimmed.split_whitespace().next().unwrap_or(trimmed);

    // 如果是相对/绝对路径，只能执行 hooks 目录下的脚本
    if first_part.contains('/') || first_part.contains('\\') {
        let config_dir = crate::config::Config::config_dir()?;
        let hooks_dir = config_dir.join("hooks");
        let canon_hooks = hooks_dir.canonicalize().with_context(|| {
            format!("hooks 目录不存在: {}", hooks_dir.display())
        })?;
        if let Ok(cmd_path) = std::path::PathBuf::from(first_part).canonicalize() {
            if cmd_path.starts_with(&canon_hooks) {
                return Ok(());
            }
        }
        anyhow::bail!(
            "[安全拦截] hook 命令路径不在 ~/.onemini/hooks/ 目录下: {trimmed}"
        );
    }

    // 纯命令名——检查白名单
    let cmd_name = first_part;
    if !ALLOWED_SYSTEM_COMMANDS.contains(&cmd_name) {
        anyhow::bail!(
            "[安全拦截] hook 命令不在白名单中: {cmd_name}（允许: {}）",
            ALLOWED_SYSTEM_COMMANDS.join(", ")
        );
    }
    Ok(())
}

fn run_command_with_timeout(
    command: &str,
    args_json: &str,
    timeout_secs: u64,
) -> Result<Output> {
    let (tx, rx) = mpsc::channel();
    let command_owned = command.to_string();
    let command_for_err = command_owned.clone();
    let args_json = args_json.to_string();
    std::thread::spawn(move || {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &command_owned])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", &command_owned])
                .env("ONEMINI_HOOK_ARGS", &args_json)
                .output()
        };
        let _ = tx.send(output);
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e).with_context(|| format!("执行 hook 失败: {command_for_err}")),
        Err(_) => Err(anyhow::anyhow!(
            "hook 执行超时（>{timeout_secs}s）: {command_for_err}"
        )),
    }
}

pub(crate) fn parse_hook_output(
    command: &str,
    output: &Output,
    fail_open: bool,
) -> Result<HookOutcome> {
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
        if fail_open {
            return Ok(HookOutcome::Continue);
        }
        anyhow::bail!(
            "hook 退出码 {code}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if let Some(outcome) = parse_hook_stdout_json(&stdout) {
        return Ok(outcome);
    }

    Ok(HookOutcome::Continue)
}

pub(crate) fn parse_hook_stdout_json(stdout: &str) -> Option<HookOutcome> {
    let v: Value = serde_json::from_str(stdout).ok()?;
    if let Some(decision) = v.get("decision").and_then(|d| d.as_str()) {
        return Some(match decision {
            "deny" => HookOutcome::Deny(
                v.get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("hook 拒绝")
                    .into(),
            ),
            "ask" => HookOutcome::Ask(
                v.get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("hook 要求确认")
                    .into(),
            ),
            "allow" => HookOutcome::Continue,
            _ => HookOutcome::Continue,
        });
    }
    if let Some(modified) = v.get("modified_input") {
        return Some(HookOutcome::Modified(modified.clone()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Output;
    use std::process::ExitStatus;

    #[test]
    fn parse_ask_decision() {
        let out = parse_hook_stdout_json(r#"{"decision":"ask","reason":"需要人工确认"}"#);
        assert!(matches!(out, Some(HookOutcome::Ask(_))));
    }

    #[test]
    fn parse_deny_decision() {
        let out = parse_hook_stdout_json(r#"{"decision":"deny","reason":"blocked"}"#);
        assert!(matches!(out, Some(HookOutcome::Deny(_))));
    }

    #[test]
    fn parse_modified_input() {
        let out = parse_hook_stdout_json(r#"{"modified_input":{"path":"foo"}}"#);
        assert!(matches!(out, Some(HookOutcome::Modified(_))));
    }

    #[test]
    #[cfg(unix)]
    fn exit_code_2_is_deny() {
        #[cfg(unix)]
        let status = {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(2 << 8)
        };
        #[cfg(not(unix))]
        let status = ExitStatus::default();
        let output = Output {
            status,
            stdout: b"reason".to_vec(),
            stderr: vec![],
        };
        let outcome = parse_hook_output("test", &output, false).unwrap();
        assert!(matches!(outcome, HookOutcome::Deny(_)));
    }
}
