//! 终端 ASCII 表格（技能列表、目录等）

use super::theme;
use unicode_width::UnicodeWidthStr;

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch == 'm' {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn cell_width(s: &str) -> usize {
    strip_ansi(s).width()
}

fn pad_cell(s: &str, width: usize) -> String {
    let w = cell_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

fn format_row(cells: &[String], widths: &[usize]) -> String {
    cells
        .iter()
        .enumerate()
        .map(|(i, c)| pad_cell(c, widths.get(i).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("  ")
}

fn format_sep(widths: &[usize]) -> String {
    if theme::use_retro_table() {
        let inner: String = widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("-+-");
        format!("+{inner}+")
    } else {
        widths
            .iter()
            .map(|w| "─".repeat(*w))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

fn format_retro_row(cells: &[String], widths: &[usize]) -> String {
    let inner: String = cells
        .iter()
        .enumerate()
        .map(|(i, c)| pad_cell(c, widths.get(i).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("|");
    format!("|{inner}|")
}

/// 渲染固定列宽的 ASCII 表格
pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }
    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| cell_width(h)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell_width(cell));
            }
        }
    }

    let header_cells: Vec<String> = headers.iter().map(|s| (*s).to_string()).collect();
    let mut out = if theme::use_retro_table() {
        format_retro_row(&header_cells, &widths)
    } else {
        format_row(&header_cells, &widths)
    };
    out.push('\n');
    out.push_str(&format_sep(&widths));
    out.push('\n');
    for row in rows {
        let mut cells = row.clone();
        cells.resize(col_count, String::new());
        if theme::use_retro_table() {
            out.push_str(&format_retro_row(&cells, &widths));
        } else {
            out.push_str(&format_row(&cells, &widths));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::render_table;

    #[test]
    fn basic_table() {
        let out = render_table(
            &["技能", "来源"],
            &[vec!["debug".into(), "内置".into()]],
        );
        assert!(out.contains("debug"));
        assert!(out.contains("内置"));
    }

    #[test]
    fn cjk_and_emoji_use_display_width() {
        let out = render_table(
            &["时段", "天气", "温度"],
            &[
                vec!["🌅 早上".into(), "烟霾".into(), "23°C".into()],
                vec!["☀️ 中午".into(), "局部阵雨".into(), "29°C".into()],
            ],
        );
        for expected in ["🌅 早上", "局部阵雨", "23°C", "☀️ 中午"] {
            assert!(out.contains(expected), "missing {expected} in:\n{out}");
        }
    }
}
