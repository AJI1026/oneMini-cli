mod prompt;
mod task;

use anyhow::{Context, Result};
use futures::StreamExt;
use serde_json::Value;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use crate::compress::{compress_messages, needs_compression};
use crate::config::Config;
use crate::git::GitManager;
use crate::hooks::HookRunner;
use crate::llm::{AssistantMessage, ChatMessage, OpenAiClient, StreamEvent, ToolCall, UsageInfo};
use crate::mcp::McpRegistry;
use crate::permissions::{PermissionDecision, PermissionManager};
use crate::session::SessionStore;
use crate::tools::{Tool, ToolRegistry};
use crate::ui::{self, StreamRenderer};
use crate::usage::SessionUsage;

pub use prompt::build_system_prompt;
pub use task::TaskState;

#[derive(Clone)]
pub struct AgentOptions {
    pub config: Config,
    pub max_rounds: u32,
    pub auto_approve: bool,
    pub resume: bool,
}

pub struct AgentSession {
    client: OpenAiClient,
    registry: ToolRegistry,
    messages: Vec<ChatMessage>,
    pub opts: AgentOptions,
    pub task_state: TaskState,
    pub session_usage: SessionUsage,
    session_store: SessionStore,
    permissions: PermissionManager,
    hooks: HookRunner,
    git: GitManager,
    had_tool_calls_this_turn: bool,
    last_turn_usage: UsageInfo,
}

impl AgentSession {
    pub async fn new(opts: AgentOptions) -> Result<Self> {
        let workdir = opts.config.workdir().to_path_buf();
        std::env::set_current_dir(&workdir)
            .with_context(|| format!("无法进入工作目录: {}", workdir.display()))?;

        let session_store = SessionStore::new()?;
        let (messages, task_state, session_usage) = if opts.resume {
            if let Some(saved) = session_store.load()? {
                if saved.workdir == workdir {
                    (
                        saved.messages,
                        saved.task_state,
                        saved.session_usage,
                    )
                } else {
                    Self::fresh_messages(&workdir)
                }
            } else {
                Self::fresh_messages(&workdir)
            }
        } else {
            Self::fresh_messages(&workdir)
        };

        let client = OpenAiClient::new(&opts.config)?;
        let mut registry = ToolRegistry::new(workdir.clone());
        registry.register(Arc::new(crate::tools::DelegateTool::new(workdir.clone())));

        if !opts.config.mcp_servers.is_empty() {
            match McpRegistry::connect_all(&opts.config.mcp_servers).await {
                Ok(mcp) => registry.extend(mcp.tools()),
                Err(e) => eprintln!("{}", ui::warn(&format!("MCP 初始化失败: {e}"))),
            }
        }

        Ok(Self {
            client,
            registry,
            messages,
            opts,
            task_state,
            session_usage,
            session_store,
            permissions: PermissionManager::load()?,
            hooks: HookRunner::load()?,
            git: GitManager::new(workdir),
            had_tool_calls_this_turn: false,
            last_turn_usage: UsageInfo::default(),
        })
    }

    fn fresh_messages(workdir: &Path) -> (Vec<ChatMessage>, TaskState, SessionUsage) {
        let system = build_system_prompt(workdir);
        (
            vec![ChatMessage::system(system)],
            TaskState::default(),
            SessionUsage::default(),
        )
    }

    pub fn workdir(&self) -> &Path {
        self.opts.config.workdir()
    }

    pub fn format_status(&self) -> String {
        let body = self.task_state.format_status();
        let mut out = format!("{}\n", ui::render_plan_text(&body));
        if self.last_turn_usage.total() > 0 {
            out.push_str(&format!(
                "{}\n",
                ui::usage_line(&self.session_usage.format_turn(
                    &self.last_turn_usage,
                    self.opts.config.model_name(),
                ))
            ));
        }
        out.push_str(&format!(
            "{}\n",
            ui::usage_line(&self.session_usage.format_session(
                self.opts.config.model_name()
            ))
        ));
        if let Some(cp) = self.git.last_checkpoint() {
            out.push_str(&format!(
                "{}\n",
                ui::status_pair("Git 检查点", &format!("{}…", &cp[..cp.len().min(8)]))
            ));
        }
        out
    }

