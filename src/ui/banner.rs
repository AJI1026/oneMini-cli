//! 启动 Banner — 窗口顶栏 + Welcome + Logo + 下方 Meta 描述

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use super::panel;
use super::theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ANSI Shadow 风格块字（figlet "OneMini"）
const LOGO_ROWS: usize = 6;

/// figlet -f "ANSI Shadow" OneMini
const LOGO_LINES: [&'static str; LOGO_ROWS] = [
    " ██████╗ ███╗   ██╗███████╗███╗   ███╗██╗███╗   ██╗██╗",
    "██╔═══██╗████╗  ██║██╔════╝████╗ ████║██║████╗  ██║██║",
    "██║   ██║██╔██╗ ██║█████╗  ██╔████╔██║██║██╔██╗ ██║██║",
    "██║   ██║██║╚██╗██║██╔══╝  ██║╚██╔╝██║██║██║╚██╗██║██║",
    "╚██████╔╝██║ ╚████║███████╗██║ ╚═╝ ██║██║██║ ╚████║██║",
    " ╚═════╝ ╚═╝  ╚═══╝╚══════╝╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═╝",
];

/// 渐变扫光帧数
const GRADIENT_SHIMMER_FRAMES: u32 = 10;

/// 启动 Banner 上下文（REPL 传入完整信息，config 引导传默认空值）
#[derive(Debug, Clone, Default)]
pub struct BannerInfo<'a> {
    pub model: Option<&'a str>,
    pub permission_mode: Option<&'a str>,
    pub workdir: Option<&'a Path>,
}

fn color_logo_line(line: &str, row: usize, phase: f32) -> String {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len().max(1);
    let row_t = row as f32 / (LOGO_ROWS - 1).max(1) as f32;
    chars
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let col_t = i as f32 / (len - 1) as f32;
            let t = col_t * 0.65 + row_t * 0.35;
            theme::banner_logo_char_gradient(c, t, phase)
        })
        .collect()
}

fn render_colored_logo(lines: &[String], phase: f32) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| color_logo_line(line, i, phase))
        .collect()
}

fn render_meta_column(info: &BannerInfo<'_>) -> Vec<String> {
    let mut rows = vec![
        theme::banner_meta_label(&format!("OneMini CLI v{VERSION}")),
        theme::banner_meta_value("终端 AI 编程助手"),
    ];
    if let Some(model) = info.model {
        rows.push(theme::banner_meta_value(&format!("model: {model}")));
    } else {
        rows.push(theme::banner_meta_value("model: (未配置)"));
    }
    if let Some(mode) = info.permission_mode {
        rows.push(theme::banner_meta_value(&format!("mode: {mode}")));
    }
    rows.push(theme::banner_meta_value(&format!(
        "theme: {}",
        theme::current_theme().as_str()
    )));
    if let Some(workdir) = info.workdir {
        rows.push(theme::banner_meta_value(&workdir.display().to_string()));
    } else {
        rows.push(theme::banner_meta_prompt("> 运行 cargo test 并解释失败原因"));
    }
    rows
}

fn render_banner_header() -> Vec<String> {
    vec![
        panel::render_window_chrome("onemini - zsh"),
        String::new(),
        panel::render_welcome_strip(),
        String::new(),
    ]
}

fn logo_lines() -> Vec<String> {
    LOGO_LINES.iter().map(|s| s.to_string()).collect()
}

fn render_banner_body(info: &BannerInfo<'_>) -> Vec<String> {
    let logo_colored = render_colored_logo(&logo_lines(), 0.0);
    let mut lines = logo_colored;
    lines.push(String::new());
    lines.extend(render_meta_column(info));
    lines
}

/// 渲染完整启动 Banner
pub fn render_banner(info: &BannerInfo<'_>) -> String {
    let mut lines = render_banner_header();
    lines.extend(render_banner_body(info));
    lines.join("\n")
}

fn resolve_banner_output(info: &BannerInfo<'_>) -> String {
    render_banner(info)
}

fn animations_enabled() -> bool {
    io::stdout().is_terminal()
        && std::env::var("CI").is_err()
        && std::env::var("ONEMINI_BANNER_ANIM").ok().as_deref() != Some("0")
}

fn flush_stdout() {
    let _ = io::stdout().flush();
}

fn print_line_cleared(line: &str) {
    print!("\x1b[2K\r{line}");
    flush_stdout();
}

fn cursor_up(lines: usize) {
    if lines > 0 {
        print!("\x1b[{lines}A");
        flush_stdout();
    }
}

