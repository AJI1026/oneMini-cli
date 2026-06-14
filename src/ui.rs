mod markdown;

use colored::Colorize;

pub use markdown::render_markdown;

pub fn banner() -> String {
    format!(
        "{}\n{}",
        "OneMini CLI".cyan().bold(),
        "终端 AI 编程助手 · 输入 /help 查看命令 · Ctrl+C 退出".dimmed()
    )
}

pub fn success(msg: &str) -> String {
    format!("{} {}", "✓".green().bold(), msg)
}

pub fn error(msg: &str) -> String {
    format!("{} {}", "✗".red().bold(), msg)
}

pub fn warn(msg: &str) -> String {
    format!("{} {}", "!".yellow().bold(), msg)
}

pub fn tool_call(name: &str, detail: &str) -> String {
    format!(
        "{} {} {}",
        "▸".blue().bold(),
        name.cyan().bold(),
        detail.dimmed()
    )
}

pub fn thinking_label() -> String {
    "思考中…".dimmed().to_string()
}

pub fn assistant_prefix() -> String {
    "OneMini".green().bold().to_string()
}

pub fn user_prefix() -> String {
    "You".blue().bold().to_string()
}

pub fn dim(text: &str) -> String {
    text.dimmed().to_string()
}
