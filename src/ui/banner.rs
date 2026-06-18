//! 启动 Banner — Claude Code 风双层堆叠 + 实心块字 + 三层投影

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use super::theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BANNER_WIDTH: usize = 72;

/// Claude Code 风欢迎条
const LOGO_WELCOME: &[&str] = &[
    "┌────────────────────────────────────────┐",
    "│ * Welcome to oneMini                   │",
    "└────────────────────────────────────────┘",
];

/// 单字母块：5 行实体 + 3 行右下 echo 投影
const LOGO_FACE_ROWS: usize = 5;
const LOGO_SHADOW_ROWS: usize = 3;
const LOGO_ROW_COUNT: usize = LOGO_FACE_ROWS + LOGO_SHADOW_ROWS;
const LOGO_LETTER_GAP: usize = 3;
/// 两行词之间的空行（ONE / MINI）
const LOGO_LINE_GAP: usize = 1;

type LogoLetter = [&'static str; LOGO_ROW_COUNT];

/// O — 10 列空心方块
const LETTER_O: LogoLetter = [
    "  ██████  ",
    " ██    ██ ",
    " ██    ██ ",
    " ██    ██ ",
    "  ██████  ",
    "  ▀▀▀▀▀▀  ",
    "   ▀▀▀▀   ",
    "    ▀▀    ",
];

/// N — 10 列双竖 + 斜杠
const LETTER_N: LogoLetter = [
    "██    ██  ",
    "███   ██  ",
    "██ █  ██  ",
    "██  █ ██  ",
    "██   ███  ",
    " ▀▀▀ ▀▀   ",
    "  ▀▀▀▀    ",
    "   ▀▀     ",
];

/// E — 10 列三横
const LETTER_E: LogoLetter = [
    "██████████",
    "██        ",
    "████████  ",
    "██        ",
    "██████████",
    " ▀▀▀▀▀▀▀▀ ",
    "  ▀▀▀▀▀▀  ",
    "   ▀▀▀▀   ",
];

/// M — 10 列双峰
const LETTER_M: LogoLetter = [
    "██      ██",
    "███    ███",
    "██ █  █ ██",
    "██  ██  ██",
    "██      ██",
    " ▀▀    ▀▀ ",
    "  ▀▀▀▀▀▀  ",
    "   ▀▀▀▀   ",
];

/// I — 10 列竖条（与 M/N 同宽）
const LETTER_I: LogoLetter = [
    "   ████   ",
    "   ████   ",
    "   ████   ",
    "   ████   ",
    "   ████   ",
    "   ▀▀▀▀   ",
    "    ▀▀▀   ",
    "     ▀    ",
];

const LINE_ONE: &[LogoLetter] = &[LETTER_O, LETTER_N, LETTER_E];
const LINE_MINI: &[LogoLetter] = &[LETTER_M, LETTER_I, LETTER_N, LETTER_I];

const SCRAMBLE_CHARS: &[char] = &['█', '▀', '▄', '▓', '▒', '░'];
const BRAND_TYPEWRITER_MS: u64 = 45;
const BRAND_SHIMMER_MS: u64 = 65;
const BRAND_SHIMMER_FRAMES: u32 = 4;

/// 启动 Banner 上下文（REPL 传入完整信息，config 引导传默认空值）
#[derive(Debug, Clone, Default)]
pub struct BannerInfo<'a> {
    pub model: Option<&'a str>,
    pub permission_mode: Option<&'a str>,
    pub workdir: Option<&'a Path>,
}

fn theme_subtitle() -> &'static str {
    match theme::current_theme() {
        theme::ThemeId::GameBoy => "DMG MODE · /help · Ctrl+C",
        theme::ThemeId::Nes => "NES MODE · /help · Ctrl+C",
        theme::ThemeId::Modern => "输入 /help 查看命令 · /skills list 查看技能 · Ctrl+C 退出",
    }
}

fn center_line(line: &str, width: usize) -> String {
    let visible = line.chars().count();
    if visible >= width {
        return line.to_string();
    }
    let pad = (width - visible) / 2;
    format!("{}{line}", " ".repeat(pad))
}

/// 计算当前行在词块内的 echo 投影层级（0 = 实体，1–3 = 逐层加深）
fn logo_shadow_tier(row: usize) -> u8 {
    let block = LOGO_ROW_COUNT + LOGO_LINE_GAP;
    let pos = row % block;
    if pos >= LOGO_ROW_COUNT {
        return 0;
    }
    if pos >= LOGO_FACE_ROWS {
        (LOGO_ROW_COUNT - pos) as u8
    } else {
        0
    }
}

