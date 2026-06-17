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
        let mut user_rules = if path.exists() {
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

        if mode == PermissionMode::AcceptEdits
            && matches!(tool, "write" | "edit")
            && !is_sensitive_write(args, workdir)
        {
            return PermissionDecision::Allow;
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
