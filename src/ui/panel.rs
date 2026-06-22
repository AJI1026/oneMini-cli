//! 通用面板 — Welcome / Tool / Warning，仅用文字色区分，不用背景块

use colored::Colorize;

use super::theme;

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