fn is_logo_shadow_row(row: usize) -> bool {
    logo_shadow_tier(row) > 0
}

fn color_logo_line(line: &str, row: usize, phase: f32) -> String {
    let tier = logo_shadow_tier(row);
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len().max(1);
    chars
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let t = i as f32 / (len - 1) as f32;
            theme::banner_logo_char_gradient(c, tier, t, phase)
        })
        .collect()
}

fn render_colored_logo(lines: &[String], phase: f32) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| color_logo_line(&center_line(line, BANNER_WIDTH), i, phase))
        .collect()
}

fn render_welcome_strip() -> Vec<String> {
    LOGO_WELCOME
        .iter()
        .map(|line| theme::banner_welcome_line(&center_line(line, BANNER_WIDTH)))
        .collect()
}

fn brand_title_line(brand_part: &str, phase: f32, modern: bool) -> String {
    let line = format!(
        "{} {}",
        theme::render_brand_gradient(brand_part, phase),
        theme::muted(&format!("v{VERSION}"))
    );
    if modern {
        center_line(&line, BANNER_WIDTH)
    } else {
        line
    }
}

fn render_title_meta() -> String {
    brand_title_line("oneMini", 0.0, true)
}

fn render_title_meta_inline() -> String {
    brand_title_line("oneMini", 0.0, false)
}

fn render_detail_text(info: &BannerInfo<'_>) -> String {
    center_line(&render_detail_text_inline(info), BANNER_WIDTH)
}

fn render_detail_text_inline(info: &BannerInfo<'_>) -> String {
    match (info.model, info.permission_mode) {
        (Some(model), Some(mode)) => format!("{model} · {mode} · {}", theme_subtitle()),
        _ => theme_subtitle().to_string(),
    }
}

fn render_workdir_text(workdir: &Path) -> String {
    center_line(
        &theme::muted(&workdir.display().to_string()),
        BANNER_WIDTH,
    )
}

fn render_workdir_text_inline(workdir: &Path) -> String {
    theme::muted(&workdir.display().to_string())
}

/// 紧凑模式：按词块内行号着色（含 echo 投影 + 横向渐变）
fn format_compact_glyph(glyph: &str, row_in_block: usize, phase: f32) -> String {
    let tier = if row_in_block >= LOGO_FACE_ROWS {
        (LOGO_ROW_COUNT - row_in_block) as u8
    } else {
        0
    };
    let chars: Vec<char> = glyph.chars().collect();
    let len = chars.len().max(1);
    chars
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let t = i as f32 / (len - 1) as f32;
            theme::banner_logo_char_gradient(c, tier, t, phase)
        })
        .collect()
}

fn compact_logo_colored_lines(phase: f32) -> Vec<String> {
    logo_compact_lines()
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if line.trim().is_empty() {
                return String::new();
            }
            let block = LOGO_ROW_COUNT + LOGO_LINE_GAP;
            let pos = i % block;
            if pos >= LOGO_ROW_COUNT {
                String::new()
            } else {
                format_compact_glyph(&line, pos, phase)
            }
        })
        .collect()
}

fn compact_text_rows(info: &BannerInfo<'_>) -> Vec<String> {
    let mut rows = vec![
        render_title_meta_inline(),
        theme::muted(&render_detail_text_inline(info)),
    ];
    if let Some(workdir) = info.workdir {
        rows.push(render_workdir_text_inline(workdir));
    }
    rows
}

/// 横向拼接字母
fn compose_logo_letters(letters: &[LogoLetter], gap: usize) -> Vec<String> {
    let mut rows = vec![String::new(); LOGO_ROW_COUNT];
    for (idx, letter) in letters.iter().enumerate() {
        for r in 0..LOGO_ROW_COUNT {
            if idx > 0 {
                rows[r].push_str(&" ".repeat(gap));
            }
            rows[r].push_str(letter[r].trim_end());
        }
    }
    rows
}

/// Claude Code 风双层 Logo：ONE 上 / MINI 下，左对齐同列
fn logo_3d_lines() -> Vec<String> {
    let mut lines = compose_logo_letters(LINE_ONE, LOGO_LETTER_GAP);
    lines.push(String::new());
    lines.extend(compose_logo_letters(LINE_MINI, LOGO_LETTER_GAP));
    lines
}

