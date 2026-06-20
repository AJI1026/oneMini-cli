use std::io::{self, Write};

use crate::ui::{
    self, render_markdown, sanitize_final, sanitize_stream_delta,
    terminal::{clear_visual_rows, terminal_width, total_visual_rows, truncate_visible, visual_rows},
};

/// 思考区最多展示的行数（不含标题与折叠提示，仅展开模式）
const MAX_VISIBLE_REASONING_LINES: usize = 3;
/// 单行展示的最大字符数
const MAX_REASONING_LINE_CHARS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamPhase {
    Idle,
    /// 等待首个 token（API 响应前）
    Waiting,
    Reasoning,
    Content,
}

pub struct StreamRenderer {
    phase: StreamPhase,
    reasoning_buf: String,
    content_buf: String,
    content_header: bool,
    /// 流式正文已占用的终端行数（用于 finish 时原地重绘 Markdown）
    content_lines_drawn: usize,
    /// true = 展开样式；false = 不显示思考过程
    show_reasoning: bool,
    spinner_frame: usize,
    /// 上次绘制的思考块占用的终端可视行数（含软换行）
    reasoning_lines_drawn: usize,
    /// 等待动画占用的终端可视行数
    waiting_lines_drawn: usize,
}

impl StreamRenderer {
    pub fn new(show_reasoning: bool) -> Self {
        Self {
            phase: StreamPhase::Idle,
            reasoning_buf: String::new(),
            content_buf: String::new(),
            content_header: false,
            content_lines_drawn: 0,
            show_reasoning,
            spinner_frame: 0,
            reasoning_lines_drawn: 0,
            waiting_lines_drawn: 0,
        }
    }

    /// 流式回合开始：显示「正在生成」等待动画
    pub fn begin_waiting(&mut self) {
        self.phase = StreamPhase::Waiting;
        self.spinner_frame = 0;
        self.redraw_waiting();
        io::stdout().flush().ok();
    }

    /// 定时刷新 Spinner（等待首 token 或折叠思考模式）
    pub fn tick(&mut self) {
        match self.phase {
            StreamPhase::Waiting => {
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
                self.redraw_waiting();
                io::stdout().flush().ok();
            }
            StreamPhase::Reasoning if !self.show_reasoning => {
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
                self.redraw_reasoning(false);
                io::stdout().flush().ok();
            }
            _ => {}
        }
    }

    pub fn on_reasoning_delta(&mut self, delta: &str) {
        if self.phase == StreamPhase::Content {
            return;
        }
        self.clear_waiting();
        self.phase = StreamPhase::Reasoning;
        self.reasoning_buf.push_str(delta);
        if !self.show_reasoning {
            return;
        }
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
        self.redraw_reasoning(false);
        io::stdout().flush().ok();
    }

    pub fn on_content_delta(&mut self, delta: &str) {
        if ui::looks_like_reasoning_delta(delta) {
            self.on_reasoning_delta(delta);
            return;
        }
        let cleaned = sanitize_stream_delta(delta);
        if cleaned.is_empty() {
            return;
        }
        self.clear_waiting();
        if self.phase == StreamPhase::Reasoning {
            self.finish_reasoning();
        }
        self.phase = StreamPhase::Content;
        self.content_buf.push_str(&cleaned);

        if !self.content_header {
            print!("{} ", ui::assistant_prefix());
            self.content_header = true;
        }
        print!("{cleaned}");
        self.content_lines_drawn = streamed_content_visual_rows(&self.content_buf);

        io::stdout().flush().ok();
    }

    pub fn on_tool_call(&mut self, name: &str, detail: &str) {
        self.clear_waiting();
        self.finish_active();
        println!("{}", ui::tool_call(name, detail));
    }

    pub fn finish(&mut self, final_content: Option<&str>) {
        self.clear_waiting();
        self.finish_active();
        self.force_clear_reasoning_ui();

        if self.content_header {
            let raw = final_content.unwrap_or(&self.content_buf);
            if !raw.trim().is_empty() {
                let lines = content_display_lines(&render_markdown(raw));
                clear_visual_rows(self.content_lines_drawn);
                print_rendered_content(&lines);
            }
            println!();
        } else {
            let raw = final_content.unwrap_or(&self.content_buf);
            let content = sanitize_final(raw);
            if !content.is_empty() {
                let lines = content_display_lines(&render_markdown(&content));
                print_rendered_content(&lines);
                println!();
            }
        }

        self.reset();
        io::stdout().flush().ok();
    }

    pub fn finish_tool_round(&mut self) {
        self.clear_waiting();
        self.finish_active();
        self.force_clear_reasoning_ui();
        self.reset();
        io::stdout().flush().ok();
    }

    fn finish_active(&mut self) {
        match self.phase {
            StreamPhase::Waiting => {
                self.clear_waiting();
                self.phase = StreamPhase::Idle;
            }
            StreamPhase::Reasoning => self.finish_reasoning(),
            StreamPhase::Content => {
                self.phase = StreamPhase::Idle;
            }
            StreamPhase::Idle => {}
        }
    }

