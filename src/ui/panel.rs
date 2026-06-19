//! 通用面板 — Welcome / Tool / Warning 框，对齐 README session/hero 配图

use colored::Colorize;

use super::palette::{Palette, Rgb};
use super::theme;

/// 面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Tool,
    Warning,
}

fn panel_bg_enabled() -> bool {
    theme::colors_enabled()
        && std::env::var("ONEMINI_PLAIN_UI").ok().as_deref() != Some("1")
}

fn fill_for(kind: PanelKind, p: Palette) -> Rgb {
    match kind {
        PanelKind::Tool => p.tool_fill,
        PanelKind::Warning => p.warn_fill,
    }
}

fn border_for(kind: PanelKind, p: Palette) -> Rgb {
    match kind {
        PanelKind::Warning => p.warn_border,
        _ => p.panel_border,
    }
}

fn style_border_char(c: char, rgb: Rgb) -> String {
    if !theme::colors_enabled() {
        return c.to_string();
    }
    c.to_string().truecolor(rgb.0, rgb.1, rgb.2).to_string()
}

fn style_panel_line(text: &str, kind: PanelKind, p: Palette) -> String {
    if !theme::colors_enabled() {
        return format!("| {text} |");
    }
    let fill = fill_for(kind, p);
    let border = border_for(kind, p);
    let left = style_border_char(
        theme::border_vertical().chars().next().unwrap_or('|'),
        border,
    );
    let body = if panel_bg_enabled() {
        format!(" {text} ")
            .on_truecolor(fill.0, fill.1, fill.2)
            .truecolor(p.text_bright.0, p.text_bright.1, p.text_bright.2)
            .to_string()
    } else {
        format!(" {text} ")
    };
    format!("{left}{body}")
}

fn horizontal_rule(width: usize, kind: PanelKind, p: Palette) -> String {
    let border = border_for(kind, p);
    let ch = theme::border_horizontal();
    if !theme::colors_enabled() {
        return ch.repeat(width);
    }
    ch.repeat(width)
        .truecolor(border.0, border.1, border.2)
        .to_string()
}

fn corner(kind: PanelKind, which: Corner, p: Palette) -> String {
    let border = border_for(kind, p);
    let c = match which {
        Corner::TopLeft => theme::border_top_left(),
        Corner::TopRight => theme::border_top_right(),
        Corner::BottomLeft => theme::border_bottom_left(),
        Corner::BottomRight => theme::border_bottom_right(),
    };
    style_border_char(c.chars().next().unwrap_or('+'), border)
}

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// 渲染带边框的面板（每行一条内容）
pub fn render_panel(kind: PanelKind, lines: &[&str], width: usize) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let p = theme::current_palette();
    let inner = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(4);
    let total_inner = inner + 2;
    let panel_width = width.max(total_inner + 2);

    let mut out = Vec::new();
    let top_mid = horizontal_rule(panel_width.saturating_sub(2), kind, p);
    out.push(format!(
        "{}{}{}",
        corner(kind, Corner::TopLeft, p),
        top_mid,
        corner(kind, Corner::TopRight, p)
    ));

    for line in lines {
        let pad = panel_width.saturating_sub(2).saturating_sub(line.chars().count());
        let padded = format!("{line}{}", " ".repeat(pad));
        out.push(style_panel_line(&padded, kind, p));
    }

    let bottom_mid = horizontal_rule(panel_width.saturating_sub(2), kind, p);
    out.push(format!(
        "{}{}{}",
        corner(kind, Corner::BottomLeft, p),
        bottom_mid,
        corner(kind, Corner::BottomRight, p)
    ));

    out.join("\n")
}

/// macOS 风窗口顶栏：`● ● ●  onemini - zsh`
pub fn render_window_chrome(title: &str) -> String {
    let p = theme::current_palette();
    if !theme::colors_enabled() {
        return format!("ooo  {title}");
    }
    let close = "●".truecolor(p.chrome_close.0, p.chrome_close.1, p.chrome_close.2);
    let min = "●".truecolor(p.chrome_minimize.0, p.chrome_minimize.1, p.chrome_minimize.2);
    let max = "●".truecolor(p.chrome_maximize.0, p.chrome_maximize.1, p.chrome_maximize.2);
    let title_styled = title.truecolor(p.text_dim.0, p.text_dim.1, p.text_dim.2);
    format!("{close} {min} {max}  {title_styled}")
}

