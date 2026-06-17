use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use super::auto::{auto_classify, AutoDecision};
use super::circuit_breaker::{check_circuit_breaker, CircuitBreaker};
use super::mode::PermissionMode;
use super::patterns::{path_pattern_match, pattern_match, tool_name_matches};
use super::rules::{PermissionRule, PermissionRulesFile, RuleEffect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny(String),
    Ask(String),
}

pub struct PermissionManager {
    user_rules: PermissionRulesFile,
    managed_rules: PermissionRulesFile,
    builtin_rules: PermissionRulesFile,
    path: PathBuf,
    allow_managed_rules_only: bool,
}

impl PermissionManager {
    pub fn load(managed: &crate::managed::ManagedSettings) -> Result<Self> {
        let path = crate::config::Config::config_dir()?.join("permissions.toml");
        let user_rules = if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("读取权限配置失败: {}", path.display()))?;
            let mut file: PermissionRulesFile = toml::from_str(&text).unwrap_or_default();
            if !file.always_allow.is_empty()
                || !file.always_deny.is_empty()
                || !file.bash_allow_patterns.is_empty()
                || !file.bash_deny_patterns.is_empty()
            {
                file.migrate_legacy();
            }
            file
        } else {
            PermissionRulesFile {
                rules: vec![
                    super::rules::PermissionRule {
                        effect: RuleEffect::Allow,
                        tool: "read".into(),
                        pattern: String::new(),
                    },
                    super::rules::PermissionRule {
                        effect: RuleEffect::Allow,
                        tool: "grep".into(),
                        pattern: String::new(),
                    },
                    super::rules::PermissionRule {
                        effect: RuleEffect::Allow,
                        tool: "glob".into(),
                        pattern: String::new(),
                    },
                ],
                ..Default::default()
            }
        };

        let builtin_rules = PermissionRulesFile::builtin_defaults();
        let allow_managed_rules_only = managed.allow_managed_rules_only;

        Ok(Self {
            user_rules,
            managed_rules: managed.rules.clone(),
            builtin_rules,
            path,
            allow_managed_rules_only,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn evaluate(
        &self,
        tool: &str,
        detail: &str,
        args: &Value,
        workdir: &Path,
        mode: PermissionMode,
        bypass: bool,
    ) -> PermissionDecision {
        if let Some(cb) = check_circuit_breaker(tool, detail) {
            return match cb {
                CircuitBreaker::HardDeny => {
                    PermissionDecision::Deny("熔断：该操作被硬拒绝".into())
                }
                CircuitBreaker::ForceAsk => PermissionDecision::Ask("熔断：该操作需人工确认".into()),
            };
        }

        if mode == PermissionMode::Plan {
            if is_mutating_tool(tool) {
                return PermissionDecision::Deny("plan 模式禁止变更类工具".into());
            }
            return PermissionDecision::Allow;
        }

        if let Some(decision) = self.evaluate_rules(tool, detail, args, workdir) {
            return decision;
        }

        if mode == PermissionMode::DontAsk {
            return PermissionDecision::Deny("dont-ask 模式：未匹配 allow 规则".into());
        }

        if mode == PermissionMode::AcceptEdits {
            if matches!(tool, "write" | "edit") && !is_sensitive_write(args, workdir) {
                return PermissionDecision::Allow;
            }
            if tool == "bash" && is_safe_filesystem_bash(detail) {
                return PermissionDecision::Allow;
            }
        }

        if mode == PermissionMode::Auto {
            return match auto_classify(tool, args, detail, workdir) {
                AutoDecision::Allow => PermissionDecision::Allow,
                AutoDecision::Deny => {
                    PermissionDecision::Deny("[auto_blocked] 启发式分类器拒绝该操作".into())
                }
                AutoDecision::Ask => PermissionDecision::Ask(String::new()),
            };
        }

        if bypass {
            return PermissionDecision::Allow;
        }

        if !is_mutating_tool(tool) {
            return PermissionDecision::Allow;
        }

        PermissionDecision::Ask(String::new())
    }

    fn evaluate_rules(
        &self,
        tool: &str,
        detail: &str,
        args: &Value,
        workdir: &Path,
    ) -> Option<PermissionDecision> {
        let user = if self.allow_managed_rules_only {
            &PermissionRulesFile::default()
        } else {
            &self.user_rules
        };

        for (rules, _) in [
            (&self.managed_rules, true),
            (user, false),
            (&self.builtin_rules, true),
        ] {
            if let Some(d) = self.match_effect(rules, RuleEffect::Deny, tool, detail, args, workdir) {
                return Some(d);
            }
        }
        for (rules, _) in [
            (&self.managed_rules, true),
            (user, false),
        ] {
            if let Some(d) = self.match_effect(rules, RuleEffect::Ask, tool, detail, args, workdir) {
                return Some(d);
            }
        }
        for (rules, _) in [
            (&self.managed_rules, true),
            (user, false),
            (&self.builtin_rules, false),
        ] {
            if let Some(d) = self.match_effect(rules, RuleEffect::Allow, tool, detail, args, workdir)
            {
                return Some(d);
            }
        }
        None
    }

    fn match_effect(
        &self,
        file: &PermissionRulesFile,
        effect: RuleEffect,
        tool: &str,
        detail: &str,
        args: &Value,
        workdir: &Path,
    ) -> Option<PermissionDecision> {
        for rule in &file.rules {
            if rule.effect != effect {
                continue;
            }
            if !tool_name_matches(&rule.tool, tool) {
                continue;
            }
            if rule.pattern.is_empty() {
                return Some(decision_from_effect(effect, &rule.tool, &rule.pattern));
            }
            if rule_matches(rule, tool, detail, args, workdir) {
                return Some(decision_from_effect(effect, &rule.tool, &rule.pattern));
            }
        }
        None
    }

    pub fn add_allow_rule(&mut self, tool: &str, pattern: &str) -> Result<()> {
        self.user_rules.rules.push(PermissionRule {
            effect: RuleEffect::Allow,
            tool: tool.to_string(),
            pattern: pattern.to_string(),
        });
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        let text = toml::to_string_pretty(&self.user_rules).context("序列化权限配置失败")?;
        crate::fs_util::write_private(&self.path, text)?;
        Ok(())
    }

    pub fn format_summary(
        &self,
        mode: PermissionMode,
        managed: &crate::managed::ManagedSettings,
    ) -> String {
        fn count_rules(file: &PermissionRulesFile) -> (usize, usize, usize) {
            let mut allow = 0;
            let mut deny = 0;
            let mut ask = 0;
            for r in &file.rules {
                match r.effect {
                    RuleEffect::Allow => allow += 1,
                    RuleEffect::Deny => deny += 1,
                    RuleEffect::Ask => ask += 1,
                }
            }
            (allow, deny, ask)
        }

        let (b_a, b_d, b_k) = count_rules(&self.builtin_rules);
        let (m_a, m_d, m_k) = count_rules(&self.managed_rules);
        let user = if self.allow_managed_rules_only {
            &PermissionRulesFile::default()
        } else {
            &self.user_rules
        };
        let (u_a, u_d, u_k) = count_rules(user);

        let managed_src = managed
            .source
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(未配置)".into());

        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            crate::ui::status_pair("当前权限模式", mode.label()),
            crate::ui::status_pair("用户规则文件", &self.path.display().to_string()),
            crate::ui::status_pair("托管策略", &managed_src),
            crate::ui::status_pair(
                "内置规则",
                &format!("allow {b_a} · deny {b_d} · ask {b_k}"),
            ),
            crate::ui::status_pair(
                "托管规则",
                &format!("allow {m_a} · deny {m_d} · ask {m_k}"),
            ),
            crate::ui::status_pair(
                "用户规则",
                &format!("allow {u_a} · deny {u_d} · ask {u_k}"),
            ),
            crate::ui::status_pair(
                "仅托管规则",
                if self.allow_managed_rules_only {
                    "是"
                } else {
                    "否"
                },
            ),
            crate::ui::dim("编辑用户规则: permissions.toml · 托管: managed.toml"),
        )
    }
}