    fn finish_reasoning(&mut self) {
        if self.reasoning_lines_drawn > 0 {
            if self.show_reasoning {
                self.redraw_reasoning(true);
                println!();
                println!("{}", ui::reasoning_footer());
            } else {
                clear_reasoning_block(self.reasoning_lines_drawn);
            }
            io::stdout().flush().ok();
        }
        self.reasoning_buf.clear();
        self.reasoning_lines_drawn = 0;
        if self.phase == StreamPhase::Reasoning {
            self.phase = StreamPhase::Idle;
        }
    }

    fn force_clear_reasoning_ui(&mut self) {
        if self.reasoning_lines_drawn > 0 {
            clear_reasoning_block(self.reasoning_lines_drawn);
            self.reasoning_lines_drawn = 0;
            self.reasoning_buf.clear();
            io::stdout().flush().ok();
        }
    }

    fn reset(&mut self) {
        self.phase = StreamPhase::Idle;
        self.reasoning_buf.clear();
        self.content_buf.clear();
        self.content_header = false;
        self.content_lines_drawn = 0;
        self.reasoning_lines_drawn = 0;
        self.waiting_lines_drawn = 0;
        self.spinner_frame = 0;
    }

    fn redraw_waiting(&mut self) {
        let line = ui::generating_spinner_line(self.spinner_frame);
        redraw_single_line(&mut self.waiting_lines_drawn, &line);
    }

    fn clear_waiting(&mut self) {
        if self.waiting_lines_drawn > 0 {
            clear_reasoning_block(self.waiting_lines_drawn);
            self.waiting_lines_drawn = 0;
            io::stdout().flush().ok();
        }
    }

    fn redraw_reasoning(&mut self, frozen: bool) {
        if self.show_reasoning || frozen {
            let lines = build_reasoning_display(
                &self.reasoning_buf,
                self.show_reasoning,
                self.spinner_frame,
                frozen,
            );
            self.reasoning_lines_drawn = redraw_lines(self.reasoning_lines_drawn, &lines);
        } else {
            let line = build_collapsed_spinner_line(&self.reasoning_buf, self.spinner_frame);
            redraw_single_line(&mut self.reasoning_lines_drawn, &line);
        }
    }
}

fn content_display_lines(rendered: &str) -> Vec<String> {
    let prefix = format!("{} ", ui::assistant_prefix());
    let rendered_lines: Vec<&str> = rendered.lines().collect();
    if rendered_lines.is_empty() {
        return vec![prefix];
    }
    let mut out = Vec::with_capacity(rendered_lines.len());
    for (i, line) in rendered_lines.iter().enumerate() {
        if i == 0 {
            out.push(format!("{prefix}{line}"));
        } else {
            out.push((*line).to_string());
        }
    }
    out
}

fn print_rendered_content(lines: &[String]) {
    for (i, line) in lines.iter().enumerate() {
        print!("\x1b[2K\r{line}");
        if i + 1 < lines.len() {
            println!();
        }
    }
}

/// 流式正文占用的终端可视行数（首行含助手前缀，含软换行）
fn streamed_content_visual_rows(content: &str) -> usize {
    if content.is_empty() {
        return 0;
    }
    let term_w = terminal_width();
    let prefix = format!("{} ", ui::assistant_prefix());
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return visual_rows(&format!("{prefix}{content}"), term_w);
    }
    let mut total = visual_rows(&format!("{prefix}{}", lines[0]), term_w);
    for line in &lines[1..] {
        total += visual_rows(line, term_w);
    }
    total
}

fn build_collapsed_spinner_line(buf: &str, spinner_frame: usize) -> String {
    let prefix_width = ui::visible_width(&ui::thinking_spinner_line(spinner_frame, ""));
    let max_summary_cols = terminal_width().saturating_sub(prefix_width);
    let summary = collapsed_reasoning_summary(buf, max_summary_cols);
    if summary.is_empty() {
        ui::thinking_spinner_line(spinner_frame, "正在分析…")
    } else {
        ui::thinking_spinner_line(spinner_frame, &summary)
    }
}

fn collapsed_reasoning_summary(buf: &str, max_cols: usize) -> String {
    let t = buf.trim();
    if t.is_empty() || max_cols == 0 {
        return String::new();
    }
    let last = t.lines().last().unwrap_or(t).trim();
    truncate_visible(last, max_cols)
}

fn build_reasoning_display(
    buf: &str,
    show_reasoning: bool,
    spinner_frame: usize,
    frozen: bool,
) -> Vec<String> {
    if !show_reasoning && !frozen {
        return vec![build_collapsed_spinner_line(buf, spinner_frame)];
    }

    let content_lines = normalize_reasoning_lines(buf);
    let total = content_lines.len();
    let folded = total.saturating_sub(MAX_VISIBLE_REASONING_LINES);

    let mut out = Vec::new();
    out.push(ui::reasoning_header());

    if folded > 0 {
        out.push(ui::reasoning_fold_line(folded));
    }

    let visible: &[String] = if total <= MAX_VISIBLE_REASONING_LINES {
        &content_lines
    } else {
        &content_lines[total - MAX_VISIBLE_REASONING_LINES..]
    };

    if visible.is_empty() {
        out.push(format!("  {}", ui::reasoning_text("正在分析…")));
    } else {
        for line in visible {
            out.push(format!("  {}", ui::reasoning_text(line)));
        }
    }

    out
}