/// 紧凑模式：ONE + MINI 完整块（实体 + echo 投影）
fn logo_compact_lines() -> Vec<String> {
    let mut lines = compose_logo_letters(LINE_ONE, LOGO_LETTER_GAP);
    lines.push(String::new());
    lines.extend(compose_logo_letters(LINE_MINI, LOGO_LETTER_GAP));
    lines
}

/// 渲染完整启动 Banner
pub fn render_banner(info: &BannerInfo<'_>) -> String {
    let mut lines = Vec::new();
    lines.extend(render_welcome_strip());
    lines.push(String::new());
    lines.extend(render_colored_logo(&logo_3d_lines(), 0.0));
    lines.push(String::new());
    lines.push(render_title_meta());
    lines.push(render_detail_text(info));
    if let Some(workdir) = info.workdir {
        lines.push(render_workdir_text(workdir));
    }
    lines.join("\n")
}

/// 紧凑 Banner（非 Modern 主题）：先完整 Logo，再基础信息
fn render_compact_banner(info: &BannerInfo<'_>) -> String {
    let mut out = compact_logo_colored_lines(0.0);
    out.push(String::new());
    out.extend(compact_text_rows(info));
    out.join("\n")
}

/// 静态 Banner（无上下文兜底）
pub fn banner_static() -> String {
    render_banner(&BannerInfo::default())
}

