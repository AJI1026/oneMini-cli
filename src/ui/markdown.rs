use colored::Colorize;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

use super::theme;

/// 将 Markdown 渲染为带 ANSI 样式的终端文本（蓝色主题）。
pub fn render_markdown(input: &str) -> String {
    let mut out = String::new();
    let parser = Parser::new_ext(input, Options::all());

    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;
    let mut list_depth = 0usize;
    let mut link_url = String::new();
    let mut heading_level: Option<HeadingLevel> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => heading_level = Some(level),
                Tag::List(_) => list_depth += 1,
                Tag::Item => {
                    out.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                    out.push_str(&format!("{} ", theme::accent("•")));
                }
                Tag::BlockQuote(_) => out.push_str(&theme::soft("│ ")),
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
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    heading_level = None;
                    out.push('\n');
                }
                TagEnd::Paragraph => out.push('\n'),
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
                            link_url.cyan().underline()
                        ));
                        link_url.clear();
                    }
                }
                TagEnd::Strong => bold = false,
                TagEnd::Emphasis => italic = false,
                _ => {}
            },
            Event::Text(text) => {
                out.push_str(&style_text(&text, bold, italic, in_code_block, heading_level));
            }
            Event::Code(text) => {
                out.push_str(&format!("`{}`", theme::primary_light(&text)));
            }
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push('\n'),
            Event::Rule => {
                out.push_str(&format!("{}\n", theme::soft(&"─".repeat(40))));
            }
            _ => {}
        }
    }

    out.trim_end().to_string()
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
        return theme::muted(text);
    }

    let mut styled = text.to_string();
    if let Some(level) = heading {
        styled = match level {
            HeadingLevel::H1 => styled.blue().bold().to_string(),
            HeadingLevel::H2 => styled.cyan().bold().to_string(),
            HeadingLevel::H3 | HeadingLevel::H4 => styled.bright_blue().bold().to_string(),
            _ => theme::soft(&styled),
        };
        return styled;
    }

    if bold && italic {
        styled.blue().bold().italic().to_string()
    } else if bold {
        styled.cyan().bold().to_string()
    } else if italic {
        styled.blue().italic().to_string()
    } else {
        styled
    }
}
