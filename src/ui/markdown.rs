use colored::Colorize;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};

/// 将 Markdown 渲染为带 ANSI 样式的终端文本。
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
                    out.push_str("• ");
                }
                Tag::BlockQuote(_) => out.push_str(&"│ ".cyan().to_string()),
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        if !lang.is_empty() {
                            out.push_str(&format!("{lang}\n").dimmed().to_string());
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
                        out.push_str(&format!(" ({})", link_url.blue().underline()));
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
                out.push_str(&format!("`{}`", text.cyan()));
            }
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push('\n'),
            Event::Rule => {
                out.push_str(&format!("{}\n", "─".repeat(40).dimmed()));
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
        return text.dimmed().to_string();
    }

    let mut styled = text.to_string();
    if let Some(level) = heading {
        styled = match level {
            HeadingLevel::H1 => styled.bold().white().to_string(),
            HeadingLevel::H2 => styled.bold().to_string(),
            HeadingLevel::H3 | HeadingLevel::H4 => styled.bold().dimmed().to_string(),
            _ => styled.dimmed().to_string(),
        };
        return styled;
    }

    if bold && italic {
        styled.bold().italic().to_string()
    } else if bold {
        styled.bold().to_string()
    } else if italic {
        styled.italic().to_string()
    } else {
        styled
    }
}
