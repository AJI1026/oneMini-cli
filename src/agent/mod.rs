mod prompt;

use anyhow::{Context, Result};
use futures::StreamExt;
use serde_json::Value;
use std::io::{self, Write};
use crate::config::Config;
use crate::llm::{AssistantMessage, ChatMessage, OpenAiClient, StreamEvent, ToolCall};
use crate::tools::ToolRegistry;
use crate::ui;

pub use prompt::build_system_prompt;

#[derive(Clone)]
pub struct AgentOptions {
    pub config: Config,
    pub max_rounds: u32,
    pub auto_approve: bool,
}

pub struct AgentSession {
    client: OpenAiClient,
    registry: ToolRegistry,
    messages: Vec<ChatMessage>,
    pub opts: AgentOptions,
}

impl AgentSession {
    pub fn new(opts: AgentOptions) -> Result<Self> {
        let workdir = opts.config.workdir().to_path_buf();
        std::env::set_current_dir(&workdir)
            .with_context(|| format!("无法进入工作目录: {}", workdir.display()))?;

        let system = build_system_prompt(&workdir);
        let client = OpenAiClient::new(&opts.config)?;
        let registry = ToolRegistry::new(workdir);

        Ok(Self {
            client,
            registry,
            messages: vec![ChatMessage::system(system)],
            opts,
        })
    }

    pub async fn run_turn(&mut self, user_input: &str, stream: bool) -> Result<String> {
        self.messages.push(ChatMessage::user(user_input));

        let mut rounds = 0u32;
        loop {
            rounds += 1;
            if rounds > self.opts.max_rounds {
                anyhow::bail!("已达最大工具调用轮次 ({})", self.opts.max_rounds);
            }

            let tools = Some(self.registry.definitions());
            let assistant = if stream {
                self.run_stream_round(tools).await?
            } else {
                self.client
                    .chat_completion(self.messages.clone(), tools)
                    .await?
            };

            let tool_calls = assistant.tool_calls.clone().unwrap_or_default();
            if tool_calls.is_empty() {
                let content = assistant.content.unwrap_or_default();
                self.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: Some(content.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                });
                return Ok(content);
            }

            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: assistant.content.clone(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
            });

            for call in tool_calls {
                let result = self.execute_tool_call(&call).await?;
                self.messages.push(ChatMessage::tool_result(&call.id, result));
            }
        }
    }

    async fn run_stream_round(
        &mut self,
        tools: Option<Vec<crate::llm::ToolDefinition>>,
    ) -> Result<AssistantMessage> {
        let stream = self
            .client
            .chat_completion_stream(self.messages.clone(), tools)
            .await?;
        futures::pin_mut!(stream);

        let mut streamed_text = String::new();
        let mut final_msg: Option<AssistantMessage> = None;
        let mut streaming = false;

        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::ContentDelta(delta) => {
                    streamed_text.push_str(&delta);
                    if !streaming {
                        print!("{} ", ui::assistant_prefix());
                        streaming = true;
                    }
                    let chars = streamed_text.chars().count();
                    print!(
                        "\r{} {}{} ",
                        ui::assistant_prefix(),
                        ui::thinking_label(),
                        ui::dim(&format!("({chars} 字)"))
                    );
                    io::stdout().flush().ok();
                }
                StreamEvent::ToolCallDelta { name, .. } => {
                    if streaming {
                        print!("\r\x1b[2K");
                        streaming = false;
                    }
                    if let Some(n) = name {
                        println!("{}", ui::tool_call(&n, "准备调用…"));
                    }
                }
                StreamEvent::Done(msg) => {
                    let content = msg
                        .content
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&streamed_text);
                    if !content.is_empty() {
                        print!("\r\x1b[2K");
                        println!(
                            "{} {}",
                            ui::assistant_prefix(),
                            ui::render_markdown(content)
                        );
                    } else if streaming {
                        print!("\r\x1b[2K\n");
                    }
                    final_msg = Some(msg);
                }
                StreamEvent::Error(e) => anyhow::bail!("流式错误: {e}"),
            }
        }

        final_msg.context("流式响应未正常结束")
    }

    async fn execute_tool_call(&self, call: &ToolCall) -> Result<String> {
        let name = &call.function.name;
        let args: Value = serde_json::from_str(&call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));

        let tool = self
            .registry
            .get(name)
            .with_context(|| format!("未知工具: {name}"))?;

        let detail = summarize_args(name, &args);
        println!("{}", ui::tool_call(name, &detail));

        if tool.requires_approval(&args) && !self.opts.auto_approve {
            print!("{} 允许执行? [y/N] ", ui::warn("权限"));
            io::stdout().flush()?;
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                return Ok("[用户拒绝执行]".into());
            }
        }

        match tool.execute(args).await {
            Ok(out) => {
                println!("{}", ui::dim(&truncate_preview(&out, 200)));
                Ok(out)
            }
            Err(e) => {
                let err = format!("[tool error] {e}");
                println!("{}", ui::error(&err));
                Ok(err)
            }
        }
    }
}

pub async fn run_agent(opts: &AgentOptions, prompt: &str) -> Result<String> {
    let mut session = AgentSession::new(opts.clone())?;
    session.run_turn(prompt, false).await
}

fn summarize_args(name: &str, args: &Value) -> String {
    match name {
        "read" | "write" | "edit" => args["path"].as_str().unwrap_or("?").into(),
        "grep" => format!(
            "pattern={}",
            args["pattern"].as_str().unwrap_or("?")
        ),
        "glob" => format!(
            "pattern={}",
            args["pattern"].as_str().unwrap_or("?")
        ),
        "bash" => {
            let cmd = args["command"].as_str().unwrap_or("?");
            if cmd.len() > 60 {
                format!("{}…", &cmd[..60])
            } else {
                cmd.into()
            }
        }
        _ => args.to_string(),
    }
}

fn truncate_preview(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
