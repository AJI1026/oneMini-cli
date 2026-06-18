//! 启动 Banner — Modern / Game Boy / NES 像素风 ASCII

use std::io::{self, Write};
use std::time::Duration;

use super::theme;

/// Modern 徽标（蛇形）
pub const LOGO_ART_MODERN: &str = r"        .=====:
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


/// Game Boy DMG 像素块
pub const LOGO_ART_GAMEBOY: &str = r"  ################
  # ##  ##  ##  ##
  #  ####  ####  #
  # ##  ##  ##  ##
  #  ####  ####  #
  #    ONE MINI   #
  #  ##  ##  ##  ##
  #  ####  ####  #
  ################";

/// NES 卡带框
pub const LOGO_ART_NES: &str = r"  +==============+
  |  ONE  MINI   |
  |  ==========  |
  |  *  *  *  *  |
  |  ==========  |
  |   CLI v0.1     |
  +==============+";

const ANIM_FRAMES: usize = 10;
const FRAME_MS: u64 = 75;

fn logo_art() -> &'static str {
    match theme::current_theme() {
        theme::ThemeId::Modern => LOGO_ART_MODERN,
        theme::ThemeId::GameBoy => LOGO_ART_GAMEBOY,
        theme::ThemeId::Nes => LOGO_ART_NES,
    }
}

fn logo_line_count() -> usize {
    logo_art().lines().count()
}

fn render_logo_frame(frame: usize) -> String {
    logo_art()
        .lines()
        .enumerate()
        .map(|(i, line)| theme::logo_line(line, frame, i))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_banner_tail() -> String {
    let subtitle = match theme::current_theme() {
        theme::ThemeId::GameBoy => "DMG MODE · /help · Ctrl+C",
        theme::ThemeId::Nes => "NES MODE · /help · Ctrl+C",
        theme::ThemeId::Modern => "终端 AI 编程助手 · 输入 /help 查看命令 · Ctrl+C 退出",
    };
    format!(
        "{}\n{}",
        theme::primary("OneMini CLI"),
        theme::muted(subtitle)
    )
}

/// 静态 Banner（非 TTY / 管道输出）
pub fn banner_static() -> String {
    format!("{}\n{}", render_logo_frame(0), render_banner_tail())
}

fn paint_logo_frame(frame: usize, line_count: usize) {
    if frame > 0 {
        print!("\x1b[{line_count}A");
    }
    for line in render_logo_frame(frame).lines() {
        print!("\x1b[2K\r{line}\n");
    }
    io::stdout().flush().ok();
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// 阻塞版动画（config setup 等同步上下文）
pub fn play_startup_banner_blocking() {
    if !theme::colors_enabled() {
        println!("{}", banner_static());
        return;
    }
    let n = logo_line_count();
    for frame in 0..ANIM_FRAMES {
        paint_logo_frame(frame, n);
        sleep_ms(FRAME_MS);
    }
    println!("{}", render_banner_tail());
}

/// 异步版动画（REPL 启动）
pub async fn play_startup_banner() {
    if !theme::colors_enabled() {
        println!("{}", banner_static());
        return;
    }
    let n = logo_line_count();
    for frame in 0..ANIM_FRAMES {
        paint_logo_frame(frame, n);
        tokio::time::sleep(Duration::from_millis(FRAME_MS)).await;
    }
    println!("{}", render_banner_tail());
}
