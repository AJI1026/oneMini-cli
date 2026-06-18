//! 单轮 Token 用量与上下文进度条

use colored::Colorize;

use crate::llm::UsageInfo;
use crate::usage::{ProgressBarView, TurnUsageView};

use super::theme;

const BAR_WIDTH: usize = 24;

/// 渲染单轮用量（统计行 + 进度条）
pub fn render_turn_usage(usage: &UsageInfo, model: &str, context_tokens: u32) -> String {
    let view = TurnUsageView::new(usage, model, context_tokens);
    let stats = view.stats_line();
    let bar = view.progress_bar(BAR_WIDTH);
    format!(
        "{}\n{}",
        super::usage_line(&format!("Token  {stats}")),
        render_progress_line(&bar)
    )
}

fn render_progress_line(bar: &ProgressBarView) -> String {
    let colored_bar = colorize_bar(bar);
    format!(
        "  {} {} {}",
        theme::primary_light(bar.label),
        colored_bar,
        theme::muted_strong(&bar.caption())
    )
}

fn colorize_bar(bar: &ProgressBarView) -> String {
    if !theme::colors_enabled() {
        return bar.bar_plain();
    }
    let filled = bar.filled_count();
    let empty = bar.width.saturating_sub(filled);
    let fill_color = if bar.percent() >= 85 {
        |s: &str| s.red().bold()
    } else if bar.percent() >= 60 {
        |s: &str| s.yellow().bold()
    } else {
        |s: &str| s.bright_cyan().bold()
    };
    format!(
        "[{}{}]",
        fill_color(&"█".repeat(filled)),
        theme::soft(&"░".repeat(empty))
    )
}

pub fn print_turn_usage(usage: &UsageInfo, model: &str, context_tokens: u32) {
    if usage.total() == 0 {
        return;
    }
    println!("{}", render_turn_usage(usage, model, context_tokens));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_stats_and_bar() {
        let usage = UsageInfo {
            prompt_tokens: 12_000,
            completion_tokens: 800,
            reasoning_tokens: 200,
            total_tokens: 0,
        };
        let out = render_turn_usage(&usage, "deepseek-chat", 12_000);
        assert!(out.contains("Token"));
        assert!(out.contains("输入"));
        assert!(out.contains('█'));
        assert!(out.contains("上下文"));
    }
}
