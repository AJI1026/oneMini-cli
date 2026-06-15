use anyhow::Result;
use crate::llm::{ChatMessage, OpenAiClient, UsageInfo};

const COMPRESS_THRESHOLD: usize = 50;
const KEEP_RECENT: usize = 20;

pub fn needs_compression(messages: &[ChatMessage]) -> bool {
    messages.len() > COMPRESS_THRESHOLD
}

/// 将较早的对话压缩为一条摘要消息，保留 system 与最近 KEEP_RECENT 条。
pub async fn compress_messages(
    client: &OpenAiClient,
    messages: &[ChatMessage],
) -> Result<(Vec<ChatMessage>, UsageInfo)> {
    if messages.len() <= COMPRESS_THRESHOLD {
        return Ok((messages.to_vec(), UsageInfo::default()));
    }

    let system = messages
        .first()
        .filter(|m| m.role == "system")
        .cloned();
    let keep_start = messages.len().saturating_sub(KEEP_RECENT);
    let to_summarize = &messages[1..keep_start];

    if to_summarize.is_empty() {
        return Ok((messages.to_vec(), UsageInfo::default()));
    }

    let mut transcript = String::new();
    for msg in to_summarize {
        let role = &msg.role;
        let content = msg.content.as_deref().unwrap_or("[工具调用]");
        transcript.push_str(&format!("[{role}] {content}\n\n"));
    }

    let summary_prompt = format!(
        "请将以下对话历史压缩为简洁的中文摘要，保留：\n\
         - 已完成的工作与改动文件\n\
         - 关键决策与约束\n\
         - 未解决的问题\n\
         不超过 800 字。\n\n\
         ---\n{transcript}"
    );

    let summary_msgs = vec![
        ChatMessage::system("你是会话摘要助手，输出简洁的中文摘要。"),
        ChatMessage::user(summary_prompt),
    ];

    let resp = client.chat_completion(summary_msgs, None).await?;
    let summary = resp.0.content.unwrap_or_default();

    let mut out = Vec::with_capacity(KEEP_RECENT + 2);
    if let Some(sys) = system {
        out.push(sys);
    }
    out.push(ChatMessage::user(format!(
        "[历史摘要 — 较早的 {} 条消息已压缩]\n\n{summary}",
        to_summarize.len()
    )));
    out.extend_from_slice(&messages[keep_start..]);
    Ok((out, UsageInfo::default()))
}