fn rule_matches(
    rule: &PermissionRule,
    tool: &str,
    detail: &str,
    args: &Value,
    workdir: &Path,
) -> bool {
    let tool_lc = tool.to_lowercase();
    if tool_lc == "bash" {
        return pattern_match(&rule.pattern, detail);
    }
    if matches!(tool_lc.as_str(), "read" | "write" | "edit" | "grep" | "glob") {
        if let Some(path) = args["path"].as_str() {
            return path_pattern_match(&rule.pattern, path, workdir);
        }
        if let Some(pattern) = args["pattern"].as_str() {
            return pattern_match(&rule.pattern, pattern);
        }
    }
    pattern_match(&rule.pattern, detail)
}

fn decision_from_effect(effect: RuleEffect, tool: &str, pattern: &str) -> PermissionDecision {
    match effect {
        RuleEffect::Allow => PermissionDecision::Allow,
        RuleEffect::Deny => PermissionDecision::Deny(format!(
            "规则拒绝: {tool}{}",
            if pattern.is_empty() {
                String::new()
            } else {
                format!(" ({pattern})")
            }
        )),
        RuleEffect::Ask => PermissionDecision::Ask(format!(
            "规则要求确认: {tool}{}",
            if pattern.is_empty() {
                String::new()
            } else {
                format!(" ({pattern})")
            }
        )),
    }
}

