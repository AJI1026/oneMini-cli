use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;

use crate::config::Config;
use crate::llm::{ChatMessage, OpenAiClient, ToolCall};
use crate::permissions::PermissionMode;
use crate::sandbox::SandboxRunner;
use crate::tools::ToolRegistry;
use crate::worktree::GitWorktree;

const MAX_SUBAGENT_ROUNDS: u32 = 8;

/// 运行独立子任务（只读工具自动批准）。
pub async fn run_subtask(workdir: PathBuf, prompt: &str) -> Result<String> {
    run_subtask_inner(workdir, prompt).await
}

/// 在 git worktree 隔离目录中运行只读子任务。
pub async fn run_subtask_isolated(repo_root: PathBuf, prompt: &str) -> Result<String> {
    let wt = GitWorktree::create(&repo_root, "delegate")?;
    let result = run_subtask_inner(wt.path.clone(), prompt).await;
    let _ = wt.remove();
    result
}

async fn run_subtask_inner(workdir: PathBuf, prompt: &str) -> Result<String> {
    let mut config = Config::load()?;
    config.workdir = Some(workdir.clone());

    let client = OpenAiClient::new(&config)?;
    let sandbox = SandboxRunner::new(&config.sandbox);
    let registry = ToolRegistry::new(workdir.clone(), sandbox);

    let system = format!(
        "你是 OneMini 子 Agent，专注完成单一子任务。\n\
         工作目录: {}\n\
         只能使用 read/grep/glob 搜索分析，不要修改文件或执行 shell。",
        workdir.display()
    );
    let mut messages = vec![
        ChatMessage::system(system),
        ChatMessage::user(prompt),
    ];

    for _ in 0..MAX_SUBAGENT_ROUNDS {
        let tools = Some(registry.definitions_for_mode(PermissionMode::Plan));
        let (assistant, _) = client.chat_completion(messages.clone(), tools).await?;
        let tool_calls = assistant.tool_calls.clone().unwrap_or_default();

        if tool_calls.is_empty() {
            return Ok(assistant.content.unwrap_or_default());
        }

        messages.push(ChatMessage {
            role: "assistant".into(),
            content: assistant.content.clone(),
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
            name: None,
        });

        for call in tool_calls {
            let result = execute_readonly_tool(&registry, &call).await?;
            messages.push(ChatMessage::tool_result(&call.id, result));
        }
    }

    anyhow::bail!("子 Agent 超过最大轮次")
}

async fn execute_readonly_tool(registry: &ToolRegistry, call: &ToolCall) -> Result<String> {
    let name = &call.function.name;
    if !matches!(name.as_str(), "read" | "grep" | "glob") {
        return Ok(format!("[子 Agent 不允许使用工具 {name}]"));
    }
    let args: Value = serde_json::from_str(&call.function.arguments)
        .unwrap_or_else(|_| serde_json::json!({}));
    let tool = registry
        .get(name)
        .with_context(|| format!("未知工具: {name}"))?;
    tool.execute(args).await.map_err(Into::into)
}
