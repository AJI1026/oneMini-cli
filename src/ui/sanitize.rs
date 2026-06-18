//! 模型输出清洗 — 剥离泄露的标签、元叙述与流式未闭合 Markdown

use std::sync::LazyLock;

use regex::Regex;

static RE_SYSTEM_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<system_instructions>[\s\S]*?</system_instructions>").unwrap());
static RE_THINKING_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<thinking>[\s\S]*?</thinking>").unwrap());
static RE_REDACTED_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<think>[\s\S]*?</think>").unwrap()
});
static RE_ANSWER_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<answer>\s*([\s\S]*?)\s*</answer>").unwrap());
static RE_SKILL_ACTIVATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[技能(?:自动)?启用:[^\]]+\]\s*\n?").unwrap());

static RE_INCOMPLETE_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?is)(?:\
            <system_instructions>[\s\S]*$|\
            <thinking>[\s\S]*$|\
            <think>[\s\S]*$|\
            <answer>[\s\S]*$|\
            <state_update>[\s\S]*$|\
            </?(?:thinking|answer|response|state_update|system_instructions)[^>]*$\
        )",
    )
    .unwrap()
});

/// 流式 delta 是否应转入 reasoning 通道（CoT 误入 content）
pub fn looks_like_reasoning_delta(delta: &str) -> bool {
    let t = delta.trim_start();
    t.starts_with("<thinking>")
        || t.starts_with("<think>")
        || t.starts_with("思考中")
        || t.starts_with("<think>")
}

/// 流式 content delta 清洗（保留增量语义，仅剔除明显泄露片段）
pub fn sanitize_stream_delta(delta: &str) -> String {
    if looks_like_reasoning_delta(delta) {
        return String::new();
    }
    let mut out = delta.to_string();
    out = RE_SYSTEM_BLOCK.replace_all(&out, "").to_string();
    out = RE_THINKING_BLOCK.replace_all(&out, "").to_string();
    out = RE_REDACTED_BLOCK.replace_all(&out, "").to_string();
    out
}

/// 最终输出清洗（finish / 非流式 / JSON 回复）
pub fn sanitize_final(input: &str) -> String {
    let mut text = input.to_string();
    text = RE_SYSTEM_BLOCK.replace_all(&text, "").to_string();
    text = RE_THINKING_BLOCK.replace_all(&text, "").to_string();
    text = RE_REDACTED_BLOCK.replace_all(&text, "").to_string();
    if let Some(caps) = RE_ANSWER_BLOCK.captures(&text) {
        text = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
    }
    text = RE_SKILL_ACTIVATION.replace_all(&text, "").to_string();
    text = strip_meta_lines(&text);
    text = strip_incomplete_tags(&text);
    text = trim_incomplete_markdown(&text);
    normalize_whitespace(&text)
}

fn strip_meta_lines(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return true;
            }
            !t.contains("用户问我")
                && !t.contains("这是一个简单的问题")
                && !t.contains("根据 ONEMINI.md")
                && !(t.contains("直接根据") && t.contains("技能"))
                && t != "思考中"
                && t != "::"
                && t != ":."
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_whitespace(text: &str) -> String {
    let collapsed = collapse_blank_lines(text);
    static RE_MULTI_NL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());
    RE_MULTI_NL
        .replace_all(&collapsed, "\n\n")
        .trim()
        .to_string()
}

fn strip_incomplete_tags(text: &str) -> String {
    RE_INCOMPLETE_TAG.replace_all(text, "").to_string()
}

