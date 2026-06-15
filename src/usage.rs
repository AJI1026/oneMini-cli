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
    pub fn add(&mut self, usage: &UsageInfo) {
        self.prompt_tokens += usage.prompt_tokens as u64;
        self.completion_tokens += usage.completion_tokens as u64;
        self.reasoning_tokens += usage.reasoning_tokens as u64;
        self.turns += 1;
    }

    pub fn format_turn(&self, usage: &UsageInfo, model: &str) -> String {
        let cost = estimate_cost(model, usage);
        let cost_str = cost
            .map(|c| format!(" · ~${c:.4}"))
            .unwrap_or_default();
        format!(
            "⎿ 输入 {} · 输出 {}{} · 合计 {}",
            usage.prompt_tokens,
            usage.completion_tokens,
            cost_str,
            usage.total()
        )
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
            self.prompt_tokens, self.completion_tokens, self.turns, cost
        )
    }
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
    let output = (usage.completion_tokens + usage.reasoning_tokens) as f64 * output_rate / 1_000_000.0;
    Some(input + output)
}
