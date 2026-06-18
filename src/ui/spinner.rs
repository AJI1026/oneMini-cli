//! 终端 Spinner 帧

use super::theme;

pub const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub const RETRO_FRAMES: &[&str] = &["|", "/", "-", "\\"];

pub fn frame(index: usize) -> &'static str {
    let frames = if theme::use_retro_table() {
        RETRO_FRAMES
    } else {
        FRAMES
    };
    frames[index % frames.len()]
}
