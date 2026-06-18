mod prompt;
mod task;

use anyhow::{Context, Result};
use futures::StreamExt;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

use crate::compress::{compress_messages, needs_compression};
use crate::config::Config;
use crate::git::GitManager;
use crate::hooks::{HookOutcome, HookRunner};
use crate::llm::{AssistantMessage, ChatMessage, OpenAiClient, StreamEvent, ToolCall, UsageInfo};
use crate::managed::ManagedSettings;
use crate::mcp::McpRegistry;
use crate::permissions::{PermissionDecision, PermissionManager, PermissionMode};
use crate::sandbox::{SandboxBackend, SandboxRunner};
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
    pub permission_mode: PermissionMode,
    pub non_interactive_yes: bool,
    pub resume: bool,
    pub worktree_delegate: bool,
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
    sandbox_runner: SandboxRunner,
    managed_settings: ManagedSettings,
    skills: crate::skills::SkillRegistry,
    had_tool_calls_this_turn: bool,
    last_turn_usage: UsageInfo,
    last_context_tokens: u32,
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
        let sandbox_runner = SandboxRunner::new(&opts.config.sandbox);
        let mut registry = ToolRegistry::new(workdir.clone(), sandbox_runner.clone());
        registry.register(Arc::new(crate::tools::DelegateTool::new(
            workdir.clone(),
            opts.worktree_delegate || opts.config.delegate_use_worktree(),
        )));

        let managed_settings = ManagedSettings::load()?;
        let skills = crate::skills::SkillRegistry::discover(&workdir)?;

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
            permissions: PermissionManager::load(&managed_settings)?,
            hooks: HookRunner::load(&managed_settings)?,
            git: GitManager::new(workdir),
            sandbox_runner,
            managed_settings,
            skills,
            had_tool_calls_this_turn: false,
            last_turn_usage: UsageInfo::default(),
            last_context_tokens: 0,
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

    pub fn permission_mode(&self) -> PermissionMode {
        self.opts.permission_mode
    }

    pub fn set_permission_mode(&mut self, mode: PermissionMode) {
        self.opts.permission_mode = mode;
    }

    pub fn apply_model(&mut self, model: &str) -> Result<()> {
        if model == self.opts.config.model_name() {
            return Ok(());
        }
        self.opts.config.model = Some(model.to_string());
        self.opts.config.save()?;
        self.client = OpenAiClient::new(&self.opts.config)?;
        Ok(())
    }

    pub fn set_show_reasoning(&mut self, enabled: bool) -> Result<()> {
        self.opts.config.show_reasoning = Some(enabled);
        self.opts.config.save()?;
        Ok(())
    }

    pub fn show_reasoning(&self) -> bool {
        self.opts.config.show_reasoning()
    }

    pub fn disable_auto_mode(&self) -> bool {
        self.managed_settings.disable_auto_mode
    }

    pub fn permissions_summary(&self) -> String {
        self.permissions
            .format_summary(self.opts.permission_mode, &self.managed_settings)
    }

    pub fn reload_config(&mut self) -> Result<()> {
        let mut cfg = Config::load()?;
        cfg.workdir = self.opts.config.workdir.clone();
        self.client = OpenAiClient::new(&cfg)?;
        self.opts.config = cfg;
        Ok(())
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
                ui::render_turn_usage(
                    &self.last_turn_usage,
                    self.opts.config.model_name(),
                    self.last_context_tokens,
                )
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
        self.last_context_tokens = 0;

        if detect_skills_query(user_input) {
            let reply = self.skills.format_cli_list();
            if stream {
                println!("{}", ui::render_markdown(&reply));
                println!();
            }
            self.messages.push(ChatMessage::user(user_input.to_string()));
            self.messages.push(ChatMessage {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            });
            self.persist()?;
            return Ok(ui::sanitize_final(&reply));
        }

        if needs_compression(&self.messages) {
            let (compressed, _) = compress_messages(&self.client, &self.messages).await?;
            self.messages = compressed;
            if stream {
                println!("{}", ui::dim("… 历史消息已自动压缩"));
            }
        }

        let (effective_input, auto_skill) = self.skills.prepare_turn_input(user_input);
        if let Some(name) = auto_skill {
            if stream {
                println!("{}", ui::dim(&format!("⟡ 自动启用技能: {name}")));
            }
        }

        if let Some(ctx) = self.task_state.turn_context_block() {
            self.messages.push(ChatMessage::user(format!(
                "{ctx}\n\n用户请求:\n{effective_input}"
            )));
        } else {
            self.messages.push(ChatMessage::user(effective_input));
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

            let tools = Some(
                self.registry
                    .definitions_for_mode(self.opts.permission_mode),
            );
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
                let content = ui::sanitize_final(
                    &assistant.content.unwrap_or_default(),
                );
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
                if stream && !summary.is_empty() {
                    println!("{}", ui::task_summary_block(&summary));
                } else if !stream && !summary.is_empty() {
                    println!("{}", ui::task_summary_block(&summary));
                    if !content.is_empty() {
                        println!(
                            "{} {}",
                            ui::assistant_prefix(),
                            ui::render_markdown(&content)
                        );
                    }
                }
                self.session_usage.finish_turn();
                if self.last_turn_usage.total() > 0 {
                    ui::print_turn_usage(
                        &self.last_turn_usage,
                        self.opts.config.model_name(),
                        self.last_context_tokens,
                    );
                }
                let _ = self.hooks.run_notification(&format!(
                    "回合完成 · {} 令牌",
                    self.last_turn_usage.total()
                ));
                self.persist()?;
                return Ok(content);
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
        if usage.total() == 0 {
            return;
        }
        self.last_turn_usage.accumulate(usage);
        self.last_context_tokens = usage.prompt_tokens;
        self.session_usage.accumulate(usage);
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
        let mut args: Value = serde_json::from_str(&call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));

        if let Err(e) = validate_tool_input(name, &args) {
            return Ok(format!("[工具输入无效] {e}"));
        }

        let tool = self
            .registry
            .get(name)
            .with_context(|| format!("未知工具: {name}"))?;

        let mut hook_force_ask = false;
        match self.hooks.run_pre_tool(name, &args) {
            Ok(HookOutcome::Deny(reason)) => {
                return Ok(format!("[hook 拒绝] {reason}"));
            }
            Ok(HookOutcome::Ask(reason)) => {
                hook_force_ask = true;
                if verbose && !reason.is_empty() {
                    println!("{}", ui::warn(&reason));
                }
            }
            Ok(HookOutcome::Modified(v)) => args = v,
            Ok(HookOutcome::Continue) => {}
            Err(e) => return Ok(format!("[hook 错误] {e}")),
        }

        let detail = summarize_args(name, &args);
        if verbose {
            println!("{}", ui::tool_call(name, &detail));
        }

        if self.maybe_git_checkpoint(name, &args, verbose)? {
            // checkpoint created
        }

        if !self.check_permission(&tool, name, &detail, &args, verbose, hook_force_ask)? {
            return Ok("[用户拒绝执行]".into());
        }

        match tool.execute(args.clone()).await {
            Ok(out) => {
                if name == "bash" && verbose {
                    if let Ok(v) = serde_json::from_str::<Value>(&out) {
                        if v["timed_out"].as_bool() == Some(true) {
                            let cmd = args["command"].as_str().unwrap_or("");
                            println!(
                                "{}",
                                ui::block_warning(ui::bash_timeout_hint(cmd))
                            );
                        }
                    }
                }
                if let Err(e) = self.hooks.run_post_tool(name, &args, &out) {
                    if verbose {
                        println!("{}", ui::warn(&format!("PostToolUse hook: {e}")));
                    }
                }
                self.track_tool_outcome(name, &args, &out);
                if verbose {
                    let preview = if name == "list_skills" {
                        format_list_skills_preview(&out).unwrap_or_else(|| truncate_preview(&out, 200))
                    } else {
                        truncate_preview(&out, 200)
                    };
                    if !preview.trim().is_empty() {
                        print!("{}", ui::tool_output_preview(&preview));
                    }
                }
                Ok(out)
            }
            Err(e) => {
                let err = format!("[工具错误] {e}");
                let _ = self.hooks.run_post_tool(name, &args, &err);
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
        match self.git.create_checkpoint(&format!("onemini 检查点: {msg}")) {
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
        tool_name: &str,
        detail: &str,
        args: &Value,
        verbose: bool,
        force_ask: bool,
    ) -> Result<bool> {
        let workdir = self.workdir().to_path_buf();
        let mode = self.opts.permission_mode;
        let bypass = mode == PermissionMode::Bypass;

        if bypass && self.managed_settings.disable_bypass_permissions {
            if verbose {
                println!(
                    "{}",
                    ui::error("托管策略已禁用 bypass 权限模式")
                );
            }
            return Ok(false);
        }

        let decision = self.permissions.evaluate(
            tool_name,
            detail,
            args,
            &workdir,
            mode,
            bypass,
        );

        match &decision {
            PermissionDecision::Deny(reason) => {
                if verbose {
                    println!("{}", ui::error(reason));
                }
                return Ok(false);
            }
            PermissionDecision::Allow if !force_ask => return Ok(true),
            PermissionDecision::Ask(reason) => {
                if !reason.is_empty() && verbose {
                    println!("{}", ui::warn(reason));
                }
            }
            PermissionDecision::Allow => {}
        }

        if tool_name == "bash"
            && self.sandbox_runner.is_enabled()
            && self.sandbox_runner.auto_allow_sandboxed_bash()
            && self.sandbox_runner.backend() != SandboxBackend::None
        {
            return Ok(true);
        }

        if !tool.requires_approval(args) {
            return Ok(true);
        }

        let risk = assess_risk(tool_name, args);
        if !risk.is_empty() && verbose {
            println!("{}", ui::warn(&risk));
        }

        if matches!(tool_name, "write" | "edit") {
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

        if !verbose {
            if self.opts.non_interactive_yes {
                return Ok(false);
            }
            return Ok(false);
        }

        match ui::select_permission(tool_name, detail) {
            Ok(ui::PermissionChoice::Allow) => Ok(true),
            Ok(ui::PermissionChoice::Deny) => Ok(false),
            Ok(ui::PermissionChoice::Always) => {
                let pattern = if tool_name == "bash" {
                    detail.to_string()
                } else {
                    args["path"].as_str().unwrap_or(detail).to_string()
                };
                if let Err(e) = self.permissions.add_allow_rule(tool_name, &pattern) {
                    println!("{}", ui::warn(&format!("无法保存权限规则: {e}")));
                } else {
                    println!(
                        "{}",
                        ui::success(&format!(
                            "已保存 allow 规则到 {}",
                            self.permissions.path().display()
                        ))
                    );
                }
                Ok(true)
            }
            Err(_) => Ok(false),
        }
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

/// 自然语言技能列表查询 — 短路 LLM，直接程序化渲染
fn detect_skills_query(input: &str) -> bool {
    let t = input.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    const KEYWORDS: &[&str] = &[
        "有哪些技能",
        "有什么技能",
        "有哪些 skill",
        "有什么 skill",
        "技能列表",
        "skill list",
        "list skills",
        "列出技能",
        "可用技能",
        "你有哪些技能",
        "你有什么技能",
        "你会什么",
        "有哪些能力",
    ];
    KEYWORDS.iter().any(|k| t.contains(k))
        || t == "skills"
        || t == "/skills list"
}

fn format_list_skills_preview(json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json).ok()?;
    let skills = v.get("skills")?.as_array()?;
    let rows: Vec<Vec<String>> = skills
        .iter()
        .filter_map(|s| {
            Some(vec![
                format!("/{}", s.get("name")?.as_str()?),
                match s.get("source")?.as_str()? {
                    "builtin" => "内置".into(),
                    "user" => "用户".into(),
                    "project" => "项目".into(),
                    other => other.into(),
                },
                s.get("description")?.as_str()?.to_string(),
            ])
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    Some(crate::ui::render_table(&["命令", "来源", "说明"], &rows))
}

fn validate_tool_input(name: &str, args: &Value) -> Result<()> {
    match name {
        "read" | "write" | "edit" => {
            if args["path"].as_str().filter(|s| !s.is_empty()).is_none() {
                anyhow::bail!("缺少 path 参数");
            }
        }
        "bash" => {
            if args["command"].as_str().filter(|s| !s.is_empty()).is_none() {
                anyhow::bail!("缺少 command 参数");
            }
        }
        "grep" => {
            if args["pattern"].as_str().filter(|s| !s.is_empty()).is_none() {
                anyhow::bail!("缺少 pattern 参数");
            }
        }
        "glob" => {
            if args["pattern"].as_str().filter(|s| !s.is_empty()).is_none() {
                anyhow::bail!("缺少 pattern 参数");
            }
        }
        "delegate" => {
            if args["task"].as_str().filter(|s| !s.is_empty()).is_none() {
                anyhow::bail!("缺少 task 参数");
            }
        }
        _ => {}
    }
    Ok(())
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
    let failed = (out.contains("exit code:") || out.contains("退出码"))
        && !out.contains("exit code: 0")
        && !out.contains("退出码: 0")
        && !out.contains("退出码 0");
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
            "模式={}",
            args["pattern"].as_str().unwrap_or("?")
        ),
        "glob" => format!(
            "模式={}",
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
