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

    pub fn cycle_next(self) -> Self {
        match self {
            Self::Default => Self::Plan,
            Self::Plan => Self::AcceptEdits,
            Self::AcceptEdits => Self::Auto,
            Self::Auto => Self::Default,
            Self::DontAsk => Self::Default,
            Self::Bypass => Self::Bypass,
        }
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
