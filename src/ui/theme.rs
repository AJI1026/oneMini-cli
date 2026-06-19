//! OneMini CLI 主题 — Matrix / Game Boy / NES 三档复古终端风格

use colored::Colorize;
use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicU8, Ordering};

use super::palette::{self, Palette, Rgb};

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
            Self::Modern => "Matrix 磷光绿",
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

#[cfg(test)]
static THEME_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

#[cfg(test)]
pub(crate) fn theme_test_guard() -> std::sync::MutexGuard<'static, ()> {
    THEME_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn current_theme() -> ThemeId {
    u8_to_theme(ACTIVE_THEME.load(Ordering::Relaxed))
}

pub fn current_palette() -> Palette {
    palette::Palette::for_theme_index(ACTIVE_THEME.load(Ordering::Relaxed))
}

/// 是否启用 ANSI 颜色（尊重 NO_COLOR 与 TTY）
pub fn colors_enabled() -> bool {
    std::env::var("NO_COLOR").is_err() && io::stdout().is_terminal()
}

fn plain(text: &str) -> String {
    text.to_string()
}

fn paint(rgb: Rgb, text: &str) -> String {
    text.truecolor(rgb.0, rgb.1, rgb.2).to_string()
}

fn paint_bold(rgb: Rgb, text: &str) -> String {
    text.truecolor(rgb.0, rgb.1, rgb.2).bold().to_string()
}

fn paint_on_bg_bold(fg: Rgb, bg: Rgb, text: &str) -> String {
    text.truecolor(fg.0, fg.1, fg.2)
        .on_truecolor(bg.0, bg.1, bg.2)
        .bold()
        .to_string()
}

/// 主色：标题、助手标识
pub fn primary(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    let p = current_palette();
    match current_theme() {
        ThemeId::Nes => paint_on_bg_bold(p.glow_hi, p.bg, text),
        _ => paint_bold(p.glow_hi, text),
    }
}

/// 主色浅色：工具名、代码、链接
pub fn primary_light(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    paint(current_palette().glow_mid, text)
}

/// 强调色：用户标识、重要状态
pub fn accent(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    let p = current_palette();
    match current_theme() {
        ThemeId::Nes => text.yellow().to_string(),
        _ => paint(p.glow_mid, text),
    }
}

fn sample_brand_gradient(stops: [Rgb; 3], t: f32) -> Rgb {
    let t = t.fract();
    if t <= 0.5 {
        stops[0].lerp(stops[1], t * 2.0)
    } else {
        stops[1].lerp(stops[2], (t - 0.5) * 2.0)
    }
}

/// Logo 块字渐变（col_t 0–1 为位置，phase 用于扫光动画）
pub fn banner_logo_char_gradient(c: char, col_t: f32, phase: f32) -> String {
    if c == ' ' {
        return " ".to_string();
    }
    if !colors_enabled() {
        return c.to_string();
    }
    let stops = current_palette().brand_gradient_stops();
    let rgb = sample_brand_gradient(stops, (col_t + phase).fract());
    paint_bold(rgb, &c.to_string())
}

/// 次要文字：说明、预览
pub fn muted(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    paint(current_palette().text_muted, text)
}

/// 次要文字（稍亮，用于卡片内正文）
pub fn muted_strong(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    let p = current_palette();
    paint(p.text_muted, text).dimmed().to_string()
}

/// 分隔线、边框
pub fn soft(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    paint(current_palette().panel_border, text)
}

/// AI 思考状态标签
pub fn thinking_label(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    paint_bold(current_palette().glow_hi, text)
}

/// AI 思考详情（展开模式）
pub fn thinking_detail(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    paint(current_palette().text_muted, text).dimmed().italic().to_string()
}

/// 警告正文
pub fn warning(text: &str) -> String {
    if !colors_enabled() {
        return plain(text);
    }
    let p = current_palette();
    match current_theme() {
        ThemeId::Modern => text.yellow().bold().to_string(),
        ThemeId::GameBoy => paint_bold(p.glow_hi, text),
        ThemeId::Nes => text.red().bold().to_string(),
    }
}

pub fn success_icon() -> String {
    if colors_enabled() {
        paint_bold(current_palette().glow_hi, "[+]")
    } else {
        "[+]".to_string()
    }
}

pub fn error_icon() -> String {
    if colors_enabled() {
        "[X]".red().bold().to_string()
    } else {
        "[X]".to_string()
    }
}

pub fn warn_icon() -> String {
    if colors_enabled() {
        match current_theme() {
            ThemeId::Modern => "[!]".yellow().bold().to_string(),
            _ => paint_bold(current_palette().warn_text, "[!]"),
        }
    } else {
        "[!]".to_string()
    }
}

pub fn thinking_icon() -> String {
    if !colors_enabled() {
        return "*".to_string();
    }
    match current_theme() {
        ThemeId::Nes => "~".white().dimmed().to_string(),
        _ => paint(current_palette().glow_mid, "*"),
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

/// 用户输入前缀 `you >`
pub fn user_prompt_prefix() -> String {
    if !colors_enabled() {
        return "you >".to_string();
    }
    let p = current_palette();
    format!(
        "{} {}",
        paint(p.user_prompt, "you"),
        paint_bold(p.user_prompt, ">")
    )
}

/// 助手前缀 `onemini >`
pub fn assistant_prompt_prefix() -> String {
    if !colors_enabled() {
        return "onemini >".to_string();
    }
    let p = current_palette();
    format!(
        "{} {}",
        paint(p.assistant_prompt, "onemini"),
        paint_bold(p.assistant_prompt, ">")
    )
}

/// Banner 右侧 meta 行着色
pub fn banner_meta_label(text: &str) -> String {
    if !colors_enabled() {
        return text.to_string();
    }
    paint(current_palette().glow_mid, text)
}

pub fn banner_meta_value(text: &str) -> String {
    if !colors_enabled() {
        return text.to_string();
    }
    paint(current_palette().text_dim, text)
}

pub fn banner_meta_prompt(text: &str) -> String {
    if !colors_enabled() {
        return text.to_string();
    }
    paint(current_palette().glow_hi, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::palette::Rgb;

    #[test]
    fn palette_tracks_theme() {
        let _g = theme_test_guard();
        set_theme(ThemeId::Modern);
        assert_eq!(current_palette().glow_hi, Rgb(57, 255, 20));
        set_theme(ThemeId::GameBoy);
        assert_eq!(current_palette().panel_fill, Rgb(155, 188, 15));
    }
}
