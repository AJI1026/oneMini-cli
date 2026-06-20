//! 通用面板 — Welcome / Tool / Warning，仅用文字色区分，不用背景块

use colored::Colorize;

use super::palette::{Palette, Rgb};
use super::theme;

/// 面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Welcome,
    Tool,
    Warning,
}

fn border_for(kind: PanelKind, p: Palette) -> Rgb {
    match kind {
        PanelKind::Warning => p.warn_border,
        _ => p.panel_border,
    }
}

fn text_color_for(kind: PanelKind, p: Palette) -> Rgb {
    match kind {
        PanelKind::Welcome => p.glow_mid,
        PanelKind::Tool => p.glow_mid,
        PanelKind::Warning => p.warn_text,
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
    let border = border_for(kind, p);
    let fg = text_color_for(kind, p);
    let left = style_border_char(
        theme::border_vertical().chars().next().unwrap_or('|'),
        border,
    );
    let body = format!(" {text} ").truecolor(fg.0, fg.1, fg.2).to_string();
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

/// 渲染带边框的面板（每行一条内容，无背景填充）
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

/// Welcome 条：`* Welcome to oneMini`
pub fn render_welcome_strip() -> String {
    if theme::colors_enabled() {
        format!("  {}", theme::accent("Welcome to oneMini"))
    } else {
        "* Welcome to oneMini".to_string()
    }
}

/// 工具调用行（无背景块）
pub fn render_tool_panel(name: &str, detail: &str, result: Option<&str>) -> String {
    let head = if theme::colors_enabled() {
        let name_styled = theme::primary_light(name);
        let detail_styled = if detail.is_empty() {
            String::new()
        } else {
            format!("  {}", theme::muted(detail))
        };
        format!("  {} {name_styled}{detail_styled}", theme::muted("->"))
    } else if detail.is_empty() {
        format!("  -> {name}")
    } else {
        format!("  -> {name}  {detail}")
    };

    let mut lines = vec![head];
    if let Some(r) = result {
        lines.push(render_tool_result(r));
    }
    lines.join("\n")
}

/// 工具执行结果行
pub fn render_tool_result(text: &str) -> String {
    if theme::colors_enabled() {
        format!("  {} {}", theme::muted("ok"), theme::muted_strong(text))
    } else {
        format!("  ok {text}")
    }
}

/// 权限确认（无背景块，用颜色区分层级）
pub fn render_permission_panel(tool_name: &str, detail: &str) -> String {
    let action = if detail.is_empty() {
        tool_name.to_string()
    } else {
        format!("{tool_name} ({detail})")
    };

    let headline = if theme::colors_enabled() {
        format!(
            "  {} {}",
            theme::warn_icon(),
            theme::warning(&format!("需要确认: {action}"))
        )
    } else {
        format!("! 需要确认: {action}")
    };

    let options = if theme::colors_enabled() {
        format!(
            "  [1] {}  [2] {}  [3] {}",
            theme::choice_yes("允许"),
            theme::choice_default("拒绝"),
            theme::choice_always("始终允许"),
        )
    } else {
        "  [1] 允许  [2] 拒绝  [3] 始终允许".to_string()
    };

    let hint = if theme::colors_enabled() {
        format!("  {}", theme::muted("输入 /mode 切换权限模式"))
    } else {
        "  输入 /mode 切换权限模式".to_string()
    };

    [headline, options, hint].join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        render_permission_panel, render_tool_panel, render_welcome_strip, render_window_chrome,
    };
    use crate::ui::theme::{self, ThemeId};

    #[test]
    fn tool_panel_is_text_only() {
        let _g = theme::theme_test_guard();
        theme::set_theme(ThemeId::GameBoy);
        let out = render_tool_panel("bash", "ls -lh", None);
        assert!(out.contains("bash"));
        assert!(!out.contains('+'));
    }

    #[test]
    fn permission_panel_is_text_only() {
        let _g = theme::theme_test_guard();
        theme::set_theme(ThemeId::GameBoy);
        let out = render_permission_panel("bash", "ls -lh");
        assert!(out.contains("需要确认"));
        assert!(out.contains("允许"));
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