/// 裁剪流式末尾未闭合 Markdown，避免表格/代码块断裂
pub fn trim_incomplete_markdown(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let mut text = content.to_string();

    let mut fence_indexes = Vec::new();
    let mut search_from = 0;
    while search_from < text.len() {
        if let Some(idx) = text[search_from..].find("```") {
            let abs = search_from + idx;
            fence_indexes.push(abs);
            search_from = abs + 3;
        } else {
            break;
        }
    }
    if fence_indexes.len() % 2 == 1 {
        text.truncate(fence_indexes[fence_indexes.len() - 1]);
    }

    text = trim_unclosed_inline(&text, "**");
    text = trim_unclosed_inline(&text, "__");
    text = trim_unclosed_inline(&text, "*");
    text = trim_unclosed_inline(&text, "`");

    let last_newline = text.rfind('\n');
    let head = last_newline.map(|i| &text[..=i]).unwrap_or("");
    let mut tail = last_newline.map(|i| &text[i + 1..]).unwrap_or(&text);
    tail = trim_incomplete_block_line(tail);
    format!("{head}{tail}")
}

fn trim_unclosed_inline(text: &str, marker: &str) -> String {
    let count = text.matches(marker).count();
    if count % 2 == 1 {
        if let Some(last) = text.rfind(marker) {
            return text[..last].to_string();
        }
    }
    text.to_string()
}

fn trim_incomplete_block_line(line: &str) -> &str {
    let t = line.trim_end();
    if t.starts_with('#') && t.chars().skip_while(|c| *c == '#').next() == Some(' ') && t.trim_end().ends_with('#') {
        return line;
    }
    if regex_is_match(r"^#{1,6}\s*$", t) {
        return "";
    }
    if regex_is_match(r"^[-*+]\s*$", t) {
        return "";
    }
    if regex_is_match(r"^\d+\.\s*$", t) {
        return "";
    }
    if regex_is_match(r"^>\s*$", t) {
        return "";
    }
    if regex_is_match(r"^(-{3,}|\*{3,}|_{3,})\s*$", t) {
        return "";
    }
    line
}

fn regex_is_match(pattern: &str, text: &str) -> bool {
    Regex::new(pattern).map(|re| re.is_match(text)).unwrap_or(false)
}

fn collapse_blank_lines(text: &str) -> String {
    let mut out = String::new();
    let mut prev_blank = false;
    for line in text.lines() {
        let blank = line.trim().is_empty();
        if blank && prev_blank {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
        prev_blank = blank;
    }
    out.trim().to_string()
}

/// 是否含需 Markdown 重绘的结构
pub fn has_markdown_structure(text: &str) -> bool {
    text.contains('#')
        || text.contains("**")
        || text.contains("```")
        || text.contains('|')
        || text.contains("\n- ")
        || text.contains("\n* ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_system_instructions_block() {
        let raw = "hello\n<system_instructions>secret</system_instructions>\nworld";
        let out = sanitize_final(raw);
        assert!(out.contains("hello"));
        assert!(out.contains("world"));
        assert!(!out.contains("system_instructions"));
    }

    #[test]
    fn strips_thinking_block() {
        let raw = "<thinking>internal</thinking>\n\n正式回答";
        assert_eq!(sanitize_final(raw), "正式回答");
    }

    #[test]
    fn extracts_answer_tag() {
        let raw = "<thinking>x</thinking><answer>用户可见</answer>";
        assert_eq!(sanitize_final(raw), "用户可见");
    }

    #[test]
    fn removes_meta_narration_lines() {
        let raw = "用户问我有哪些skill，这是一个简单的问题。\n\n| A | B |\n|---|---|\n| 1 | 2 |";
        let out = sanitize_final(raw);
        assert!(!out.contains("用户问我"));
        assert!(!out.contains("简单的问题"));
        assert!(out.contains("| A | B |"));
    }

    #[test]
    fn trim_incomplete_fence() {
        let raw = "text\n```rust\nfn main()";
        let out = trim_incomplete_markdown(raw);
        assert!(!out.contains("```"));
        assert!(out.contains("text"));
    }

    #[test]
    fn preserves_valid_table() {
        let md = "| 列A | 列B |\n| --- | --- |\n| 1 | 2 |";
        assert_eq!(sanitize_final(md), md);
    }

    #[test]
    fn reasoning_delta_detection() {
        assert!(looks_like_reasoning_delta("<thinking>"));
        assert!(!looks_like_reasoning_delta("正常输出"));
    }
}
