pub struct BuiltinSkillDef {
    pub name: &'static str,
    pub description: &'static str,
    pub body: &'static str,
}

pub const BUILTIN_SKILLS: &[BuiltinSkillDef] = &[
    BuiltinSkillDef {
        name: "storyboard",
        description: "将创意拆成可拍可生成的分镜脚本",
        body: include_str!("../../skills/storyboard/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "character-brief",
        description: "整理角色外形、性格与服装设定卡",
        body: include_str!("../../skills/character-brief/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "prompt-polish",
        description: "扩写结构化图/视频提示词",
        body: include_str!("../../skills/prompt-polish/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "visual-style",
        description: "统一色调、镜头语言与材质关键词",
        body: include_str!("../../skills/visual-style/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "shot-list",
        description: "编排集数与镜头表字段",
        body: include_str!("../../skills/shot-list/SKILL.md"),
    },
    BuiltinSkillDef {
        name: "blender-modeling",
        description: "引导通过 CLI 本机 MCP 连接 Blender（Web 不开放）",
        body: include_str!("../../skills/blender-modeling/SKILL.md"),
    },
];
