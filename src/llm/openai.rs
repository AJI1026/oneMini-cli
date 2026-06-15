use anyhow::{Context, Result};
use async_stream::stream;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use super::types::*;
use crate::config::Config;

pub struct OpenAiClient {
    client: Client,
    base_url: String,
    api_key: String,
    config: Config,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponseWithUsage {
    choices: Vec<Choice>,
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<StreamToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCallDelta {
    index: usize,
    id: Option<String>,
    function: Option<StreamFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

impl OpenAiClient {
    pub fn new(config: &Config) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .filter(|k| !k.is_empty())
            .context("API Key 未配置")?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".into())
            .trim_end_matches('/')
            .to_string();

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()?,
            base_url,
            api_key,
            config: config.clone(),
        })
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<(AssistantMessage, UsageInfo)> {
        let req = ChatRequest {
            model: self
                .config
                .model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".into()),
            messages,
            tools,
            temperature: self.config.temperature.unwrap_or(0.2),
            max_tokens: self.config.max_tokens.unwrap_or(8192),
            stream: false,
        };

        let resp = self
            .client
            .post(self.chat_url())
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .context("请求 LLM API 失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API 错误 {status}: {body}");
        }

        let data: ChatCompletionResponseWithUsage =
            resp.json().await.context("解析 LLM 响应失败")?;
        let msg = data
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .context("LLM 响应为空")?;
        Ok((msg, data.usage.unwrap_or_default()))
    }

    pub async fn chat_completion_stream(
        &self,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<impl futures::Stream<Item = StreamEvent> + use<>> {
        let req = ChatRequest {
            model: self
                .config
                .model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".into()),
            messages,
            tools,
            temperature: self.config.temperature.unwrap_or(0.2),
            max_tokens: self.config.max_tokens.unwrap_or(8192),
            stream: true,
        };

        let resp = self
            .client
            .post(self.chat_url())
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .context("请求 LLM 流式 API 失败")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API 错误 {status}: {body}");
        }

        let byte_stream = resp.bytes_stream();
        let stream = stream! {
            let mut buffer = String::new();
            let mut content = String::new();
            let mut reasoning = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut last_usage = UsageInfo::default();

            futures::pin_mut!(byte_stream);
            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield StreamEvent::Error(e.to_string());
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();
                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }
                    let data = line.strip_prefix("data:").map(str::trim).unwrap_or(&line);
                    if data == "[DONE]" {
                        if last_usage.total() > 0 {
                            yield StreamEvent::Usage(last_usage.clone());
                        }
                        let msg = AssistantMessage {
                            role: "assistant".into(),
                            content: if content.is_empty() { None } else { Some(content.clone()) },
                            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls.clone()) },
                            reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning.clone()) },
                        };
                        yield StreamEvent::Done(msg);
                        return;
                    }
                    let parsed: Result<StreamChunk, _> = serde_json::from_str(data);
                    let chunk = match parsed {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if let Some(usage) = chunk.usage {
                        last_usage = usage;
                    }
                    for choice in chunk.choices {
                        if let Some(text) = choice.delta.reasoning_content {
                            if !text.is_empty() {
                                reasoning.push_str(&text);
                                yield StreamEvent::ReasoningDelta(text);
                            }
                        }
                        if let Some(text) = choice.delta.content {
                            if !text.is_empty() {
                                content.push_str(&text);
                                yield StreamEvent::ContentDelta(text);
                            }
                        }
                        if let Some(deltas) = choice.delta.tool_calls {
                            for d in deltas {
                                while tool_calls.len() <= d.index {
                                    tool_calls.push(ToolCall {
                                        id: String::new(),
                                        call_type: "function".into(),
                                        function: FunctionCall {
                                            name: String::new(),
                                            arguments: String::new(),
                                        },
                                    });
                                }
                                if let Some(id) = d.id.clone() {
                                    tool_calls[d.index].id = id;
                                }
                                if let Some(func) = &d.function {
                                    if let Some(name) = &func.name {
                                        tool_calls[d.index].function.name = name.clone();
                                    }
                                    if let Some(args) = &func.arguments {
                                        tool_calls[d.index].function.arguments.push_str(args);
                                    }
                                }
                                yield StreamEvent::ToolCallDelta {
                                    name: d.function.as_ref().and_then(|f| f.name.clone()),
                                };
                            }
                        }
                        if choice.finish_reason.is_some() {
                            if last_usage.total() > 0 {
                                yield StreamEvent::Usage(last_usage.clone());
                            }
                            let msg = AssistantMessage {
                                role: "assistant".into(),
                                content: if content.is_empty() { None } else { Some(content.clone()) },
                                tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls.clone()) },
                                reasoning_content: if reasoning.is_empty() { None } else { Some(reasoning.clone()) },
                            };
                            yield StreamEvent::Done(msg);
                            return;
                        }
                    }
                }
            }
        };

        Ok(stream)
    }
}
