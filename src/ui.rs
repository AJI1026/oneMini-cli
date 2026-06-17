mod markdown;
mod plan;
mod repl_helper;
mod stream;
mod theme;

use colored::Colorize;

pub use markdown::render_markdown;
pub use plan::render_plan_text;
pub use repl_helper::{colored_input_prompt, input_prompt_plain, ReplHelper};
pub use stream::{print_diff_preview, print_usage_line, StreamRenderer};

const LOGO_ART: &str = r"        .=====:
   .:==-*:   .#+-+-
 -**#- .*-   :#= :+*:
 .+-    .++--=-    -#-..
 -#.    :-  .++     =+..
 **    .-+++++:     =*:
 =%. .+*=:.   :-:   =*
  **.#+         := -#:
   ==#=         .+ =:
     :+=:.....:--
        :::::::.";

pub fn banner() -> String {
    let art = LOGO_ART
        .lines()
        .map(theme::primary_light)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n{}\n{}",
        art,
        theme::primary("OneMini CLI"),
        theme::muted("终端 AI 编程助手 · 输入 /help 查看命令 · Ctrl+C 退出")
    )
}

pub fn success(msg: &str) -> String {
    format!("{} {}", theme::success_icon(), theme::primary_light(msg))
}

pub fn error(msg: &str) -> String {
    format!("{} {}", theme::error_icon(), msg.red())
}

pub fn warn(msg: &str) -> String {
    format!("{} {}", theme::warn_icon(), theme::accent(msg))
}

pub fn tool_call(name: &str, detail: &str) -> String {
    format!(
        "{} {} {}",
        theme::tool_icon(),
        theme::primary_light(name),
        theme::muted(detail)
    )
}

pub fn reasoning_header() -> String {
    format!(
        "  {} {}",
        theme::thinking_icon(),
        "思考中".blue().bold()
    )
}

pub fn reasoning_text(text: &str) -> String {
    text.blue().dimmed().italic().to_string()
}

pub fn reasoning_footer() -> String {
    format!(
        "  {} {}",
        theme::thinking_icon(),
        theme::soft(&"─".repeat(32))
    )
}

pub fn diff_block(diff: &str) -> String {
    diff.lines()
        .map(|line| {
            if line.starts_with('+') && !line.starts_with("+++") {
                theme::diff_add(line)
            } else if line.starts_with('-') && !line.starts_with("---") {
                theme::diff_remove(line)
            } else {
                theme::muted(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn separator() -> String {
    theme::soft(&"─".repeat(48))
}

pub fn assistant_prefix() -> String {
    theme::primary("OneMini")
}

pub fn user_prefix() -> String {
    theme::accent("You")
}

pub fn dim(text: &str) -> String {
    theme::muted(text)
}

pub fn usage_line(text: &str) -> String {
    format!("{} {}", theme::soft("⎿"), theme::soft(text))
}

pub fn hint(text: &str) -> String {
    theme::primary_light(text)
}

pub fn section_title(text: &str) -> String {
    format!("\n{}\n", theme::primary(text))
}

pub fn status_pair(label: &str, value: &str) -> String {
    format!(
        "{} {}",
        theme::primary_light(&format!("{label}:")),
        theme::accent(value)
    )
}
