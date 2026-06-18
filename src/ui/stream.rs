use std::io::{self, Write};

use crate::ui::{self, has_markdown_structure, sanitize_final, sanitize_stream_delta};

/// 思考区最多展示的行数（不含标题与折叠提示）
const MAX_VISIBLE_REASONING_LINES: usize = 3;
/// 单行展示的最大字符数
const MAX_REASONING_LINE_CHARS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamPhase {
    Idle,
    Reasoning,
    Content,
}

pub struct StreamRenderer {
    phase: StreamPhase,
    reasoning_buf: String,
    content_buf: String,
    content_header: bool,
    /// true = 展开样式；false = 折叠样式（仍只显示最新几行）
    show_reasoning: bool,
    spinner_frame: usize,
    /// 上次绘制的思考块占用的终端行数
    reasoning_lines_drawn: usize,
}

impl StreamRenderer {
    pub fn new(show_reasoning: bool) -> Self {
        Self {
            phase: StreamPhase::Idle,
            reasoning_buf: String::new(),
            content_buf: String::new(),
            content_header: false,
            show_reasoning,
            spinner_frame: 0,
            reasoning_lines_drawn: 0,
        }
    }

    pub fn on_reasoning_delta(&mut self, delta: &str) {
        if self.phase == StreamPhase::Content {
            self.finish_content_raw();
        }
        self.phase = StreamPhase::Reasoning;
        self.reasoning_buf.push_str(delta);
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
        if self.phase == StreamPhase::Reasoning {
            self.finish_reasoning();
        }
        self.phase = StreamPhase::Content;
        if !self.content_header {
            print!("{} ", ui::assistant_prefix());
            self.content_header = true;
        }
        print!("{cleaned}");
        self.content_buf.push_str(&cleaned);
        io::stdout().flush().ok();
    }

    pub fn on_tool_call(&mut self, name: &str, detail: &str) {
        self.finish_active();
        println!("{}", ui::tool_call(name, detail));
    }

    pub fn finish(&mut self, final_content: Option<&str>) {
        self.finish_active();
        let raw = final_content.unwrap_or(&self.content_buf);
        let content = sanitize_final(raw);
        if !content.is_empty() && self.content_header {
            println!();
            if has_markdown_structure(&content) {
                print!("\x1b[2K\r");
                println!(
                    "{} {}",
                    ui::assistant_prefix(),
                    ui::render_markdown(&content)
                );
            }
        } else if !content.is_empty() && !self.content_header {
            println!(
                "{} {}",
                ui::assistant_prefix(),
                ui::render_markdown(&content)
            );
        }
        self.reset();
    }

    pub fn finish_tool_round(&mut self) {
        self.finish_active();
        self.reset();
    }

    fn finish_active(&mut self) {
        match self.phase {
            StreamPhase::Reasoning => self.finish_reasoning(),
            StreamPhase::Content => self.finish_content_raw(),
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

    fn finish_content_raw(&mut self) {
        if self.content_header {
            println!();
        }
        self.content_buf.clear();
        self.content_header = false;
        if self.phase == StreamPhase::Content {
            self.phase = StreamPhase::Idle;
        }
    }

    fn reset(&mut self) {
        self.phase = StreamPhase::Idle;
        self.reasoning_buf.clear();
        self.content_buf.clear();
        self.content_header = false;
        self.reasoning_lines_drawn = 0;
        self.spinner_frame = 0;
    }

    fn redraw_reasoning(&mut self, frozen: bool) {
        let lines = build_reasoning_display(
            &self.reasoning_buf,
            self.show_reasoning,
            self.spinner_frame,
            frozen,
        );
        self.reasoning_lines_drawn = redraw_lines(self.reasoning_lines_drawn, &lines);
    }
}

fn build_reasoning_display(
    buf: &str,
    show_reasoning: bool,
    spinner_frame: usize,
    frozen: bool,
) -> Vec<String> {
    let content_lines = normalize_reasoning_lines(buf);
    let total = content_lines.len();
    let folded = total.saturating_sub(MAX_VISIBLE_REASONING_LINES);

    let mut out = Vec::new();

    if show_reasoning || frozen {
        out.push(ui::reasoning_header());
    } else {
        out.push(ui::thinking_spinner_line(spinner_frame, ""));
    }

    if folded > 0 {
        out.push(ui::reasoning_fold_line(folded));
    }

    let visible: &[String] = if total <= MAX_VISIBLE_REASONING_LINES {
        &content_lines
    } else {
        &content_lines[total - MAX_VISIBLE_REASONING_LINES..]
    };

    if visible.is_empty() {
        let hint = if show_reasoning {
            ui::reasoning_text("正在分析…")
        } else {
            ui::thinking_spinner_line(spinner_frame, "正在分析…")
        };
        if show_reasoning || frozen {
            out.push(format!("  {hint}"));
        } else {
            out[0] = hint;
        }
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

/// 原地重绘多行块；返回新行数。光标回到块首行，便于下次更新。
fn redraw_lines(prev_line_count: usize, lines: &[String]) -> usize {
    if prev_line_count > 0 {
        print!("\x1b[{prev_line_count}A");
    }

    let new_count = lines.len();
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

pub fn print_diff_preview(diff: &str) {
    if diff.trim().is_empty() {
        return;
    }
    println!("{}", ui::diff_block(diff));
}

#[cfg(test)]
mod tests {
    use super::{build_reasoning_display, normalize_reasoning_lines, MAX_VISIBLE_REASONING_LINES};

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
    fn display_folds_when_many_lines() {
        let buf = (1..=8).map(|i| format!("line{i}")).collect::<Vec<_>>().join("\n");
        let display = build_reasoning_display(&buf, false, 0, false);
        assert!(
            display
                .iter()
                .any(|l| l.contains(&format!("已折叠 {}", 8 - MAX_VISIBLE_REASONING_LINES)))
        );
        assert!(display.iter().any(|l| l.contains("line8")));
        assert!(!display.iter().any(|l| l.contains("line1")));
    }

    #[test]
    fn display_shows_all_when_few_lines() {
        let buf = "a\nb\nc";
        let display = build_reasoning_display(buf, true, 0, false);
        assert!(!display.iter().any(|l| l.contains("已折叠")));
        assert!(display.iter().any(|l| l.contains('a')));
        assert!(display.iter().any(|l| l.contains('c')));
    }
}
