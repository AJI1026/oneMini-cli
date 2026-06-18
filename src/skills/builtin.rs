pub struct BuiltinSkillDef {
    pub name: &'static str,
    pub description: &'static str,
    pub body: &'static str,
}

pub const BUILTIN_SKILLS: &[BuiltinSkillDef] = &[
    BuiltinSkillDef {
        name: "commit-message",
        description: "根据 git diff 生成规范的 commit message",
        body: include_str!("../../skills/commit-message/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "code-review",
        description: "结构化代码审查与改进建议",
        body: include_str!("../../skills/code-review/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "debug",
        description: "系统化调试：复现 → 定位 → 修复 → 验证",
        body: include_str!("../../skills/debug/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "refactor",
        description: "安全小步重构，保持行为不变",
        body: include_str!("../../skills/refactor/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "readme",
        description: "为项目或模块编写 README 文档",
        body: include_str!("../../skills/readme/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "explore-codebase",
        description: "快速理解陌生代码库结构与入口",
        body: include_str!("../../skills/explore-codebase/SKILL.md"),
    },
];
