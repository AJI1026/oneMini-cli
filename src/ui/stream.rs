use std::io::{self, Write};

use crate::ui;

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
    reasoning_header: bool,
    content_header: bool,
    show_reasoning: bool,
}

impl StreamRenderer {
    pub fn new(show_reasoning: bool) -> Self {
        Self {
            phase: StreamPhase::Idle,
            reasoning_buf: String::new(),
            content_buf: String::new(),
            reasoning_header: false,
            content_header: false,
            show_reasoning,
        }
    }

    pub fn on_reasoning_delta(&mut self, delta: &str) {
        if !self.show_reasoning {
            return;
        }
        if self.phase == StreamPhase::Content {
            self.finish_content_raw();
        }
        self.phase = StreamPhase::Reasoning;
        if !self.reasoning_header {
            println!("{}", ui::reasoning_header());
            self.reasoning_header = true;
        }
        print!("{}", ui::reasoning_text(delta));
        self.reasoning_buf.push_str(delta);
        io::stdout().flush().ok();
    }

    pub fn on_content_delta(&mut self, delta: &str) {
        if self.phase == StreamPhase::Reasoning {
            self.finish_reasoning();
        }
        self.phase = StreamPhase::Content;
        if !self.content_header {
            print!("{} ", ui::assistant_prefix());
            self.content_header = true;
        }
        print!("{delta}");
        self.content_buf.push_str(delta);
        io::stdout().flush().ok();
    }

    pub fn on_tool_call(&mut self, name: &str, detail: &str) {
        self.finish_active();
        println!("{}", ui::tool_call(name, detail));
    }

    pub fn finish(&mut self, final_content: Option<&str>) {
        self.finish_active();
        let content = final_content.unwrap_or(&self.content_buf);
        if !content.is_empty() && self.content_header {
            // 流式已输出原文，换行收尾；复杂 markdown 在最终渲染
            println!();
            if content.contains('#') || content.contains("**") || content.contains("```") {
                print!("\x1b[2K\r");
                println!(
                    "{} {}",
                    ui::assistant_prefix(),
                    ui::render_markdown(content)
                );
            }
        } else if !content.is_empty() && !self.content_header {
            println!(
                "{} {}",
                ui::assistant_prefix(),
                ui::render_markdown(content)
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
        if self.reasoning_header {
            println!();
            println!("{}", ui::reasoning_footer());
        }
        self.reasoning_buf.clear();
        self.reasoning_header = false;
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
        self.reasoning_header = false;
        self.content_header = false;
    }
}

pub fn print_usage_line(line: &str) {
    println!("{}", ui::usage_line(line));
}

pub fn print_diff_preview(diff: &str) {
    if diff.trim().is_empty() {
        return;
    }
    println!("{}", ui::diff_block(diff));
}
