use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleEffect {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionRule {
    pub effect: RuleEffect,
    pub tool: String,
    #[serde(default)]
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PermissionRulesFile {
    /// 旧格式兼容
    #[serde(default)]
    pub always_allow: Vec<String>,
    #[serde(default)]
    pub always_deny: Vec<String>,
    #[serde(default)]
    pub bash_allow_patterns: Vec<String>,
    #[serde(default)]
    pub bash_deny_patterns: Vec<String>,
    #[serde(default)]
    pub rules: Vec<PermissionRule>,
}

impl PermissionRulesFile {
    pub fn migrate_legacy(&mut self) {
        for t in self.always_deny.drain(..) {
            self.rules.push(PermissionRule {
                effect: RuleEffect::Deny,
                tool: t,
                pattern: String::new(),
            });
        }
        for t in self.always_allow.drain(..) {
            self.rules.push(PermissionRule {
                effect: RuleEffect::Allow,
                tool: t,
                pattern: String::new(),
            });
        }
        for p in self.bash_deny_patterns.drain(..) {
            self.rules.push(PermissionRule {
                effect: RuleEffect::Deny,
                tool: "bash".into(),
                pattern: p,
            });
        }
        for p in self.bash_allow_patterns.drain(..) {
            self.rules.push(PermissionRule {
                effect: RuleEffect::Allow,
                tool: "bash".into(),
                pattern: p,
            });
        }
    }

    pub fn builtin_defaults() -> Self {
        Self {
            rules: vec![
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "read".into(),
                    pattern: "**/.env*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "read".into(),
                    pattern: "**/id_rsa*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "read".into(),
                    pattern: "**/*.pem".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "read".into(),
                    pattern: "**/credentials.json".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "grep".into(),
                    pattern: "**/.env*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "grep".into(),
                    pattern: "**/id_rsa*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "grep".into(),
                    pattern: "**/*.pem".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "grep".into(),
                    pattern: "**/credentials.json".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "glob".into(),
                    pattern: "**/.env*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "glob".into(),
                    pattern: "**/id_rsa*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "glob".into(),
                    pattern: "**/*.pem".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "glob".into(),
                    pattern: "**/credentials.json".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "write".into(),
                    pattern: "**/.env*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "edit".into(),
                    pattern: "**/.env*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "write".into(),
                    pattern: "**/id_rsa*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "edit".into(),
                    pattern: "**/id_rsa*".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "write".into(),
                    pattern: ".git/**".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "edit".into(),
                    pattern: ".git/**".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "write".into(),
                    pattern: ".onemini/**".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "edit".into(),
                    pattern: ".onemini/**".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "bash".into(),
                    pattern: "rm -rf /".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "bash".into(),
                    pattern: "rm -rf ~".into(),
                },
                PermissionRule {
                    effect: RuleEffect::Deny,
                    tool: "bash".into(),
                    pattern: "git push --force*".into(),
                },
            ],
            ..Default::default()
        }
    }
}
