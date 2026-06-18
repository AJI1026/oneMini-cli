//! OneMini CLI 主题 — Modern / Game Boy / NES 三档复古主机风格

use colored::Colorize;
use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicU8, Ordering};

/// 主题标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Modern,
    GameBoy,
    Nes,
}

impl ThemeId {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "modern" | "default" => Some(Self::Modern),
            "gameboy" | "gb" | "dmg" => Some(Self::GameBoy),
            "nes" | "fc" => Some(Self::Nes),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modern => "modern",
            Self::GameBoy => "gameboy",
            Self::Nes => "nes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern（默认）",
            Self::GameBoy => "Game Boy DMG",
            Self::Nes => "NES / FC",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Modern => "cyan 现代终端风",
            Self::GameBoy => "四色绿像素 / DMG",
            Self::Nes => "蓝底 FC 卡带风",
        }
    }

    pub fn select_label(self) -> String {
        format!("{}  —  {}", self.label(), self.description())
    }

    pub const ALL: [Self; 3] = [Self::Modern, Self::GameBoy, Self::Nes];
}

static ACTIVE_THEME: AtomicU8 = AtomicU8::new(0);

fn theme_to_u8(t: ThemeId) -> u8 {
    match t {
        ThemeId::Modern => 0,
        ThemeId::GameBoy => 1,
        ThemeId::Nes => 2,
    }
}

fn u8_to_theme(v: u8) -> ThemeId {
    match v {
        1 => ThemeId::GameBoy,
        2 => ThemeId::Nes,
        _ => ThemeId::Modern,
    }
}

/// 初始化主题：ONEMINI_THEME 环境变量 > config [ui].theme
pub fn init_theme(config_theme: Option<&str>) {
    let theme = std::env::var("ONEMINI_THEME")
        .ok()
        .and_then(|s| ThemeId::parse(&s))
        .or_else(|| config_theme.and_then(ThemeId::parse))
        .unwrap_or(ThemeId::Modern);
    set_theme(theme);
}

pub fn set_theme(theme: ThemeId) {
    ACTIVE_THEME.store(theme_to_u8(theme), Ordering::Relaxed);
}

pub fn current_theme() -> ThemeId {
    u8_to_theme(ACTIVE_THEME.load(Ordering::Relaxed))
}

/// 是否启用 ANSI 颜色（尊重 NO_COLOR 与 TTY）
pub fn colors_enabled() -> bool {
    std::env::var("NO_COLOR").is_err() && io::stdout().is_terminal()
}

fn plain(text: &str) -> String {
    text.to_string()
}

/// 主色：标题、助手标识
pub fn primary(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.bright_cyan().bold().to_string(),
        ThemeId::GameBoy => text.truecolor(155, 188, 15).bold().to_string(),
        ThemeId::Nes => text.on_truecolor(0, 0, 170).white().bold().to_string(),
    }
}

/// 主色浅色：工具名、代码、链接
pub fn primary_light(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.cyan().to_string(),
        ThemeId::GameBoy => text.truecolor(139, 172, 15).to_string(),
        ThemeId::Nes => text.truecolor(255, 255, 255).to_string(),
    }
}

/// 强调色：用户标识、重要状态
pub fn accent(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.bright_blue().to_string(),
        ThemeId::GameBoy => text.truecolor(48, 98, 48).to_string(),
        ThemeId::Nes => text.yellow().to_string(),
    }
}

/// 次要文字：说明、预览
pub fn muted(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.bright_black().to_string(),
        ThemeId::GameBoy => text.truecolor(48, 98, 48).to_string(),
        ThemeId::Nes => text.truecolor(170, 170, 170).to_string(),
    }
}

/// 次要文字（稍亮，用于卡片内正文）
pub fn muted_strong(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.white().dimmed().to_string(),
        ThemeId::GameBoy => text.truecolor(139, 172, 15).dimmed().to_string(),
        ThemeId::Nes => text.white().dimmed().to_string(),
    }
}

/// 分隔线、边框
pub fn soft(text: &str) -> String {
    muted(text)
}

/// AI 思考状态标签
pub fn thinking_label(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.cyan().bold().to_string(),
        ThemeId::GameBoy => text.truecolor(139, 172, 15).bold().to_string(),
        ThemeId::Nes => text.truecolor(255, 255, 255).dimmed().to_string(),
    }
}

/// AI 思考详情（展开模式）
pub fn thinking_detail(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.white().dimmed().italic().to_string(),
        ThemeId::GameBoy => text.truecolor(48, 98, 48).italic().to_string(),
        ThemeId::Nes => text.truecolor(170, 170, 170).italic().to_string(),
    }
}

/// 警告正文
pub fn warning(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    match current_theme() {
        ThemeId::Modern => text.yellow().bold().to_string(),
        ThemeId::GameBoy => text.truecolor(155, 188, 15).bold().to_string(),
        ThemeId::Nes => text.red().bold().to_string(),
    }
}

