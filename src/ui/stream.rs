use std::io::{self, Write};

use crate::ui::{self, render_markdown, sanitize_final, sanitize_stream_delta};

/// 思考区最多展示的行数（不含标题与折叠提示，仅展开模式）
const MAX_VISIBLE_REASONING_LINES: usize = 3;
/// 单行展示的最大字符数
const MAX_REASONING_LINE_CHARS: usize = 100;
/// 折叠模式摘要最大字符数
const COLLAPSED_SUMMARY_CHARS: usize = 48;

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
    /// 上次绘制的思考块占用的终端行数
    reasoning_lines_drawn: usize,
    /// 等待动画占用的终端行数
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
        self.content_lines_drawn = count_content_lines(&self.content_buf);

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
                clear_content_block(self.content_lines_drawn);
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

/// 流式正文行数（首行含助手前缀，后续按换行计）
fn count_content_lines(content: &str) -> usize {
    if content.is_empty() {
        0
    } else {
        1 + content.matches('\n').count()
    }
}

/// 清除流式输出的原始 Markdown 块，便于 finish 时原地重绘
fn clear_content_block(line_count: usize) {
    if line_count == 0 {
        return;
    }
    print!("\r");
    for i in 0..line_count {
        print!("\x1b[2K");
        if i + 1 < line_count {
            print!("\x1b[1A");
        }
    }
}

fn build_collapsed_spinner_line(buf: &str, spinner_frame: usize) -> String {
    let summary = collapsed_reasoning_summary(buf);
    if summary.is_empty() {
        ui::thinking_spinner_line(spinner_frame, "正在分析…")
    } else {
        ui::thinking_spinner_line(spinner_frame, &summary)
    }
}

fn collapsed_reasoning_summary(buf: &str) -> String {
    let t = buf.trim();
    if t.is_empty() {
        return String::new();
    }
    let last = t.lines().last().unwrap_or(t).trim();
    if last.chars().count() <= COLLAPSED_SUMMARY_CHARS {
        last.to_string()
    } else {
        format!(
            "{}…",
            last.chars().take(COLLAPSED_SUMMARY_CHARS).collect::<String>()
        )
    }
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

/// 折叠模式：始终单行 `\r` 重绘，避免光标上移破坏 readline 与历史输出
fn redraw_single_line(prev_line_count: &mut usize, line: &str) {
    if *prev_line_count > 0 {
        print!("\x1b[{}A", *prev_line_count);
    }
    print!("\x1b[2K\r{line}");
    *prev_line_count = 1;
}

/// 原地重绘多行块（仅用于思考区展开模式）
fn redraw_lines(prev_line_count: usize, lines: &[String]) -> usize {
    if prev_line_count > 0 {
        print!("\x1b[{prev_line_count}A");
    }

    let new_count = lines.len().max(1);
    for (i, line) in lines.iter().enumerate() {
        print!("\x1b[2K\r{line}");
        if i + 1 < new_count {
            println!();
        }
    }

    if prev_line_count > new_count {
        for _ in new_count..prev_line_count {
            print!("\x1b[2K\r\n");
        }
        print!("\x1b[{}A", prev_line_count - new_count);
    }

    if new_count > 1 {
        print!("\x1b[{}A", new_count - 1);
    }

    new_count
}

fn clear_reasoning_block(line_count: usize) {
    if line_count == 0 {
        return;
    }
    print!("\x1b[{line_count}A");
    for i in 0..line_count {
        print!("\x1b[2K\r");
        if i + 1 < line_count {
            println!();
        }
    }
    print!("\x1b[{line_count}A");
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
        count_content_lines, normalize_reasoning_lines, MAX_VISIBLE_REASONING_LINES,
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
        let lines = content_display_lines("第一行\n第二行");
        assert!(lines[0].contains("OneMini"));
        assert!(lines[0].contains('第'));
        assert!(!lines[1].contains("OneMini"));
        assert_eq!(lines[1], "第二行");
    }

    #[test]
    fn count_content_lines_handles_newlines() {
        assert_eq!(count_content_lines(""), 0);
        assert_eq!(count_content_lines("单行"), 1);
        assert_eq!(count_content_lines("a\nb\nc"), 3);
    }
}
