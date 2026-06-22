//! 终端宽度与原地重绘辅助（处理软换行）

use std::io::Write;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// 去掉 ANSI 转义序列后的可见宽度
pub fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

/// 文本在终端上占用的行数（含软换行）
pub fn visual_rows(line: &str, term_width: usize) -> usize {
    let w = visible_width(line);
    if w == 0 {
        1
    } else {
        w.div_ceil(term_width.max(1))
    }
}

/// 多行块在终端上占用的总行数
pub fn total_visual_rows(lines: &[String], term_width: usize) -> usize {
    if lines.is_empty() {
        1
    } else {
        lines
            .iter()
            .map(|l| visual_rows(l, term_width))
            .sum::<usize>()
            .max(1)
    }
}

/// 按可见列数截断纯文本，超出时追加省略号
pub fn truncate_visible(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if text.width() <= max_cols {
        return text.to_string();
    }
    let budget = max_cols.saturating_sub(1);
    let mut used = 0usize;
    let mut out = String::new();
    for ch in text.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > budget {
            break;
        }
        used += cw;
        out.push(ch);
    }
    out.push('…');
    out
}

/// 清除光标所在块（共 `rows` 个终端行，含软换行）
pub fn clear_visual_rows(rows: usize) {
    if rows == 0 {
        return;
    }
    if rows == 1 {
        print!("\x1b[2K\r");
        return;
    }
    print!("\x1b[{}A", rows - 1);
    for i in 0..rows {
        print!("\x1b[2K");
        if i + 1 < rows {
            print!("\x1b[1B");
        }
    }
    print!("\x1b[{}A", rows - 1);
}

pub fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&w| w > 0)
        .unwrap_or(80)
}

/// 将终端光标恢复为细竖线（readline / 流式输出后常见粗块光标）
pub fn set_cursor_bar() {
    // DECSCUSR 6: steady bar — iTerm2 / Terminal.app / VS Code / Cursor 终端均支持
    print!("\x1b[6 q");
    let _ = std::io::stdout().flush();
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_rows_single_line() {
        assert_eq!(visual_rows("hello", 80), 1);
    }

    #[test]
    fn visual_rows_wraps_long_line() {
        let line = "a".repeat(100);
        assert_eq!(visual_rows(&line, 40), 3);
    }

    #[test]
    fn truncate_visible_adds_ellipsis() {
        let out = truncate_visible("你好世界", 4);
        assert!(out.ends_with('…'));
        assert!(out.width() <= 4);
    }

    #[test]
    fn total_visual_rows_sums_each_line() {
        let lines = vec!["a".repeat(50), "short".to_string()];
        assert_eq!(total_visual_rows(&lines, 40), 3);
    }
}