fn resolve_banner_output(info: &BannerInfo<'_>) -> String {
    match theme::current_theme() {
        theme::ThemeId::Modern => match (info.model, info.permission_mode, info.workdir) {
            (None, None, None) => banner_static(),
            _ => render_banner(info),
        },
        _ => render_compact_banner(info),
    }
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

fn random_scramble_char() -> char {
    let mut buf = [0u8; 1];
    if getrandom::getrandom(&mut buf).is_err() {
        return '█';
    }
    SCRAMBLE_CHARS[buf[0] as usize % SCRAMBLE_CHARS.len()]
}

fn scramble_line(line: &str) -> String {
    line.chars()
        .map(|c| if c == ' ' { ' ' } else { random_scramble_char() })
        .collect()
}

async fn anim_delay(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

async fn play_brand_title_line(modern: bool) {
    let brand = "oneMini";
    let char_count = brand.chars().count();
    for n in 1..=char_count {
        let partial: String = brand.chars().take(n).collect();
        print_line_cleared(&brand_title_line(&partial, 0.0, modern));
        if n < char_count {
            anim_delay(BRAND_TYPEWRITER_MS).await;
        }
    }
    for frame in 0..BRAND_SHIMMER_FRAMES {
        let phase = frame as f32 / BRAND_SHIMMER_FRAMES as f32;
        print_line_cleared(&brand_title_line(brand, phase, modern));
        anim_delay(BRAND_SHIMMER_MS).await;
    }
    println!("{}", brand_title_line(brand, 0.0, modern));
}

async fn play_modern_animated(info: &BannerInfo<'_>) {
    for line in render_welcome_strip() {
        println!("{line}");
        anim_delay(30).await;
    }
    println!();
    anim_delay(40).await;

    let logo_lines = logo_3d_lines();
    let logo_final = render_colored_logo(&logo_lines, 0.0);

    for i in 0..logo_lines.len() {
        if logo_lines[i].trim().is_empty() {
            println!();
            continue;
        }
        let centered = center_line(&logo_lines[i], BANNER_WIDTH);
        let scrambled = scramble_line(&centered);
        let is_shadow = is_logo_shadow_row(i);
        print_line_cleared(&color_logo_line(&scrambled, i, 0.0));
        anim_delay(if is_shadow { 18 } else { 12 }).await;
        print_line_cleared(&logo_final[i]);
        println!();
        anim_delay(if is_shadow { 10 } else { 4 }).await;
    }

    println!();
    play_brand_title_line(true).await;
    println!("{}", render_detail_text(info));
    if let Some(workdir) = info.workdir {
        println!("{}", render_workdir_text(workdir));
        anim_delay(35).await;
    }
}

async fn play_compact_animated(info: &BannerInfo<'_>) {
    let compact = logo_compact_lines();
    let text_rows = compact_text_rows(info);

    for (i, glyph) in compact.iter().enumerate() {
        if glyph.trim().is_empty() {
            println!();
        } else {
            let block = LOGO_ROW_COUNT + LOGO_LINE_GAP;
            let pos = i % block;
            let colored = if pos >= LOGO_ROW_COUNT {
                String::new()
            } else {
                format_compact_glyph(glyph, pos, 0.0)
            };
            print_line_cleared(&colored);
            println!();
        }
        anim_delay(if is_logo_shadow_row(i) { 40 } else { 55 }).await;
    }

    println!();
    play_brand_title_line(false).await;
    for text in text_rows.iter().skip(1) {
        println!("{text}");
        anim_delay(35).await;
    }
}

async fn play_startup_banner_animated(info: &BannerInfo<'_>) {
    match theme::current_theme() {
        theme::ThemeId::Modern => play_modern_animated(info).await,
        _ => play_compact_animated(info).await,
    }
}

/// 阻塞版（config setup 等同步上下文）
pub fn play_startup_banner_blocking(info: &BannerInfo<'_>) {
    if !animations_enabled() {
        println!("{}", resolve_banner_output(info));
        return;
    }
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(play_startup_banner_animated(info));
    } else {
        futures::executor::block_on(play_startup_banner_animated(info));
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
    fn banner_has_3d_logo_and_title() {
        theme::set_theme(theme::ThemeId::Modern);
        let out = render_banner(&BannerInfo::default());
        assert!(out.contains("Welcome to oneMini"));
        assert!(out.contains("oneMini"));
        assert!(out.contains("██    ██"));
        assert!(out.contains("▀▀"));
    }

    #[test]
    fn one_line_is_wide_enough() {
        let lines = compose_logo_letters(LINE_ONE, LOGO_LETTER_GAP);
        assert!(
            lines[0].chars().count() >= 32,
            "ONE 首行宽度 {} 不足",
            lines[0].chars().count()
        );
    }

    #[test]
    fn compose_two_line_logo() {
        let lines = logo_3d_lines();
        assert_eq!(lines.len(), LOGO_ROW_COUNT * 2 + LOGO_LINE_GAP);
        assert!(lines[0].contains("████"));
        let mini_start = LOGO_ROW_COUNT + LOGO_LINE_GAP;
        assert!(lines[mini_start].starts_with("██"));
    }

    #[test]
    fn compose_logo_letters_gap() {
        let one = compose_logo_letters(&[LETTER_O], 0);
        let two = compose_logo_letters(&[LETTER_O, LETTER_I], LOGO_LETTER_GAP);
        assert!(two[0].chars().count() > one[0].chars().count());
    }

    #[test]
    fn shadow_tier_within_word_block() {
        assert_eq!(logo_shadow_tier(4), 0);
        assert_eq!(logo_shadow_tier(5), 3);
        assert_eq!(logo_shadow_tier(6), 2);
        assert_eq!(logo_shadow_tier(7), 1);
        assert_eq!(logo_shadow_tier(8), 0);
        assert_eq!(logo_shadow_tier(14), 3);
    }

    #[test]
    fn banner_includes_workdir_row_when_present() {
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

    #[test]
    fn logo_has_face_and_shadow_rows() {
        let lines = logo_3d_lines();
        assert!(lines[0].contains('█'));
        assert!(lines[5].contains('▀'));
        assert!(lines[7].contains('▀'));
    }

    #[test]
    fn compact_banner_shows_one_and_mini() {
        theme::set_theme(theme::ThemeId::GameBoy);
        let compact = logo_compact_lines();
        assert_eq!(compact.len(), LOGO_ROW_COUNT * 2 + LOGO_LINE_GAP);
        let mini_start = LOGO_ROW_COUNT + LOGO_LINE_GAP;
        assert!(compact[mini_start].starts_with("██"));

        let out = render_compact_banner(&BannerInfo::default());
        let lines: Vec<_> = out.lines().collect();
        let logo_rows = LOGO_ROW_COUNT * 2 + LOGO_LINE_GAP;
        assert!(lines.len() >= logo_rows + 2);
        let title_idx = lines
            .iter()
            .position(|l| l.contains("oneMini"))
            .expect("应有标题行");
        assert!(title_idx > logo_rows, "标题应在 Logo 绘制完成后显示");
        assert_eq!(
            lines.iter().filter(|l| l.contains("oneMini")).count(),
            1
        );
        // MINI 末行实体含 i 竖条
        let mini_last_face = &compact[mini_start + LOGO_FACE_ROWS - 1];
        assert!(mini_last_face.contains("████"));
    }

    #[test]
    fn scramble_line_preserves_spaces() {
        let scrambled = scramble_line("  █  ");
        assert_eq!(scrambled.chars().nth(0), Some(' '));
        assert_eq!(scrambled.chars().nth(1), Some(' '));
        assert_ne!(scrambled.chars().nth(2), Some(' '));
        assert_eq!(scrambled.chars().nth(4), Some(' '));
    }
}