    pub fn persist(&self) -> Result<()> {
        self.session_store.save(
            self.workdir(),
            &self.messages,
            &self.task_state,
            &self.session_usage,
        )
    }

    pub fn clear_persisted(&self) -> Result<()> {
        self.session_store.clear()
    }

    pub async fn compact_history(&mut self) -> Result<()> {
        if !needs_compression(&self.messages) {
            return Ok(());
        }
        let (compressed, _) = compress_messages(&self.client, &self.messages).await?;
        self.messages = compressed;
        self.persist()?;
        Ok(())
    }

    pub fn rollback_git(&mut self) -> Result<String> {
        let hash = self
            .git
            .last_checkpoint()
            .context("没有可回滚的检查点")?
            .to_string();
        self.git.rollback_checkpoint()?;
        Ok(hash)
    }

    pub async fn run_turn(&mut self, user_input: &str, stream: bool) -> Result<String> {
        let workdir = self.workdir().to_path_buf();
        self.task_state.begin_turn(user_input, &workdir);
        self.had_tool_calls_this_turn = false;
        self.last_turn_usage = UsageInfo::default();

        if needs_compression(&self.messages) {
            let (compressed, _) = compress_messages(&self.client, &self.messages).await?;
            self.messages = compressed;
            if stream {
                println!("{}", ui::dim("… 历史消息已自动压缩"));
            }
        }

        if let Some(ctx) = self.task_state.turn_context_block() {
            self.messages.push(ChatMessage::user(format!(
                "{ctx}\n\n用户请求:\n{user_input}"
            )));
        } else {
            self.messages.push(ChatMessage::user(user_input));
        }

        let mut rounds = 0u32;
        loop {
            rounds += 1;
            if rounds > self.opts.max_rounds {
                self.task_state
                    .last_errors
                    .push(format!("已达最大工具调用轮次 ({})", self.opts.max_rounds));
                anyhow::bail!("已达最大工具调用轮次 ({})", self.opts.max_rounds);
            }

            let tools = Some(self.registry.definitions());
            let assistant = if stream {
                self.run_stream_round(tools).await?
            } else {
                let (msg, usage) = self
                    .client
                    .chat_completion(self.messages.clone(), tools)
                    .await?;
                self.record_usage(&usage);
                msg
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
                self.task_state
                    .advance_after_turn(self.had_tool_calls_this_turn);
                let summary = self.task_state.finish_summary();
                let final_content = if summary.is_empty() {
                    content
                } else {
                    format!("{content}{summary}")
                };
                if stream {
                    ui::print_usage_line(
                        &self
                            .session_usage
                            .format_turn(&self.last_turn_usage, self.opts.config.model_name()),
                    );
                }
                let _ = self.hooks.run_notification(&format!(
                    "回合完成 · {} tokens",
                    self.last_turn_usage.total()
                ));
                self.persist()?;
                return Ok(final_content);
            }

            self.had_tool_calls_this_turn = true;
            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: assistant.content.clone(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
            });

            for call in tool_calls {
                let result = self.execute_tool_call(&call, stream).await?;
                self.messages
                    .push(ChatMessage::tool_result(&call.id, result));
            }
        }
    }

    pub async fn retry_last_failure(&mut self, stream: bool) -> Result<String> {
        let prompt = self
            .task_state
            .retry_prompt()
            .context("当前没有可重试的失败步骤")?;
        self.run_turn(&prompt, stream).await
    }

    fn record_usage(&mut self, usage: &UsageInfo) {
        if usage.total() > 0 {
            self.last_turn_usage = usage.clone();
            self.session_usage.add(usage);
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

        let show_reasoning = self.opts.config.show_reasoning();
        let mut renderer = StreamRenderer::new(show_reasoning);
        let mut final_msg: Option<AssistantMessage> = None;

        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::ReasoningDelta(delta) => {
                    renderer.on_reasoning_delta(&delta);
                }
                StreamEvent::ContentDelta(delta) => {
                    renderer.on_content_delta(&delta);
                }
                StreamEvent::ToolCallDelta { name, .. } => {
                    if let Some(n) = name {
                        renderer.on_tool_call(&n, "准备调用…");
                    }
                }
                StreamEvent::Usage(usage) => {
                    self.record_usage(&usage);
                }
                StreamEvent::Done(msg) => {
                    let content = msg.content.as_deref().filter(|s| !s.is_empty());
                    let has_tools = msg.tool_calls.as_ref().is_some_and(|t| !t.is_empty());
                    if has_tools {
                        renderer.finish_tool_round();
                    } else {
                        renderer.finish(content);
                    }
                    final_msg = Some(msg);
                }
                StreamEvent::Error(e) => anyhow::bail!("流式错误: {e}"),
            }
        }

        final_msg.context("流式响应未正常结束")
    }

    async fn execute_tool_call(&mut self, call: &ToolCall, verbose: bool) -> Result<String> {
        let name = &call.function.name;
        let args: Value = serde_json::from_str(&call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));

        let tool = self
            .registry
            .get(name)
            .with_context(|| format!("未知工具: {name}"))?;

        let detail = summarize_args(name, &args);
        if verbose {
            println!("{}", ui::tool_call(name, &detail));
        }

        self.hooks.run_pre_tool(name, &args).ok();

        if self.maybe_git_checkpoint(name, &args, verbose)? {
            // checkpoint created
        }

        if !self.check_permission(&tool, &detail, &args, verbose)? {
            return Ok("[用户拒绝执行]".into());
        }

        match tool.execute(args.clone()).await {
            Ok(out) => {
                self.hooks.run_post_tool(name, &args, &out).ok();
                self.track_tool_outcome(name, &args, &out);
                if verbose {
                    println!("{}", ui::dim(&truncate_preview(&out, 200)));
                }
                Ok(out)
            }
            Err(e) => {
                let err = format!("[tool error] {e}");
                self.hooks.run_post_tool(name, &args, &err).ok();
                self.track_tool_error(name, &args, &err);
                if verbose {
                    println!("{}", ui::error(&err));
                }
                Ok(err)
            }
        }
    }

    fn maybe_git_checkpoint(
        &mut self,
        name: &str,
        _args: &Value,
        verbose: bool,
    ) -> Result<bool> {
        if !self.opts.config.auto_git_checkpoint() || !self.git.is_repo() {
            return Ok(false);
        }
        if !matches!(name, "write" | "edit") {
            return Ok(false);
        }
        if self.task_state.changed_files.len() < 2 {
            return Ok(false);
        }
        let msg = crate::git::GitManager::suggest_commit_message(&self.task_state.changed_files);
        match self.git.create_checkpoint(&format!("onemini checkpoint: {msg}")) {
            Ok(hash) => {
                if verbose {
                    println!(
                        "{}",
                        ui::success(&format!("已创建 git 检查点: {}", &hash[..hash.len().min(8)]))
                    );
                }
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    fn check_permission(
        &mut self,
        tool: &Arc<dyn Tool>,
        detail: &str,
        args: &Value,
        verbose: bool,
    ) -> Result<bool> {
        let name = tool.name();
        if self.opts.auto_approve {
            return Ok(true);
        }

        match self.permissions.evaluate(name, detail) {
            PermissionDecision::Allow => return Ok(true),
            PermissionDecision::Deny(reason) => {
                if verbose {
                    println!("{}", ui::error(&reason));
                }
                return Ok(false);
            }
            PermissionDecision::Ask(_) => {}
        }

        if !tool.requires_approval(args) {
            return Ok(true);
        }

        let risk = assess_risk(name, args);
        if !risk.is_empty() && verbose {
            println!("{}", ui::warn(&risk));
        }

        if matches!(name, "write" | "edit") {
            if let Some(path) = args["path"].as_str() {
                if let Ok(diff) = self.git.diff_preview(&[path]) {
                    ui::print_diff_preview(&diff);
                }
                if let Ok(staged) = self.git.diff_staged_preview() {
                    if !staged.trim().is_empty() {
                        ui::print_diff_preview(&staged);
                    }
                }
            }
        }

        if verbose {
            print!("{} 允许执行? [y/N/a=始终允许] ", ui::warn("权限"));
            io::stdout().flush()?;
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            let answer = line.trim().to_lowercase();
            if answer == "a" || answer == "always" {
                // 用户选择始终允许此工具 — 提示写入 permissions.toml
                println!(
                    "{}",
                    ui::dim(&format!(
                        "提示: 可在 {} 的 always_allow 中添加 \"{name}\"",
                        self.permissions.path().display()
                    ))
                );
                return Ok(true);
            }
            return Ok(answer == "y" || answer == "yes");
        }
        Ok(false)
    }

    fn track_tool_outcome(&mut self, name: &str, args: &Value, out: &str) {
        match name {
            "write" | "edit" => {
                if let Some(path) = args["path"].as_str() {
                    self.task_state.record_file_change(path);
                }
            }
            "bash" => {
                let command = args["command"].as_str().unwrap_or("");
                let (success, message) = parse_bash_result(out);
                self.task_state.record_bash_result(command, success, message);
                self.update_verification_from_bash(command, success);
            }
            _ => {}
        }
    }

    fn track_tool_error(&mut self, name: &str, args: &Value, err: &str) {
        self.task_state.last_errors.push(err.to_string());
        if name == "bash" {
            let command = args["command"].as_str().unwrap_or("");
            self.task_state
                .record_bash_result(command, false, Some(err.to_string()));
        }
        self.task_state.mark_current_failed();
    }

    fn update_verification_from_bash(&mut self, command: &str, success: bool) {
        for check in &mut self.task_state.verification_checks {
            if check
                .command
                .as_ref()
                .is_some_and(|c| command_contains(command, c))
            {
                check.passed = Some(success);
                check.message = if success {
                    "已执行并通过".into()
                } else {
                    "已执行但失败".into()
                };
            }
        }
    }
}

pub async fn run_agent(opts: &AgentOptions, prompt: &str, stream: bool) -> Result<String> {
    let mut session = AgentSession::new(opts.clone()).await?;
    session.run_turn(prompt, stream).await
}

fn parse_bash_result(out: &str) -> (bool, Option<String>) {
    if let Ok(v) = serde_json::from_str::<Value>(out) {
        let success = v["success"].as_bool().unwrap_or(true);
        let reason = v["failure_reason"]
            .as_str()
            .or_else(|| v["stderr_preview"].as_str())
            .map(str::to_string);
        return (success, reason);
    }
    let failed = out.contains("exit code:") && !out.contains("exit code: 0");
    (
        !failed,
        if failed {
            Some(truncate_bytes(out, 200))
        } else {
            None
        },
    )
}

fn command_contains(command: &str, expected: &str) -> bool {
    command.contains(expected) || expected.contains(command)
}

fn assess_risk(name: &str, args: &Value) -> String {
    match name {
        "bash" => {
            let cmd = args["command"].as_str().unwrap_or("").to_lowercase();
            if cmd.contains("rm -rf")
                || cmd.contains("git reset --hard")
                || cmd.contains("git push --force")
                || cmd.contains("drop table")
            {
                return "高风险命令：可能造成不可恢复的数据丢失。".into();
            }
            if cmd.contains("git clean") || cmd.contains("chmod -r") {
                return "中风险命令：可能影响工作区文件或权限。".into();
            }
            String::new()
        }
        "write" | "edit" => {
            if args["path"]
                .as_str()
                .is_some_and(|p| p.contains(".env") || p.contains("id_rsa"))
            {
                return "高风险写入：目标文件可能包含敏感信息。".into();
            }
            String::new()
        }
        _ => String::new(),
    }
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
            truncate_bytes(cmd, 60)
        }
        "delegate" => truncate_bytes(args["task"].as_str().unwrap_or("?"), 60),
        _ => args.to_string(),
    }
}

fn truncate_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

fn truncate_preview(s: &str, max: usize) -> String {
    truncate_bytes(s, max)
}