async fn anim_delay(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

async fn print_banner_header() {
    for line in render_banner_header() {
        if line.is_empty() {
            println!();
        } else {
            println!("{line}");
        }
        anim_delay(25).await;
    }
}

async fn reveal_logo_rows(logo_raw: &[String], row_delay_ms: u64) {
    for (row, raw_line) in logo_raw.iter().enumerate() {
        if raw_line.trim().is_empty() {
            println!();
            continue;
        }
        let phase = row as f32 * 0.06;
        println!("{}", color_logo_line(raw_line, row, phase));
        anim_delay(row_delay_ms).await;
    }
}

async fn shimmer_logo(logo_raw: &[String]) {
    let row_count = logo_raw.len();
    for frame in 0..GRADIENT_SHIMMER_FRAMES {
        let phase = frame as f32 / GRADIENT_SHIMMER_FRAMES as f32;
        let logo_colored = render_colored_logo(logo_raw, phase);
        cursor_up(row_count);
        for line in logo_colored.iter().take(row_count) {
            print_line_cleared(line);
            println!();
        }
        anim_delay(55).await;
    }
}

async fn print_meta_block(meta: &[String]) {
    println!();
    for line in meta {
        println!("{line}");
        anim_delay(20).await;
    }
}

async fn play_startup_banner_animated(info: &BannerInfo<'_>) {
    print_banner_header().await;

    let logo_raw = logo_lines();
    let meta = render_meta_column(info);
    let row_delay = match theme::current_theme() {
        theme::ThemeId::Modern => 35,
        _ => 50,
    };

    reveal_logo_rows(&logo_raw, row_delay).await;
    if animations_enabled() && theme::colors_enabled() {
        shimmer_logo(&logo_raw).await;
    }
    print_meta_block(&meta).await;
}

/// 阻塞版（config setup 等同步上下文）
pub fn play_startup_banner_blocking(info: &BannerInfo<'_>) {
    if !animations_enabled() {
        println!("{}", resolve_banner_output(info));
        return;
    }
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(play_startup_banner_animated(info)));
    } else {
        tokio::runtime::Runtime::new()
            .expect("failed to create tokio runtime")
            .block_on(play_startup_banner_animated(info));
    }
}

/// 异步版（REPL 启动）
pub async fn play_startup_banner(info: &BannerInfo<'_>) {
    if !animations_enabled() {
        println!("{}", resolve_banner_output(info));
        return;
    }
    play_startup_banner_animated(info).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_has_window_chrome_and_welcome_panel() {
        let _g = theme::theme_test_guard();
        theme::set_theme(theme::ThemeId::Modern);
        let out = render_banner(&BannerInfo::default());
        assert!(out.contains("onemini - zsh"));
        assert!(out.contains("Welcome to oneMini"));
        assert!(out.contains("OneMini CLI"));
    }

    #[test]
    fn banner_has_logo_and_title() {
        let _g = theme::theme_test_guard();
        theme::set_theme(theme::ThemeId::Modern);
        let out = render_banner(&BannerInfo::default());
        assert!(out.contains("Welcome to oneMini"));
        assert!(out.contains("OneMini CLI"));
        assert!(out.contains("██████╗"));
        assert!(out.contains("╚═════╝"));
    }

    #[test]
    fn logo_has_six_unicode_rows() {
        let lines = logo_lines();
        assert_eq!(lines.len(), LOGO_ROWS);
        assert!(lines.iter().all(|l| l.chars().any(|c| c != ' ')));
    }

    #[test]
    fn logo_spells_onemini() {
        let lines = logo_lines();
        assert_eq!(lines.len(), LOGO_ROWS);
        // ANSI Shadow：首行含 O 左缘，末行含 i 右缘
        assert!(lines[0].contains("██████╗"));
        assert!(lines[5].contains("╚═══╝"));
    }

    #[test]
    fn compact_banner_shows_onemini_art() {
        let _g = theme::theme_test_guard();
        theme::set_theme(theme::ThemeId::GameBoy);
        let compact = logo_lines();
        assert_eq!(compact.len(), LOGO_ROWS);
        assert!(compact[0].contains("██████╗"));

        let out = render_banner(&BannerInfo::default());
        assert!(out.contains("Welcome to oneMini"));
        assert!(out.contains("OneMini CLI"));
        assert!(out.contains("onemini - zsh"));
    }

    #[test]
    fn meta_appears_below_logo() {
        let _g = theme::theme_test_guard();
        theme::set_theme(theme::ThemeId::Modern);
        let out = render_banner(&BannerInfo::default());
        let lines: Vec<_> = out.lines().collect();
        let logo_idx = lines.iter().position(|l| l.contains("██████╗")).unwrap();
        let meta_idx = lines.iter().position(|l| l.contains("OneMini CLI")).unwrap();
        assert!(meta_idx > logo_idx, "描述文字应在 Logo 下方");
    }

    #[test]
    fn banner_includes_workdir_row_when_present() {
        let _g = theme::theme_test_guard();
        theme::set_theme(theme::ThemeId::Modern);
        let info = BannerInfo {
            model: Some("gpt-4o"),
            permission_mode: Some("默认"),
            workdir: Some(Path::new("/tmp/project")),
        };
        let rendered = render_banner(&info);
        let lines: Vec<_> = rendered.lines().collect();
        assert!(lines.len() > 5);
        assert!(rendered.contains("/tmp/project"));
    }
}