fn normalize_reasoning_lines(buf: &str) -> Vec<String> {
    let mut lines: Vec<String> = if buf.is_empty() {
        Vec::new()
    } else {
        buf.lines().map(String::from).collect()
    };

    if lines.is_empty() && !buf.is_empty() {
        lines.push(buf.to_string());
    }

    for line in &mut lines {
        if line.chars().count() > MAX_REASONING_LINE_CHARS {
            let tail: String = line
                .chars()
                .rev()
                .take(MAX_REASONING_LINE_CHARS)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            *line = format!("…{tail}");
        }
    }

    lines
}

/// 折叠模式：单行原地重绘（按终端可视行数清除，避免软换行残留）
fn redraw_single_line(prev_rows: &mut usize, line: &str) {
    clear_visual_rows(*prev_rows);
    print!("\x1b[2K\r{line}");
    *prev_rows = visual_rows(line, terminal_width());
}

/// 原地重绘多行块（仅用于思考区展开模式）
fn redraw_lines(prev_rows: usize, lines: &[String]) -> usize {
    let term_w = terminal_width();
    clear_visual_rows(prev_rows);

    if lines.is_empty() {
        print!("\x1b[2K\r");
        return 1;
    }

    for (i, line) in lines.iter().enumerate() {
        print!("\x1b[2K\r{line}");
        if i + 1 < lines.len() {
            println!();
        }
    }

    total_visual_rows(lines, term_w)
}

fn clear_reasoning_block(rows: usize) {
    clear_visual_rows(rows);
}

/// 流式输出结束后整理终端，避免残留 spinner 与 readline 提示符重叠
pub fn ensure_terminal_ready() {
    let _ = io::stdout().flush();
}

pub fn print_diff_preview(diff: &str) {
    if diff.trim().is_empty() {
        return;
    }
    println!("{}", ui::diff_block(diff));
}

#[cfg(test)]
mod tests {
    use super::{
        build_collapsed_spinner_line, build_reasoning_display, content_display_lines,
        normalize_reasoning_lines, streamed_content_visual_rows, MAX_VISIBLE_REASONING_LINES,
    };

    #[test]
    fn normalize_empty_buf() {
        assert!(normalize_reasoning_lines("").is_empty());
    }

    #[test]
    fn normalize_single_line_without_newline() {
        let lines = normalize_reasoning_lines("hello");
        assert_eq!(lines, vec!["hello".to_string()]);
    }

    #[test]
    fn collapsed_mode_is_single_line() {
        let buf = (1..=8)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let display = build_reasoning_display(&buf, false, 0, false);
        assert_eq!(display.len(), 1);
        assert!(display[0].contains("思考中"));
        assert!(display[0].contains("line8"));
    }

    #[test]
    fn collapsed_spinner_uses_latest_line() {
        let line = build_collapsed_spinner_line("first\nsecond line", 0);
        assert!(line.contains("second line"));
        assert!(!line.contains("first"));
    }

    #[test]
    fn display_folds_when_many_lines_expanded() {
        let buf = (1..=8)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let display = build_reasoning_display(&buf, true, 0, false);
        assert!(
            display
                .iter()
                .any(|l| l.contains(&format!("已折叠 {}", 8 - MAX_VISIBLE_REASONING_LINES)))
        );
        assert!(display.iter().any(|l| l.contains("line8")));
        assert!(!display.iter().any(|l| l.contains("line1")));
    }

    #[test]
    fn display_shows_all_when_few_lines_expanded() {
        let buf = "a\nb\nc";
        let display = build_reasoning_display(buf, true, 0, false);
        assert!(!display.iter().any(|l| l.contains("已折叠")));
        assert!(display.iter().any(|l| l.contains('a')));
        assert!(display.iter().any(|l| l.contains('c')));
    }

    #[test]
    fn content_lines_prefix_first_line_only() {
        let _g = crate::ui::theme::theme_test_guard();
        crate::ui::set_theme(crate::ui::ThemeId::Modern);
        let lines = content_display_lines("第一行\n第二行");
        assert!(lines[0].contains("onemini"));
        assert!(lines[0].contains('第'));
        assert!(!lines[1].contains("onemini"));
        assert_eq!(lines[1], "第二行");
    }

    #[test]
    fn streamed_content_rows_counts_wrapped_first_line() {
        let _g = crate::ui::theme::theme_test_guard();
        crate::ui::set_theme(crate::ui::ThemeId::Modern);
        let long = "a".repeat(120);
        assert!(streamed_content_visual_rows(&long) >= 2);
    }

    #[test]
    fn streamed_content_rows_counts_newlines() {
        let _g = crate::ui::theme::theme_test_guard();
        crate::ui::set_theme(crate::ui::ThemeId::Modern);
        assert_eq!(streamed_content_visual_rows(""), 0);
        assert_eq!(streamed_content_visual_rows("单行"), 1);
        assert!(streamed_content_visual_rows("a\nb\nc") >= 3);
    }
}
