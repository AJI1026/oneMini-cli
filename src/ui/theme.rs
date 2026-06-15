//! OneMini CLI 蓝色主题配色

use colored::Colorize;

/// 主色：标题、助手标识、重要标签
pub fn primary(text: &str) -> String {
    text.blue().bold().to_string()
}

/// 主色浅色：工具名、代码、链接
pub fn primary_light(text: &str) -> String {
    text.cyan().to_string()
}

/// 强调色：图标、高亮元素
pub fn accent(text: &str) -> String {
    text.bright_blue().to_string()
}

/// 次要文字：说明、预览、Token 统计
pub fn muted(text: &str) -> String {
    text.bright_black().to_string()
}

/// 弱化主色：分隔线、推理块
pub fn soft(text: &str) -> String {
    text.blue().dimmed().to_string()
}

pub fn success_icon() -> String {
    "✓".cyan().bold().to_string()
}

pub fn error_icon() -> String {
    "✗".red().bold().to_string()
}

pub fn warn_icon() -> String {
    "!".bright_blue().bold().to_string()
}

pub fn tool_icon() -> String {
    "▸".bright_blue().bold().to_string()
}

pub fn thinking_icon() -> String {
    "◆".blue().to_string()
}

pub fn diff_add(line: &str) -> String {
    line.cyan().to_string()
}

pub fn diff_remove(line: &str) -> String {
    line.red().dimmed().to_string()
}
