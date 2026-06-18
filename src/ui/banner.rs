//! 启动 Banner — Matrix 黑客帝国风：衔尾蛇 ASCII + 品牌字标

use std::path::Path;

use super::theme;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BANNER_WIDTH: usize = 72;

/// 衔尾蛇主图形（首尾闭环，外圈标注技能模块缩写；每行等宽 50 便于居中）
const OUROBOROS: &[&str] = &[
    "                      .-=+*########*+=-.          ",
    "                 .-+#*\\             /*#+-.        ",
    "               .+*#                   #*+.        ",
    "             +*#  [CR]           [DG]  #*+        ",
    "            *#                           #*       ",
    "          #*  $ > ONEMINI [SKILL_SYSTEM]  *#      ",
    "       #*                                   *#    ",
    "     #*  [EC]                         [DOC]  *#   ",
    "   #*                                         *#  ",
    "  #*    [PDF]                       [PPT]     *#  ",
    " #*                                            *# ",
    "  #*  [RF]                           [RD]     *#  ",
    "    #*    [XL]                   [CM]         *#  ",
    "       #*         .-'```'-.                 *#    ",
    "        #*       /'  (o)   '\\               *#    ",
    "        #*      |    >===<    |              *#   ",
    "         #*      \\    ```    /              *#    ",
    "           +*#     '-./\\___/\\.-'          #*+     ",
    "               .+*#                   #*+.        ",
    "                 .-+#*\\             /*#+-.        ",
    "                      .-=+*########*+=-.          ",
];

/// 块字 ONEMINI 标题
const TITLE_BLOCK: &[&str] = &[
    "  ___   _   _______ _____ _   _ ___ ",
    " / _ \\ | \\ | |  ___|_   _| \\ | |_ _|",
    "| | | ||  \\| | |_    | | |  \\| || | ",
    "| |_| || |\\  |  _|   | | | |\\  || | ",
    " \\___/ |_| \\_|_|     |_| |_| \\_|___|",
];

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

fn color_banner_line(line: &str) -> String {
    line.chars().map(theme::banner_glyph_char).collect()
}

fn render_colored_block(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .map(|line| color_banner_line(&center_line(line, BANNER_WIDTH)))
        .collect()
}

fn render_title_meta() -> String {
    center_line(
        &format!(
            "{}{} {}",
            theme::primary("One"),
            theme::accent("Mini"),
            theme::muted(&format!("v{VERSION}"))
        ),
        BANNER_WIDTH,
    )
}

fn render_detail_text(info: &BannerInfo<'_>) -> String {
    let text = match (info.model, info.permission_mode) {
        (Some(model), Some(mode)) => format!("{model} · {mode} · {}", theme_subtitle()),
        _ => theme_subtitle().to_string(),
    };
    center_line(&theme::muted(&text), BANNER_WIDTH)
}

fn render_workdir_text(workdir: &Path) -> String {
    center_line(
        &theme::muted(&workdir.display().to_string()),
        BANNER_WIDTH,
    )
}

fn render_matrix_easter_egg() -> String {
    center_line(
        &theme::banner_faint("warning: function 'choice_default' is never used"),
        BANNER_WIDTH,
    )
}

/// 渲染完整启动 Banner
pub fn render_banner(info: &BannerInfo<'_>) -> String {
    let mut lines = Vec::new();
    lines.extend(render_colored_block(OUROBOROS));
    lines.push(String::new());
    lines.extend(render_colored_block(TITLE_BLOCK));
    lines.push(render_matrix_easter_egg());
    lines.push(render_title_meta());
    lines.push(render_detail_text(info));
    if let Some(workdir) = info.workdir {
        lines.push(render_workdir_text(workdir));
    }
    lines.join("\n")
}

/// 紧凑 Banner（非 Modern 主题或极窄终端兜底）
fn render_compact_banner(info: &BannerInfo<'_>) -> String {
    const GLYPH: [&str; 3] = ["  ●▀▀▀▀●", " ▐█    █▌", "  ▀▄▄▄▄▀"];
    const GLYPH_COL_WIDTH: usize = 10;
    const GLYPH_GAP: usize = 3;

    let mut text_rows = vec![render_title_meta(), render_detail_text(info)];
    if let Some(workdir) = info.workdir {
        text_rows.push(render_workdir_text(workdir));
    }

    GLYPH
        .iter()
        .zip(text_rows.iter())
        .map(|(glyph, text)| {
            let mut left = color_banner_line(glyph);
            let pad = GLYPH_COL_WIDTH.saturating_sub(glyph.chars().count());
            left.push_str(&" ".repeat(pad));
            format!("{left}{:width$}{text}", "", width = GLYPH_GAP)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 静态 Banner（无上下文兜底）
pub fn banner_static() -> String {
    render_banner(&BannerInfo::default())
}

/// 阻塞版（config setup 等同步上下文）
pub fn play_startup_banner_blocking(info: &BannerInfo<'_>) {
    let output = match theme::current_theme() {
        theme::ThemeId::Modern => match (info.model, info.permission_mode, info.workdir) {
            (None, None, None) => banner_static(),
            _ => render_banner(info),
        },
        _ => render_compact_banner(info),
    };
    println!("{output}");
}

/// 异步版（REPL 启动）
pub async fn play_startup_banner(info: &BannerInfo<'_>) {
    play_startup_banner_blocking(info);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_has_ouroboros_and_title() {
        theme::set_theme(theme::ThemeId::Modern);
        let out = render_banner(&BannerInfo::default());
        assert!(out.contains("ONEMINI [SKILL_SYSTEM]"));
        assert!(out.contains("One"));
        assert!(out.contains("Mini"));
        assert!(out.contains("[CR]"));
        assert!(out.contains("choice_default"));
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
}
