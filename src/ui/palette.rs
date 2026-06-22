//! 设计令牌 — 与 docs/assets/*.svg / README 配图色值对齐

/// RGB 三元组
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            (self.0 as f32 + (other.0 as f32 - self.0 as f32) * t).round() as u8,
            (self.1 as f32 + (other.1 as f32 - self.1 as f32) * t).round() as u8,
            (self.2 as f32 + (other.2 as f32 - self.2 as f32) * t).round() as u8,
        )
    }
}

/// 单主题完整色板（部分字段为 SVG 设计令牌，运行时未必全部引用）
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {
    /// 终端窗口背景近似色 #0a0f0a
    pub bg: Rgb,
    /// Welcome / 卡片填充 #0d1a0d
    pub panel_fill: Rgb,
    /// 工具卡填充 #0f1a0f
    pub tool_fill: Rgb,
    /// 面板边框 #2a4a2a
    pub panel_border: Rgb,
    /// 主发光色 #39ff14
    pub glow_hi: Rgb,
    /// 次发光色 #7fff5a
    pub glow_mid: Rgb,
    /// 正文高亮 #d4ffd4
    pub text_bright: Rgb,
    /// 次要文字 #5a8a5a
    pub text_muted: Rgb,
    /// 更暗说明 #4a6a4a
    pub text_dim: Rgb,
    /// 渐变暗端
    pub gradient_lo: Rgb,
    /// 渐变亮端
    pub gradient_hi: Rgb,
    /// 警告面板填充 #1a1208
    pub warn_fill: Rgb,
    /// 警告边框 #6a4a1a
    pub warn_border: Rgb,
    /// 警告文字 #febc2e
    pub warn_text: Rgb,
    /// 警告次要 #c4a46a
    pub warn_muted: Rgb,
    /// macOS 窗口按钮
    pub chrome_close: Rgb,
    pub chrome_minimize: Rgb,
    pub chrome_maximize: Rgb,
    /// 用户输入前缀色
    pub user_prompt: Rgb,
    /// 助手前缀色
    pub assistant_prompt: Rgb,
}

impl Palette {
    pub fn for_theme_index(v: u8) -> Self {
        match v {
            1 => PALETTE_GAMEBOY,
            2 => PALETTE_NES,
            _ => PALETTE_MODERN,
        }
    }

    pub fn brand_gradient_stops(self) -> [Rgb; 3] {
        [self.gradient_lo, self.glow_hi, self.gradient_hi]
    }
}

/// Modern — hero.svg / session.svg
pub const PALETTE_MODERN: Palette = Palette {
    bg: Rgb(10, 15, 10),
    panel_fill: Rgb(13, 26, 13),
    tool_fill: Rgb(15, 26, 15),
    panel_border: Rgb(42, 74, 42),
    glow_hi: Rgb(57, 255, 20),
    glow_mid: Rgb(127, 255, 90),
    text_bright: Rgb(212, 255, 212),
    text_muted: Rgb(90, 138, 90),
    text_dim: Rgb(74, 106, 74),
    gradient_lo: Rgb(0, 140, 45),
    gradient_hi: Rgb(0, 255, 160),
    warn_fill: Rgb(26, 18, 8),
    warn_border: Rgb(106, 74, 26),
    warn_text: Rgb(254, 188, 46),
    warn_muted: Rgb(196, 164, 106),
    chrome_close: Rgb(255, 95, 87),
    chrome_minimize: Rgb(254, 188, 46),
    chrome_maximize: Rgb(40, 200, 64),
    user_prompt: Rgb(127, 255, 90),
    assistant_prompt: Rgb(57, 255, 20),
};

/// Game Boy — themes.svg 左卡
pub const PALETTE_GAMEBOY: Palette = Palette {
    bg: Rgb(155, 188, 15),
    panel_fill: Rgb(155, 188, 15),
    tool_fill: Rgb(139, 172, 15),
    panel_border: Rgb(15, 56, 15),
    glow_hi: Rgb(15, 56, 15),
    glow_mid: Rgb(48, 98, 48),
    text_bright: Rgb(15, 56, 15),
    text_muted: Rgb(48, 98, 48),
    text_dim: Rgb(48, 98, 48),
    gradient_lo: Rgb(15, 56, 15),
    gradient_hi: Rgb(210, 245, 90),
    warn_fill: Rgb(139, 172, 15),
    warn_border: Rgb(15, 56, 15),
    warn_text: Rgb(15, 56, 15),
    warn_muted: Rgb(48, 98, 48),
    chrome_close: Rgb(15, 56, 15),
    chrome_minimize: Rgb(48, 98, 48),
    chrome_maximize: Rgb(15, 56, 15),
    user_prompt: Rgb(15, 56, 15),
    assistant_prompt: Rgb(15, 56, 15),
};

/// NES — themes.svg 右卡
pub const PALETTE_NES: Palette = Palette {
    bg: Rgb(28, 44, 110),
    panel_fill: Rgb(28, 44, 110),
    tool_fill: Rgb(0, 0, 170),
    panel_border: Rgb(104, 136, 200),
    glow_hi: Rgb(248, 248, 248),
    glow_mid: Rgb(168, 184, 232),
    text_bright: Rgb(248, 248, 248),
    text_muted: Rgb(168, 184, 232),
    text_dim: Rgb(104, 136, 200),
    gradient_lo: Rgb(80, 80, 180),
    gradient_hi: Rgb(255, 220, 80),
    warn_fill: Rgb(0, 0, 170),
    warn_border: Rgb(104, 136, 200),
    warn_text: Rgb(255, 216, 120),
    warn_muted: Rgb(168, 184, 232),
    chrome_close: Rgb(255, 95, 87),
    chrome_minimize: Rgb(254, 188, 46),
    chrome_maximize: Rgb(40, 200, 64),
    user_prompt: Rgb(248, 248, 248),
    assistant_prompt: Rgb(248, 248, 248),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_glow_matches_svg() {
        let p = PALETTE_MODERN;
        assert_eq!(p.glow_hi, Rgb(57, 255, 20));
        assert_eq!(p.glow_mid, Rgb(127, 255, 90));
        assert_eq!(p.panel_fill, Rgb(13, 26, 13));
    }

    #[test]
    fn gameboy_panel_bg_matches_svg() {
        assert_eq!(PALETTE_GAMEBOY.panel_fill, Rgb(155, 188, 15));
    }

    #[test]
    fn nes_panel_bg_matches_svg() {
        assert_eq!(PALETTE_NES.panel_fill, Rgb(28, 44, 110));
    }
}