fn is_mutating_tool(tool: &str) -> bool {
    matches!(tool, "write" | "edit" | "bash" | "delegate")
        || tool.starts_with("mcp_")
}

fn is_sensitive_write(args: &Value, workdir: &Path) -> bool {
    args["path"]
        .as_str()
        .is_some_and(|p| is_sensitive_path_write(p, workdir))
}

fn is_sensitive_path_write(path: &str, workdir: &Path) -> bool {
    path_pattern_match("**/.env*", path, workdir)
        || path_pattern_match("**/id_rsa*", path, workdir)
        || path_pattern_match(".git/**", path, workdir)
        || path_pattern_match(".onemini/**", path, workdir)
}

/// accept-edits 模式下可自动放行的安全文件系统 bash（单条、无链式）。
fn is_safe_filesystem_bash(detail: &str) -> bool {
    let cmd = detail.trim();
    if cmd.is_empty() {
        return false;
    }
    let lower = cmd.to_lowercase();
    if lower.contains("&&")
        || lower.contains(';')
        || lower.contains('|')
        || lower.contains("$(")
        || lower.contains('`')
    {
        return false;
    }
    lower.starts_with("mkdir ")
        || lower.starts_with("touch ")
        || lower.starts_with("mv ")
        || lower.starts_with("cp ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed::ManagedSettings;
    use serde_json::json;
    use std::path::Path;

    fn test_manager() -> PermissionManager {
        PermissionManager::load(&ManagedSettings::default()).expect("load permissions")
    }

    #[test]
    fn builtin_deny_sensitive_read() {
        let mgr = test_manager();
        let workdir = Path::new("/tmp/project");
        let args = json!({"path": ".env"});
        let d = mgr.evaluate("read", ".env", &args, workdir, PermissionMode::Default, false);
        assert!(matches!(d, PermissionDecision::Deny(_)));
    }

    #[test]
    fn accept_edits_allows_safe_bash() {
        let mgr = test_manager();
        let workdir = Path::new("/tmp/project");
        let args = json!({"command": "mkdir src"});
        let d = mgr.evaluate(
            "bash",
            "mkdir src",
            &args,
            workdir,
            PermissionMode::AcceptEdits,
            false,
        );
        assert!(matches!(d, PermissionDecision::Allow));
    }

    #[test]
    fn accept_edits_rejects_chained_bash() {
        let mgr = test_manager();
        let workdir = Path::new("/tmp/project");
        let args = json!({"command": "mkdir a && rm -rf b"});
        let d = mgr.evaluate(
            "bash",
            "mkdir a && rm -rf b",
            &args,
            workdir,
            PermissionMode::AcceptEdits,
            false,
        );
        assert!(!matches!(d, PermissionDecision::Allow));
    }
}