pub fn success_icon() -> String {
    match current_theme() {
        ThemeId::Modern if colors_enabled() => "[+]".bright_green().bold().to_string(),
        ThemeId::GameBoy if colors_enabled() => "[+]".truecolor(155, 188, 15).bold().to_string(),
        ThemeId::Nes if colors_enabled() => "[+]".green().bold().to_string(),
        _ => "[+]".to_string(),
    }
}

pub fn error_icon() -> String {
    match current_theme() {
        ThemeId::Modern if colors_enabled() => "[X]".red().bold().to_string(),
        ThemeId::GameBoy if colors_enabled() => "[X]".truecolor(48, 98, 48).bold().to_string(),
        ThemeId::Nes if colors_enabled() => "[X]".red().bold().to_string(),
        _ => "[X]".to_string(),
    }
}

pub fn warn_icon() -> String {
    match current_theme() {
        ThemeId::Modern if colors_enabled() => "[!]".yellow().bold().to_string(),
        ThemeId::GameBoy if colors_enabled() => "[!]".truecolor(139, 172, 15).bold().to_string(),
        ThemeId::Nes if colors_enabled() => "[!]".yellow().bold().to_string(),
        _ => "[!]".to_string(),
    }
}

pub fn tool_icon() -> String {
    match current_theme() {
        ThemeId::Modern if colors_enabled() => ">".bright_cyan().bold().to_string(),
        ThemeId::GameBoy if colors_enabled() => ">".truecolor(155, 188, 15).bold().to_string(),
        ThemeId::Nes if colors_enabled() => ">".white().bold().to_string(),
        _ => ">".to_string(),
    }
}

pub fn thinking_icon() -> String {
    match current_theme() {
        ThemeId::Modern if colors_enabled() => "*".cyan().to_string(),
        ThemeId::GameBoy if colors_enabled() => "*".truecolor(139, 172, 15).to_string(),
        ThemeId::Nes if colors_enabled() => "~".white().dimmed().to_string(),
        _ => "*".to_string(),
    }
}

pub fn choice_yes(text: &str) -> String {
    if colors_enabled() {
        text.green().bold().to_string()
    } else {
        plain(text)
    }
}

pub fn choice_default(text: &str) -> String {
    if colors_enabled() {
        text.white().bold().to_string()
    } else {
        plain(text)
    }
}

pub fn choice_always(text: &str) -> String {
    if colors_enabled() {
        text.yellow().bold().to_string()
    } else {
        plain(text)
    }
}

pub fn diff_add(line: &str) -> String {
    if colors_enabled() {
        line.green().to_string()
    } else {
        plain(line)
    }
}

pub fn diff_remove(line: &str) -> String {
    if colors_enabled() {
        line.red().dimmed().to_string()
    } else {
        plain(line)
    }
}

/// 品牌 Logo 行着色（帧动画用）
pub fn logo_line(text: &str, frame: usize, line_idx: usize) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    let idx = (frame + line_idx) % 4;
    match current_theme() {
        ThemeId::Modern => match idx {
            0 => text.bright_cyan().to_string(),
            1 => text.cyan().to_string(),
            2 => text.bright_blue().to_string(),
            _ => text.white().dimmed().to_string(),
        },
        ThemeId::GameBoy => match idx {
            0 => text.truecolor(155, 188, 15).to_string(),
            1 => text.truecolor(139, 172, 15).to_string(),
            2 => text.truecolor(48, 98, 48).to_string(),
            _ => text.truecolor(15, 56, 15).to_string(),
        },
        ThemeId::Nes => match idx {
            0 => text.white().on_truecolor(0, 0, 170).to_string(),
            1 => text.yellow().on_truecolor(0, 0, 170).to_string(),
            2 => text.red().on_truecolor(0, 0, 170).to_string(),
            _ => text.truecolor(170, 170, 170).to_string(),
        },
    }
}

/// 边框字符
pub fn border_top_left() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "┌",
        ThemeId::GameBoy | ThemeId::Nes => "+",
    }
}

pub fn border_top_right() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "┐",
        ThemeId::GameBoy | ThemeId::Nes => "+",
    }
}

pub fn border_bottom_left() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "└",
        ThemeId::GameBoy | ThemeId::Nes => "+",
    }
}

pub fn border_bottom_right() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "┘",
        ThemeId::GameBoy | ThemeId::Nes => "+",
    }
}

pub fn border_horizontal() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "─",
        ThemeId::GameBoy | ThemeId::Nes => "-",
    }
}

pub fn border_vertical() -> &'static str {
    match current_theme() {
        ThemeId::Modern => "│",
        ThemeId::GameBoy | ThemeId::Nes => "|",
    }
}

pub fn separator_line(width: usize) -> String {
    let ch = border_horizontal();
    soft(&ch.repeat(width))
}

pub fn panel_title(text: &str) -> String {
    match current_theme() {
        ThemeId::Nes => format!("= {} =", primary(text)),
        _ => primary(text),
    }
}

pub fn list_bullet() -> String {
    match current_theme() {
        ThemeId::Modern => accent("•"),
        ThemeId::GameBoy | ThemeId::Nes => accent("*"),
    }
}

pub fn use_retro_table() -> bool {
    !matches!(current_theme(), ThemeId::Modern)
}
