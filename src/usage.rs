use serde::{Deserialize, Serialize};

use crate::llm::UsageInfo;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub reasoning_tokens: u64,
    pub turns: u32,
}

impl SessionUsage {
    /// 累加单次 API 调用的 token（不增加轮次计数）
    pub fn accumulate(&mut self, usage: &UsageInfo) {
        self.prompt_tokens += usage.prompt_tokens as u64;
        self.completion_tokens += usage.completion_tokens as u64;
        self.reasoning_tokens += usage.reasoning_tokens as u64;
    }

    /// 完成一轮用户提问
    pub fn finish_turn(&mut self) {
        self.turns += 1;
    }

    pub fn format_session(&self, model: &str) -> String {
        let usage = UsageInfo {
            prompt_tokens: self.prompt_tokens.min(u32::MAX as u64) as u32,
            completion_tokens: self.completion_tokens.min(u32::MAX as u64) as u32,
            reasoning_tokens: self.reasoning_tokens.min(u32::MAX as u64) as u32,
            total_tokens: 0,
        };
        let cost = estimate_cost(model, &usage)
            .map(|c| format!(" · 会话累计 ~${c:.4}"))
            .unwrap_or_default();
        format!(
            "令牌用量: 输入 {} · 输出 {} · {} 轮{}",
            format_token_count(self.prompt_tokens),
            format_token_count(self.completion_tokens + self.reasoning_tokens),
            self.turns,
            cost
        )
    }
}

/// 单轮用量视图（含进度条）
pub struct TurnUsageView<'a> {
    usage: &'a UsageInfo,
    model: &'a str,
    context_tokens: u32,
}

impl<'a> TurnUsageView<'a> {
    pub fn new(usage: &'a UsageInfo, model: &'a str, context_tokens: u32) -> Self {
        Self {
            usage,
            model,
            context_tokens,
        }
    }

    pub fn stats_line(&self) -> String {
        let cost = estimate_cost(self.model, self.usage)
            .map(|c| format!(" · ~${c:.4}"))
            .unwrap_or_default();
        let output_total = self
            .usage
            .completion_tokens
            .saturating_add(self.usage.reasoning_tokens);
        let mut parts = vec![
            format!(
                "输入 {}",
                format_token_count(self.usage.prompt_tokens as u64)
            ),
            format!("输出 {}", format_token_count(output_total as u64)),
        ];
        if self.usage.reasoning_tokens > 0 {
            parts.push(format!(
                "推理 {}",
                format_token_count(self.usage.reasoning_tokens as u64)
            ));
        }
        parts.push(format!(
            "合计 {}",
            format_token_count(self.usage.total() as u64)
        ));
        format!("{}{}", parts.join(" · "), cost)
    }

    pub fn progress_bar(&self, width: usize) -> ProgressBarView {
        ProgressBarView {
            label: "上下文",
            used: self.context_tokens,
            limit: model_context_limit(self.model),
            width,
        }
    }
}

pub struct ProgressBarView {
    pub label: &'static str,
    pub used: u32,
    pub limit: u32,
    pub width: usize,
}

impl ProgressBarView {
    pub fn ratio(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        (self.used as f64 / self.limit as f64).clamp(0.0, 1.0)
    }

    pub fn percent(&self) -> u32 {
        (self.ratio() * 100.0).round() as u32
    }

    pub fn filled_count(&self) -> usize {
        ((self.width as f64) * self.ratio()).round() as usize
    }

    pub fn bar_plain(&self) -> String {
        let filled = self.filled_count();
        let empty = self.width.saturating_sub(filled);
        format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
    }

    pub fn caption(&self) -> String {
        format!(
            "{} · {} / {}",
            format_percent(self.percent()),
            format_token_short(self.used),
            format_token_short(self.limit)
        )
    }
}

pub fn model_context_limit(model: &str) -> u32 {
    let m = model.to_lowercase();
    if m.contains("deepseek") {
        64_000
    } else if m.contains("claude") {
        200_000
    } else {
        128_000
    }
}

pub fn format_token_count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub fn format_token_short(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else if n >= 1_000 {
        format!("{:.2}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_percent(p: u32) -> String {
    format!("{p}%")
}

fn model_pricing(model: &str) -> Option<(f64, f64)> {
    let m = model.to_lowercase();
    if m.contains("deepseek-reasoner") {
        Some((0.55, 2.19))
    } else if m.contains("deepseek") {
        Some((0.14, 0.28))
    } else if m.contains("gpt-4o-mini") {
        Some((0.15, 0.60))
    } else if m.contains("gpt-4o") {
        Some((2.50, 10.0))
    } else if m.contains("claude") {
        Some((3.0, 15.0))
    } else {
        None
    }
}

pub fn estimate_cost(model: &str, usage: &UsageInfo) -> Option<f64> {
    let (input_rate, output_rate) = model_pricing(model)?;
    let input = usage.prompt_tokens as f64 * input_rate / 1_000_000.0;
    let output = (usage.completion_tokens + usage.reasoning_tokens) as f64 * output_rate
        / 1_000_000.0;
    Some(input + output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fills_correctly() {
        let bar = ProgressBarView {
            label: "上下文",
            used: 32_000,
            limit: 128_000,
            width: 20,
        };
        assert_eq!(bar.percent(), 25);
        assert_eq!(bar.filled_count(), 5);
    }

    #[test]
    fn format_token_count_commas() {
        assert_eq!(format_token_count(1234567), "1,234,567");
    }
}
