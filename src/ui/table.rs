//! 终端 ASCII 表格（技能列表、目录等）

use super::theme;

fn cell_width(s: &str) -> usize {
    s.chars().count()
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
}
