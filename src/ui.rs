mod banner;
mod markdown;
mod palette;
mod panel;
mod plan;
mod sanitize;
mod table;
mod repl_helper;
mod spinner;
mod stream;
mod terminal;
mod theme;
mod usage_bar;

use colored::Colorize;

pub use banner::{play_startup_banner, play_startup_banner_blocking, BannerInfo};
pub use markdown::render_markdown;
pub use sanitize::{
    looks_like_reasoning_delta, sanitize_final, sanitize_stream_delta,
};
pub use table::render_table;
pub use plan::render_plan_text;
pub use repl_helper::{colored_input_prompt, input_prompt_plain, ReplHelper};
pub use spinner::frame as spinner_frame;
pub use stream::{ensure_terminal_ready, print_diff_preview, StreamRenderer};
pub use terminal::{set_cursor_bar, visible_width};

pub fn success(msg: &str) -> String {
    format!("{} {}", theme::success_icon(), theme::primary_light(msg))
}

pub fn error(msg: &str) -> String {
    format!("{} {}", theme::error_icon(), msg.red())
}

pub fn warn(msg: &str) -> String {
    format!("{} {}", theme::warn_icon(), theme::warning(msg))
}

pub fn block_warning(msg: &str) -> String {
    format!("{} {}", theme::warn_icon(), theme::warning(msg))
}

/// 工具调用卡片（视觉隔离，对齐 session 配图）
pub fn tool_call(name: &str, detail: &str) -> String {
    format!("\n{}\n", panel::render_tool_panel(name, detail, None))
}

/// 工具输出预览（卡片第二行）
pub fn tool_output_preview(text: &str) -> String {
    format!("\n{}\n", panel::render_tool_result(text))
}

pub fn reasoning_header() -> String {
    format!(
        "  {} {}",
        theme::thinking_icon(),
        theme::thinking_label("思考中")
    )
}

pub fn reasoning_text(text: &str) -> String {
    theme::thinking_detail(text)
}

pub fn reasoning_footer() -> String {
    format!(
        "  {} {}",
        theme::thinking_icon(),
        theme::separator_line(32)
    )
}

pub fn reasoning_fold_line(folded_count: usize) -> String {
    format!(
        "  {}",
        theme::muted(&format!("… (已折叠 {folded_count} 行)"))
    )
}

/// 折叠模式：单行 Spinner + 摘要
pub fn thinking_spinner_line(frame: usize, summary: &str) -> String {
    spinner_status_line(frame, theme::thinking_label("思考中"), summary)
}

/// 等待模型响应时的单行 Spinner
pub fn generating_spinner_line(frame: usize) -> String {
    spinner_status_line(frame, theme::primary_light("正在生成"), "…")
}

fn spinner_status_line(frame: usize, label: String, hint: &str) -> String {
    let spin = spinner::frame(frame);
    let hint = theme::muted_strong(hint);
    format!("  {} {} {hint}", spin, label)
}

pub fn diff_block(diff: &str) -> String {
    let inner = diff
        .lines()
        .map(|line| {
            if line.starts_with('+') && !line.starts_with("+++") {
                theme::diff_add(line)
            } else if line.starts_with('-') && !line.starts_with("---") {
                theme::diff_remove(line)
            } else {
                theme::muted_strong(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n  {} {} {}\n{}\n  {} {} {}\n",
        theme::soft(theme::border_top_left()),
        theme::border_horizontal().repeat(2),
        theme::muted("diff"),
        inner
            .lines()
            .map(|l| format!("  {} {l}", theme::soft(theme::border_vertical())))
            .collect::<Vec<_>>()
            .join("\n"),
        theme::soft(theme::border_bottom_left()),
        theme::border_horizontal().repeat(2),
        theme::separator_line(24)
    )
}

pub fn separator() -> String {
    theme::separator_line(48)
}

/// 任务流摘要独立块（STATUS SCREEN）
pub fn task_summary_block(summary: &str) -> String {
    let title = theme::panel_title("STATUS");
    let body = summary
        .trim()
        .lines()
        .map(|l| format!("  {} {l}", theme::soft(theme::border_vertical())))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n  {} {} {}\n  {} {}\n{body}\n  {} {} {}\n",
        theme::soft(theme::border_top_left()),
        theme::border_horizontal().repeat(42),
        theme::soft(theme::border_top_right()),
        theme::soft(theme::border_vertical()),
        title,
        theme::soft(theme::border_bottom_left()),
        theme::border_horizontal().repeat(42),
        theme::soft(theme::border_bottom_right()),
    )
}

pub fn assistant_prefix() -> String {
    theme::assistant_prompt_prefix()
}

pub fn user_prefix() -> String {
    theme::user_prompt_prefix()
}

pub fn dim(text: &str) -> String {
    theme::muted(text)
}

pub use usage_bar::{print_turn_usage, render_turn_usage};

pub use theme::{init_theme, set_theme, ThemeId};

pub fn usage_line(text: &str) -> String {
    format!("{} {}", theme::soft("⎿"), theme::muted_strong(text))
}

pub fn hint(text: &str) -> String {
    theme::primary_light(text)
}

pub fn section_title(text: &str) -> String {
    format!("\n{}\n", theme::primary(text))
}

pub fn status_pair(label: &str, value: &str) -> String {
    format!(
        "{} {}",
        theme::primary_light(&format!("{label}:")),
        theme::accent(value)
    )
}

/// 终端交互式列表选择（用于 REPL 斜杠命令等）
pub fn select_index(
    prompt: &str,
    items: &[String],
    default: usize,
) -> Result<usize, dialoguer::Error> {
    use dialoguer::{theme::ColorfulTheme, Select};
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(default.min(items.len().saturating_sub(1)))
        .interact()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    Allow,
    Deny,
    Always,
}

/// 权限确认：先打印配图风格警告面板，再列表选择
pub fn select_permission(tool_name: &str, detail: &str) -> Result<PermissionChoice, dialoguer::Error> {
    println!("{}", panel::render_permission_panel(tool_name, detail));
    let prompt = if detail.is_empty() {
        format!("允许执行 {tool_name}？")
    } else {
        format!("允许执行 {tool_name}？ ({detail})")
    };
    let items = vec![
        theme::choice_yes("允许"),
        theme::choice_default("拒绝"),
        theme::choice_always("始终允许"),
    ];
    let idx = select_index(&prompt, &items, 1)?;
    Ok(match idx {
        0 => PermissionChoice::Allow,
        2 => PermissionChoice::Always,
        _ => PermissionChoice::Deny,
    })
}

/// Bash 超时友好提示
pub fn bash_timeout_hint(command: &str) -> &'static str {
    let lower = command.to_lowercase();
    if lower.contains("plt.show")
        || lower.contains(".show()")
        || (lower.contains("matplotlib") && lower.contains("show"))
    {
        "检测到脚本因缺少 GUI 环境而超时。OneMini 建议改用 plt.savefig() 或非交互后端（Agg）。"
    } else if lower.contains("input(") || lower.contains("readline") {
        "命令可能在等待终端输入而阻塞。请改为非交互模式或传入参数。"
    } else {
        "命令执行超时。尝试缩小范围、拆分步骤，或检查是否因阻塞调用而挂起。"
    }
}
