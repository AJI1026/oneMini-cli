use colored::Colorize;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

use super::{table, theme};

/// 将 Markdown 渲染为带 ANSI 样式的终端文本（高对比度主题）。
pub fn render_markdown(input: &str) -> String {
    let cleaned = super::sanitize::sanitize_final(input);
    render_markdown_inner(&cleaned)
}

fn render_markdown_inner(input: &str) -> String {
    let mut out = String::new();
    let parser = Parser::new_ext(input, Options::all());

    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;
    let mut list_depth = 0usize;
    let mut link_url = String::new();
    let mut heading_level: Option<HeadingLevel> = None;

    let mut md_table: Option<MdTable> = None;
    let mut in_table_cell = false;
    let mut table_cell = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => heading_level = Some(level),
                Tag::List(_) => list_depth += 1,
                Tag::Item => {
                    out.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                    out.push_str(&format!("{} ", theme::list_bullet()));
                }
                Tag::BlockQuote(_) => out.push_str(&theme::soft(&format!("{} ", theme::border_vertical()))),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        let lang_str = lang.to_string();
                        if lang_str == "diff" {
                            out.push_str(&format!("{}\n", theme::primary_light("diff")));
                        } else if !lang_str.is_empty() {
                            out.push_str(&format!("{}\n", theme::muted(&lang_str)));
                        }
                    }
                }
                Tag::Link { dest_url, .. } => link_url = dest_url.to_string(),
                Tag::Strong => bold = true,
                Tag::Emphasis => italic = true,
                Tag::Table(_) => md_table = Some(MdTable::default()),
                Tag::TableHead => {
                    if let Some(t) = md_table.as_mut() {
                        t.in_head = true;
                        t.current_row.clear();
                    }
                }
                Tag::TableRow => {
                    if let Some(t) = md_table.as_mut() {
                        t.current_row.clear();
                    }
                    table_cell.clear();
                }
                Tag::TableCell => {
                    in_table_cell = true;
                    table_cell.clear();
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    heading_level = None;
                    out.push('\n');
                }
                TagEnd::Paragraph => {
                    if !in_table_cell {
                        out.push('\n');
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    out.push('\n');
                }
                TagEnd::List(_) => list_depth = list_depth.saturating_sub(1),
                TagEnd::Item => out.push('\n'),
                TagEnd::Link => {
                    if !link_url.is_empty() {
                        out.push_str(&format!(
                            " ({})",
                            theme::primary_light(&link_url)
                        ));
                        link_url.clear();
                    }
                }
                TagEnd::Strong => bold = false,
                TagEnd::Emphasis => italic = false,
                TagEnd::TableHead => {
                    if let Some(t) = md_table.as_mut() {
                        t.headers = std::mem::take(&mut t.current_row);
                        t.in_head = false;
                    }
                }
                TagEnd::TableCell => {
                    in_table_cell = false;
                    if let Some(t) = md_table.as_mut() {
                        t.current_row.push(std::mem::take(&mut table_cell));
                    }
                }
                TagEnd::TableRow => {
                    if let Some(t) = md_table.as_mut() {
                        if !t.in_head {
                            t.rows.push(std::mem::take(&mut t.current_row));
                        }
                    }
                }
                TagEnd::Table => {
                    if let Some(t) = md_table.take() {
                        out.push('\n');
                        out.push_str(&t.render());
                        out.push('\n');
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_table_cell {
                    table_cell.push_str(&style_text(&text, bold, italic, false, None));
                } else {
                    out.push_str(&style_text(&text, bold, italic, in_code_block, heading_level));
                }
            }
            Event::Code(text) => {
                let styled = format!("`{}`", theme::primary_light(&text));
                if in_table_cell {
                    table_cell.push_str(&styled);
                } else {
                    out.push_str(&styled);
                }
            }
            Event::SoftBreak => {
                if in_table_cell {
                    table_cell.push(' ');
                } else {
                    out.push(' ');
                }
            }
            Event::HardBreak => {
                if in_table_cell {
                    table_cell.push(' ');
                } else {
                    out.push('\n');
                }
            }
            Event::Rule => {
                out.push_str(&format!("{}\n", theme::separator_line(40)));
            }
            _ => {}
        }
    }

    out.trim_end().to_string()
}

#[derive(Default)]
struct MdTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    in_head: bool,
}

impl MdTable {
    fn render(&self) -> String {
        if self.headers.is_empty() {
            return String::new();
        }
        let header_refs: Vec<&str> = self.headers.iter().map(String::as_str).collect();
        table::render_table(&header_refs, &self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn bold_inline() {
        let rendered = render_markdown("我是 **OneMini CLI**，助手。");
        assert!(!rendered.contains("**"));
        assert!(rendered.contains("OneMini CLI"));
    }

    #[test]
    fn table_renders_without_pipes() {
        let md = "| 列A | 列B |\n| --- | --- |\n| 1 | 2 |";
        let rendered = render_markdown(md);
        assert!(!rendered.contains('|'));
        assert!(rendered.contains('─'));
        assert!(rendered.contains('1'));
    }

    #[test]
    fn table_renders_cjk_weather_rows() {
        let md = "\
| 时段 | 天气 | 温度 |\n\
| --- | --- | --- |\n\
| 🌅 早上 | 烟霾 | 23°C |\n\
| ☀️ 中午 | 局部阵雨 | 29°C |";
        let rendered = render_markdown(md);
        assert!(!rendered.contains('|'));
        assert!(rendered.contains("🌅 早上"));
        assert!(rendered.contains("局部阵雨"));
    }

    #[test]
    fn table_cell_bold_renders_without_markers() {
        let md = "| 项目 | 详情 |\n| --- | --- |\n| **当前天气** | 多云间晴 |";
        let rendered = render_markdown(md);
        assert!(!rendered.contains("**"));
        assert!(rendered.contains("当前天气"));
        assert!(rendered.contains("多云间晴"));
    }

    #[test]
    fn heading_bold_renders_without_markers() {
        let rendered = render_markdown("**成都今日天气（2026年6月18日）**");
        assert!(!rendered.contains("**"));
        assert!(rendered.contains("成都今日天气"));
    }
}

fn style_text(
    text: &str,
    bold: bool,
    italic: bool,
    in_code_block: bool,
    heading: Option<HeadingLevel>,
) -> String {
    if in_code_block {
        if text.starts_with('+') {
            return theme::diff_add(text);
        }
        if text.starts_with('-') {
            return theme::diff_remove(text);
        }
        return theme::muted_strong(text);
    }

    let mut styled = text.to_string();
    if let Some(level) = heading {
        styled = match level {
            HeadingLevel::H1 => theme::primary(&styled),
            HeadingLevel::H2 => theme::primary_light(&styled),
            HeadingLevel::H3 | HeadingLevel::H4 => theme::accent(&styled),
            _ => theme::muted_strong(&styled),
        };
        return styled;
    }

    if bold && italic {
        if theme::colors_enabled() {
            styled.bright_cyan().bold().italic().to_string()
        } else {
            styled
        }
    } else if bold {
        theme::primary_light(&styled)
    } else if italic {
        theme::thinking_detail(&styled)
    } else {
        styled
    }
}
