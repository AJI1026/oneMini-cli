//! Anthropic 官方技能目录（https://github.com/anthropics/skills）

pub struct CatalogEntry {
    pub id: &'static str,
    pub category: &'static str,
    /// apache2 | proprietary | source-available
    pub license: &'static str,
    pub note: &'static str,
}

pub const ANTHROPIC_REPO: (&str, &str) = ("anthropics", "skills");

/// 首次配置时预装的设计类技能（Apache-2.0）
pub const DEFAULT_DESIGN_SKILL_IDS: &[&str] = &[
    "frontend-design",
    "canvas-design",
    "theme-factory",
    "web-artifacts-builder",
    "brand-guidelines",
];

pub const DESIGN_BUNDLE_MARKER: &str = ".design-bundle-v1";

pub const CATALOG: &[CatalogEntry] = &[
    // 文档处理（OneMini 内置 + 可选 Anthropic 增强版）
    CatalogEntry {
        id: "docx",
        category: "文档",
        license: "apache2",
        note: "Word 创建/编辑（内置）；install 可装 Anthropic 版",
    },
    CatalogEntry {
        id: "pdf",
        category: "文档",
        license: "apache2",
        note: "PDF 读写/合并/表单（内置）",
    },
    CatalogEntry {
        id: "pptx",
        category: "文档",
        license: "apache2",
        note: "PPT 创建/编辑（内置）",
    },
    CatalogEntry {
        id: "xlsx",
        category: "文档",
        license: "apache2",
        note: "Excel 公式/格式（内置）",
    },
    CatalogEntry {
        id: "doc-coauthoring",
        category: "文档",
        license: "apache2",
        note: "协作文档写作流程",
    },
    // 页面 / 视觉设计
    CatalogEntry {
        id: "frontend-design",
        category: "设计",
        license: "apache2",
        note: "Web UI 美学与排版",
    },
    CatalogEntry {
        id: "canvas-design",
        category: "设计",
        license: "apache2",
        note: "Canvas 视觉设计",
    },
    CatalogEntry {
        id: "theme-factory",
        category: "设计",
        license: "apache2",
        note: "主题与配色系统",
    },
    CatalogEntry {
        id: "web-artifacts-builder",
        category: "设计",
        license: "apache2",
        note: "构建 Web 组件/页面产物",
    },
    CatalogEntry {
        id: "brand-guidelines",
        category: "设计",
        license: "apache2",
        note: "品牌视觉规范",
    },
    CatalogEntry {
        id: "algorithmic-art",
        category: "设计",
        license: "apache2",
        note: "算法生成艺术",
    },
    CatalogEntry {
        id: "slack-gif-creator",
        category: "设计",
        license: "apache2",
        note: "Slack GIF 制作",
    },
    // 开发 / 测试
    CatalogEntry {
        id: "mcp-builder",
        category: "开发",
        license: "apache2",
        note: "构建 MCP Server",
    },
    CatalogEntry {
        id: "webapp-testing",
        category: "开发",
        license: "apache2",
        note: "Web 测试；需 Playwright",
    },
    CatalogEntry {
        id: "claude-api",
        category: "开发",
        license: "apache2",
        note: "Claude API 集成指南",
    },
    CatalogEntry {
        id: "skill-creator",
        category: "开发",
        license: "apache2",
        note: "编写新 Agent Skill",
    },
    // 协作 / 沟通
    CatalogEntry {
        id: "internal-comms",
        category: "协作",
        license: "apache2",
        note: "内部沟通文档",
    },
];

pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|e| e.id == id)
}

pub fn format_catalog_table() -> String {
    let rows: Vec<Vec<String>> = CATALOG
        .iter()
        .map(|e| {
            let lic = match e.license {
                "apache2" => "Apache-2.0",
                "proprietary" => "专有",
                _ => e.license,
            };
            vec![
                e.id.to_string(),
                e.category.to_string(),
                lic.to_string(),
                e.note.to_string(),
            ]
        })
        .collect();
    let table = crate::ui::render_table(&["ID", "分类", "许可", "说明"], &rows);
    format!(
        "技能目录（onemini skills install <id> 可安装 Anthropic 官方增强版）\n\
         来源: https://github.com/anthropics/skills\n\n\
         docx / pdf / pptx / xlsx 已随 CLI 内置（Apache-2.0，含 scripts/）。\n\n\
         {table}\n\n\
         快捷安装（可选，覆盖为用户版）:\n\
         onemini skills install docs     # Anthropic 文档四件套\n\
         onemini skills install design   # 设计技能包\n\
         onemini skills install pdf      # 单个 Anthropic 技能"
    )
}
