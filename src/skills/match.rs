//! 根据用户输入自动匹配 Agent Skill（无需 `/技能名`）。

use super::SkillRegistry;

pub struct SkillTriggerRule {
    pub id: &'static str,
    pub triggers: &'static [&'static str],
}

/// 技能 id → 触发词（中英文）；越具体的关键词权重越高（按字符长度计分）
pub const AUTO_TRIGGER_RULES: &[SkillTriggerRule] = &[
    // 内置编程技能
    SkillTriggerRule {
        id: "commit-message",
        triggers: &[
            "commit message",
            "commit msg",
            "提交信息",
            "提交说明",
            "写 commit",
            "git commit",
            "commit",
        ],
    },
    SkillTriggerRule {
        id: "code-review",
        triggers: &[
            "code review",
            "代码审查",
            "审查代码",
            "review pr",
            "review 代码",
            "pr 审查",
            "看看这段代码",
        ],
    },
    SkillTriggerRule {
        id: "debug",
        triggers: &[
            "调试",
            "debug",
            "报错",
            "异常",
            "不工作",
            "失败了",
            "测试失败",
            "bug",
            "fix bug",
            "根因",
            "stack trace",
            "堆栈",
        ],
    },
    SkillTriggerRule {
        id: "refactor",
        triggers: &["重构", "refactor", "整理代码", "去重复", "提取函数"],
    },
    SkillTriggerRule {
        id: "readme",
        triggers: &["readme", "README", "项目文档", "写文档", "安装说明", "使用说明"],
    },
    SkillTriggerRule {
        id: "explore-codebase",
        triggers: &[
            "代码库",
            "项目结构",
            "架构",
            "入口在哪",
            "怎么跑起来",
            "熟悉项目",
            "explore codebase",
            "codebase",
        ],
    },
    // Anthropic 设计类
    SkillTriggerRule {
        id: "frontend-design",
        triggers: &[
            "landing page",
            "landing",
            "首页",
            "页面设计",
            "网页设计",
            "前端设计",
            "frontend design",
            "web design",
            "界面设计",
            "ui 设计",
            "ui设计",
            "网站设计",
            "做一个页面",
            "设计页面",
        ],
    },
    SkillTriggerRule {
        id: "canvas-design",
        triggers: &["canvas", "canvas 设计", "视觉设计", "平面设计", "poster layout"],
    },
    SkillTriggerRule {
        id: "theme-factory",
        triggers: &["主题", "配色", "design token", "主题色", "dark mode", "浅色模式", "theme"],
    },
    SkillTriggerRule {
        id: "web-artifacts-builder",
        triggers: &[
            "web artifact",
            "html 页面",
            "react 组件",
            "vue 页面",
            "单页",
            "dashboard ui",
        ],
    },
    SkillTriggerRule {
        id: "brand-guidelines",
        triggers: &["品牌", "brand guideline", "视觉规范", "logo 规范", "品牌色"],
    },
    // 文档类（安装后可用）
    SkillTriggerRule {
        id: "pdf",
        triggers: &["pdf", "合并 pdf", "拆分 pdf", "提取 pdf", ".pdf"],
    },
    SkillTriggerRule {
        id: "docx",
        triggers: &["docx", "word 文档", "word文档", ".docx", "写 word"],
    },
    SkillTriggerRule {
        id: "pptx",
        triggers: &["pptx", "ppt", "幻灯片", "演示文稿", ".pptx"],
    },
    SkillTriggerRule {
        id: "xlsx",
        triggers: &["xlsx", "excel", "表格", ".xlsx", "电子表格"],
    },
    SkillTriggerRule {
        id: "doc-coauthoring",
        triggers: &["协作文档", "共同撰写", "coauthor", "合写文档"],
    },
    SkillTriggerRule {
        id: "mcp-builder",
        triggers: &["mcp server", "mcp 服务", "构建 mcp", "mcp-builder"],
    },
    SkillTriggerRule {
        id: "webapp-testing",
        triggers: &["playwright", "e2e 测试", "端到端测试", "web 测试", "浏览器测试"],
    },
];

const MIN_MATCH_SCORE: i32 = 4;

impl SkillRegistry {
    /// 自动匹配最合适的技能；可通过 `ONEMINI_NO_AUTO_SKILLS=1` 关闭
    pub fn auto_match<'a>(&'a self, input: &str) -> Option<&'a super::Skill> {
        if std::env::var("ONEMINI_NO_AUTO_SKILLS").ok().as_deref() == Some("1") {
            return None;
        }
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed.starts_with('[') {
            return None;
        }

        let lower = trimmed.to_lowercase();
        let mut best: Option<(&super::Skill, i32)> = None;

        for rule in AUTO_TRIGGER_RULES {
            let Some(skill) = self.get(rule.id) else {
                continue;
            };
            let mut score = 0i32;
            for trigger in rule.triggers {
                let t = trigger.to_lowercase();
                if t.len() < 3 {
                    continue;
                }
                if lower.contains(&t) {
                    score += 3 + t.chars().count() as i32;
                }
            }
            let name_spaced = rule.id.replace('-', " ");
            if lower.contains(rule.id) || lower.contains(&name_spaced) {
                score += 12;
            }
            if score >= MIN_MATCH_SCORE {
                if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
                    best = Some((skill, score));
                }
            }
        }
        best.map(|(skill, _)| skill)
    }
}

#[cfg(test)]
mod tests {
    use crate::skills::{Skill, SkillRegistry, SkillSource};

    fn test_registry(ids: &[&str]) -> SkillRegistry {
        let mut skills = std::collections::HashMap::new();
        for id in ids {
            skills.insert(
                (*id).to_string(),
                Skill {
                    name: (*id).to_string(),
                    description: String::new(),
                    source: SkillSource::Builtin,
                    path: None,
                    body: Some(String::new()),
                },
            );
        }
        SkillRegistry { skills }
    }

    #[test]
    fn auto_match_frontend_design() {
        let reg = test_registry(&["frontend-design"]);
        let skill = reg.auto_match("帮我做一个 SaaS landing page").unwrap();
        assert_eq!(skill.name, "frontend-design");
    }

    #[test]
    fn auto_match_debug() {
        let reg = test_registry(&["debug"]);
        let skill = reg.auto_match("登录接口一直报 500，帮我调试").unwrap();
        assert_eq!(skill.name, "debug");
    }

    #[test]
    fn auto_match_none_for_generic() {
        let reg = test_registry(&["debug", "frontend-design"]);
        assert!(reg.auto_match("你好").is_none());
    }
}