/// Welcome 条：`* Welcome to oneMini`（纯文字，无填充面板）
pub fn render_welcome_strip() -> String {
    let p = theme::current_palette();
    if theme::colors_enabled() {
        format!(
            "* {}",
            "Welcome to oneMini".truecolor(p.glow_mid.0, p.glow_mid.1, p.glow_mid.2)
        )
    } else {
        "* Welcome to oneMini".to_string()
    }
}

/// 工具调用面板
pub fn render_tool_panel(name: &str, detail: &str, result: Option<&str>) -> String {
    let p = theme::current_palette();
    let arrow = if theme::colors_enabled() {
        format!(
            "-> {}",
            format!("{name}  {detail}").truecolor(p.text_muted.0, p.text_muted.1, p.text_muted.2)
        )
    } else {
        format!("-> {name}  {detail}")
    };
    let mut lines: Vec<String> = vec![arrow];
    if let Some(r) = result {
        let ok = if theme::colors_enabled() {
            format!(
                "  ok {}",
                r.truecolor(p.text_dim.0, p.text_dim.1, p.text_dim.2)
            )
        } else {
            format!("  ok {r}")
        };
        lines.push(ok);
    }
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    render_panel(PanelKind::Tool, &refs, 68)
}

/// 工具执行结果行（接在 tool 面板后）
pub fn render_tool_result(text: &str) -> String {
    let p = theme::current_palette();
    if theme::colors_enabled() {
        format!(
            "  ok {}",
            text.truecolor(p.text_dim.0, p.text_dim.1, p.text_dim.2)
        )
    } else {
        format!("  ok {text}")
    }
}

/// 权限确认警告面板（配图风格）
pub fn render_permission_panel(tool_name: &str, detail: &str) -> String {
    let p = theme::current_palette();
    let action = if detail.is_empty() {
        tool_name.to_string()
    } else {
        format!("{tool_name} ({detail})")
    };
    let headline = if theme::colors_enabled() {
        format!(
            "! 需要确认: {}",
            action.truecolor(p.warn_text.0, p.warn_text.1, p.warn_text.2)
        )
    } else {
        format!("! 需要确认: {action}")
    };
    let options = if theme::colors_enabled() {
        format!(
            "  [1] {}  [2] {}  [3] {}",
            "允许".truecolor(p.warn_muted.0, p.warn_muted.1, p.warn_muted.2),
            "拒绝".truecolor(p.warn_muted.0, p.warn_muted.1, p.warn_muted.2),
            "始终允许".truecolor(p.warn_muted.0, p.warn_muted.1, p.warn_muted.2),
        )
    } else {
        "  [1] 允许  [2] 拒绝  [3] 始终允许".to_string()
    };
    let hint = if theme::colors_enabled() {
        format!(
            "  {}",
            "输入 /mode 切换权限模式".truecolor(p.text_dim.0, p.text_dim.1, p.text_dim.2)
        )
    } else {
        "  输入 /mode 切换权限模式".to_string()
    };
    render_panel(
        PanelKind::Warning,
        &[&headline, &options, &hint],
        68,
    )
}

/// 工具调用面板
mod tests {
    use super::*;
    use crate::ui::theme::{self, ThemeId};

    #[test]
    fn panel_has_corners_without_color() {
        let _g = theme::theme_test_guard();
        std::env::set_var("NO_COLOR", "1");
        theme::set_theme(ThemeId::Modern);
        let out = render_panel(PanelKind::Tool, &["-> read file"], 20);
        assert!(out.contains("-> read file"));
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn welcome_strip_contains_text() {
        let _g = theme::theme_test_guard();
        theme::set_theme(ThemeId::Modern);
        let out = render_welcome_strip();
        assert!(out.contains("Welcome to oneMini"));
    }

    #[test]
    fn window_chrome_has_dots() {
        let _g = theme::theme_test_guard();
        theme::set_theme(ThemeId::Modern);
        let out = render_window_chrome("onemini - zsh");
        assert!(out.contains("onemini"));
    }
}
