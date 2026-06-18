use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    #[default]
    Default,
    Plan,
    AcceptEdits,
    Auto,
    DontAsk,
    Bypass,
}

impl PermissionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Plan => "plan",
            Self::AcceptEdits => "accept-edits",
            Self::Auto => "auto",
            Self::DontAsk => "dont-ask",
            Self::Bypass => "bypass",
        }
    }

    /// REPL 中 `/mode` 可选的模式（不含 dont-ask / bypass）
    pub fn repl_choices(disable_auto: bool) -> &'static [Self] {
        if disable_auto {
            &[Self::Default, Self::Plan, Self::AcceptEdits]
        } else {
            &[Self::Default, Self::Plan, Self::AcceptEdits, Self::Auto]
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Default => "变更类工具需确认",
            Self::Plan => "只读：仅 read / grep / glob / fetch",
            Self::AcceptEdits => "工作区内 write/edit 与安全 bash 自动放行",
            Self::Auto => "启发式分类器自动判断",
            Self::DontAsk => "未匹配 allow 规则则拒绝",
            Self::Bypass => "跳过所有确认（仅隔离环境）",
        }
    }

    pub fn select_label(self) -> String {
        format!("{}  —  {}", self.label(), self.description())
    }

    pub fn is_readonly(self) -> bool {
        matches!(self, Self::Plan)
    }
}

impl FromStr for PermissionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "default" => Ok(Self::Default),
            "plan" => Ok(Self::Plan),
            "accept-edits" | "acceptedits" => Ok(Self::AcceptEdits),
            "auto" => Ok(Self::Auto),
            "dont-ask" | "dontask" => Ok(Self::DontAsk),
            "bypass" | "bypass-permissions" => Ok(Self::Bypass),
            other => Err(format!("未知权限模式: {other}")),
        }
    }
}
