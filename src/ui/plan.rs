//! 为 /plan、/status 等纯文本输出着色

use colored::Colorize;

use super::theme;

pub fn render_plan_text(text: &str) -> String {
    text.lines()
        .map(color_plan_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn color_plan_line(line: &str) -> String {
    if line.starts_with("任务目标:") {
        let goal = line.trim_start_matches("任务目标:").trim();
        return format!("{} {}", theme::primary("任务目标:"), theme::accent(goal));
    }
    if line.starts_with("计划步骤:") || line.starts_with("验证状态:") {
        return theme::primary(line);
    }
    if line.contains("[✓]") {
        return line.replace("[✓]", &theme::primary_light("[✓]"));
    }
    if line.contains("[→]") {
        return line.replace("[→]", &theme::accent("[→]"));
    }
    if line.contains("[✗]") {
        return line.replace("[✗]", &theme::error_icon());
    }
    if line.contains("← 当前") {
        return line.replace("← 当前", &theme::accent("← 当前"));
    }
    if line.contains("[通过]") {
        return line.replace("[通过]", &theme::primary_light("[通过]"));
    }
    if line.contains("[失败]") {
        return line.replace("[失败]", &"[失败]".red().to_string());
    }
    if line.starts_with("  - ") {
        return format!(
            "  {} {}",
            theme::soft("-"),
            theme::muted(line.trim_start_matches("  - "))
        );
    }
    if line.starts_with("  [") || line.starts_with('[') {
        return theme::muted(line);
    }
    line.to_string()
}
