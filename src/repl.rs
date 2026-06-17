use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use crate::agent::AgentOptions;
use crate::agent::AgentSession;
use crate::config::{Config, ConfigureOptions};
use crate::slash::SlashRegistry;
use crate::ui::{self, ReplHelper};

pub struct Repl {
    editor: Editor<ReplHelper, DefaultHistory>,
    session: AgentSession,
    slash: SlashRegistry,
}

impl Repl {
    pub async fn new(opts: AgentOptions) -> Result<Self> {
        let mut editor = Editor::new()?;
        editor.set_helper(Some(ReplHelper::new()));
        let workdir = opts.config.workdir().to_path_buf();
        let slash = SlashRegistry::load(&workdir)?;
        let session = AgentSession::new(opts).await?;
        Ok(Self {
            editor,
            session,
            slash,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("{}", ui::banner());
        println!(
            "{}",
            ui::dim(&format!(
                "工作目录: {}",
                self.session.workdir().display()
            ))
        );
        println!(
            "{}",
            ui::dim(&format!(
                "权限模式: {} · 输入 /mode 切换",
                self.session.permission_mode().label()
            ))
        );
        println!("{}", ui::separator());
        println!();

        loop {
            let prompt = ui::input_prompt_plain();
            if let Some(helper) = self.editor.helper_mut() {
                helper.colored_prompt = ui::colored_input_prompt();
            }
            match self.editor.readline(prompt) {
                Ok(line) => {
                    let input = line.trim();
                    if input.is_empty() {
                        continue;
                    }
                    self.editor.add_history_entry(input)?;

                    if input.starts_with('/') {
                        if self.handle_slash_command(input).await? {
                            break;
                        }
                        continue;
                    }

                    println!("{} {}", ui::user_prefix(), input);
                    match self.session.run_turn(input, true).await {
                        Ok(_) => println!(),
                        Err(e) => println!("{}\n", ui::error(&e.to_string())),
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!();
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!();
                    break;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    async fn handle_slash_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts.first().copied() {
            Some("/exit") | Some("/quit") => return Ok(true),
            Some("/help") => {
                let help = format!(
                    "\n{}\n\
                      /help     显示帮助\n\
                      /plan     查看当前任务计划\n\
                      /status   查看步骤、验证、令牌用量\n\
                      /retry    重试最近失败步骤\n\
                      /compact  压缩历史消息\n\
                      /rollback 回滚到最近 git 检查点\n\
                      /clear    清空对话历史\n\
                      /config        显示当前配置\n\
                      /config setup  重新配置 API / 模型\n\
                      /mode     切换权限模式\n\
                      /permissions  查看权限规则摘要\n\
                      /exit     退出\n{}",
                    ui::section_title("可用命令"),
                    self.slash.format_help()
                );
                println!("{help}");
            }
            Some("/plan") => {
                println!(
                    "\n{}",
                    ui::render_plan_text(&self.session.task_state.format_plan())
                );
            }
            Some("/status") => {
                println!("\n{}", self.session.format_status());
            }
            Some("/retry") => match self.session.retry_last_failure(true).await {
                Ok(_) => println!(),
                Err(e) => println!("{}\n", ui::error(&e.to_string())),
            },
            Some("/compact") => match self.session.compact_history().await {
                Ok(()) => println!("{}", ui::success("历史消息已压缩")),
                Err(e) => println!("{}", ui::error(&e.to_string())),
            },
            Some("/mode") => {
                let next = self
                    .session
                    .permission_mode()
                    .cycle_next(self.session.disable_auto_mode());
                self.session.set_permission_mode(next);
                println!(
                    "{}",
                    ui::success(&format!("权限模式已切换为: {}", next.label()))
                );
            }
            Some("/permissions") => {
                println!("\n{}", self.session.permissions_summary());
            }
            Some("/rollback") => match self.session.rollback_git() {
                Ok(hash) => println!(
                    "{}",
                    ui::success(&format!(
                        "已回滚到检查点 {}",
                        &hash[..hash.len().min(8)]
                    ))
                ),
                Err(e) => println!("{}", ui::error(&e.to_string())),
            }
            Some("/clear") => {
                let workdir = self.session.workdir().to_path_buf();
                let opts = self.session.opts.clone();
                self.session = AgentSession::new(AgentOptions {
                    config: {
                        let mut c = opts.config.clone();
                        c.workdir = Some(workdir);
                        c
                    },
                    max_rounds: opts.max_rounds,
                    permission_mode: opts.permission_mode,
                    non_interactive_yes: opts.non_interactive_yes,
                    resume: false,
                    worktree_delegate: opts.worktree_delegate,
                })
                .await?;
                self.session.clear_persisted()?;
                println!("{}", ui::success("对话历史与任务状态已清空"));
            }
            Some("/config") => {
                if parts.get(1) == Some(&"setup") {
                    let force = parts.get(2) == Some(&"--force");
                    match Config::configure_interactive(ConfigureOptions::setup(force)) {
                        Ok(path) => {
                            println!(
                                "{}",
                                ui::success(&format!("配置已保存: {}", path.display()))
                            );
                            match self.session.reload_config() {
                                Ok(()) => {
                                    println!("{}", ui::success("当前会话已应用新配置"));
                                    println!();
                                    println!("{}", self.session.opts.config.display());
                                }
                                Err(e) => println!("{}", ui::error(&e.to_string())),
                            }
                        }
                        Err(e) => println!("{}", ui::error(&e.to_string())),
                    }
                } else {
                    println!("{}", self.session.opts.config.display());
                }
            }
            Some(cmd) => {
                let name = cmd.trim_start_matches('/');
                if let Some(custom) = self.slash.resolve(name) {
                    let mut prompt = custom.prompt.clone();
                    if parts.len() > 1 {
                        prompt.push(' ');
                        prompt.push_str(&parts[1..].join(" "));
                    }
                    match self.session.run_turn(&prompt, true).await {
                        Ok(_) => println!(),
                        Err(e) => println!("{}\n", ui::error(&e.to_string())),
                    }
                } else {
                    println!(
                        "{}",
                        ui::warn(&format!("未知命令: {cmd}，输入 /help 查看帮助"))
                    );
                }
            }
            None => {}
        }
        Ok(false)
    }
}
