use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use std::str::FromStr;

use dialoguer::{theme::ColorfulTheme, Input};

use crate::agent::AgentOptions;
use crate::agent::AgentSession;
use crate::config::{Config, ConfigureOptions};
use crate::permissions::PermissionMode;
use crate::skills::SkillRegistry;
use crate::slash::SlashRegistry;
use crate::ui::{self, ReplHelper};

pub struct Repl {
    editor: Editor<ReplHelper, DefaultHistory>,
    session: AgentSession,
    slash: SlashRegistry,
    skills: SkillRegistry,
}

impl Repl {
    pub async fn new(opts: AgentOptions) -> Result<Self> {
        let mut editor = Editor::new()?;
        editor.set_helper(Some(ReplHelper::new()));
        let workdir = opts.config.workdir().to_path_buf();
        let slash = SlashRegistry::load(&workdir)?;
        let skills = SkillRegistry::discover(&workdir)?;
        let session = AgentSession::new(opts).await?;
        Ok(Self {
            editor,
            session,
            slash,
            skills,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        ui::play_startup_banner().await;
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
                "权限模式: {} · 模型: {} · 输入 /help 查看命令",
                self.session.permission_mode().label(),
                self.session.opts.config.model_name(),
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
                      /model    选择模型（列表）\n\
                      /reasoning  选择是否显示思考过程\n\
                      /theme    从列表选择 UI 主题\n\
                      /mode     选择权限模式（列表）\n\
                      /permissions  查看权限规则摘要\n\
                      /skills   从列表选择并激活技能\n\
                      /skills list  列出可用 Agent Skills\n\
                      /exit     退出\n{}{}",
                    ui::section_title("可用命令"),
                    self.format_skills_help(),
                    self.slash.format_help()
                );
                println!("{help}");
            }
            Some("/skills") => {
                if parts.get(1) == Some(&"list") {
                    println!("{}", self.skills.format_cli_list());
                } else {
                    self.select_and_run_skill(parts.get(1).copied()).await?;
                }
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
                self.select_permission_mode(parts.get(1))?;
            }
            Some("/model") => {
                self.select_model(parts.get(1))?;
            }
            Some("/reasoning") => {
                self.select_reasoning(parts.get(1))?;
            }
            Some("/theme") => {
                self.select_theme(parts.get(1))?;
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
                let user_args = parts.get(1..).unwrap_or(&[]).join(" ");
                if let Some(prompt) = self.skills.activation_prompt(name, &user_args) {
                    match self.session.run_turn(&prompt, true).await {
                        Ok(_) => println!(),
                        Err(e) => println!("{}\n", ui::error(&e.to_string())),
                    }
                } else if let Some(custom) = self.slash.resolve(name) {
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

    fn select_permission_mode(&mut self, arg: Option<&&str>) -> Result<()> {
        let current = self.session.permission_mode();
        let disable_auto = self.session.disable_auto_mode();
        let choices = PermissionMode::repl_choices(disable_auto);

        let selected = if let Some(name) = arg {
            match PermissionMode::from_str(name) {
                Ok(mode) if choices.contains(&mode) => mode,
                Ok(mode) => {
                    let available = choices
                        .iter()
                        .map(|m| m.label())
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!(
                        "{}",
                        ui::warn(&format!(
                            "模式 {} 在当前环境不可用，可选: {available}",
                            mode.label()
                        ))
                    );
                    return Ok(());
                }
                Err(e) => {
                    println!("{}", ui::error(&e));
                    return Ok(());
                }
            }
        } else {
            let labels: Vec<String> = choices.iter().map(|m| m.select_label()).collect();
            let default = choices
                .iter()
                .position(|&m| m == current)
                .unwrap_or(0);
            match ui::select_index("选择权限模式", &labels, default) {
                Ok(idx) => choices[idx],
                Err(_) => {
                    println!("{}", ui::dim("已取消"));
                    return Ok(());
                }
            }
        };

        if selected == current {
            println!(
                "{}",
                ui::dim(&format!("当前权限模式: {}", current.label()))
            );
        } else {
            self.session.set_permission_mode(selected);
            println!(
                "{}",
                ui::success(&format!("权限模式已切换为: {}", selected.label()))
            );
        }
        Ok(())
    }

    fn select_model(&mut self, arg: Option<&&str>) -> Result<()> {
        let current = self.session.opts.config.model_name().to_string();
        match self.session.opts.config.pick_model(arg.copied()) {
            Ok(selected) => {
                if selected == current {
                    println!(
                        "{}",
                        ui::dim(&format!("当前模型: {current}"))
                    );
                } else {
                    match self.session.apply_model(&selected) {
                        Ok(()) => println!(
                            "{}",
                            ui::success(&format!("模型已切换为: {selected}"))
                        ),
                        Err(e) => println!("{}", ui::error(&e.to_string())),
                    }
                }
            }
            Err(_) => println!("{}", ui::dim("已取消")),
        }
        Ok(())
    }

    fn select_reasoning(&mut self, arg: Option<&&str>) -> Result<()> {
        let current = self.session.show_reasoning();
        match self.session.opts.config.pick_show_reasoning(arg.copied()) {
            Ok(selected) => {
                if selected == current {
                    println!(
                        "{}",
                        ui::dim(&format!(
                            "思考过程显示: {}",
                            if current { "开启" } else { "关闭" }
                        ))
                    );
                } else {
                    match self.session.set_show_reasoning(selected) {
                        Ok(()) => println!(
                            "{}",
                            ui::success(&format!(
                                "思考过程显示已{}",
                                if selected { "开启" } else { "关闭" }
                            ))
                        ),
                        Err(e) => println!("{}", ui::error(&e.to_string())),
                    }
                }
            }
            Err(_) => println!("{}", ui::dim("已取消")),
        }
        Ok(())
    }

    fn select_theme(&mut self, arg: Option<&&str>) -> Result<()> {
        let current = ui::ThemeId::parse(
            self.session
                .opts
                .config
                .ui
                .theme
                .as_deref()
                .unwrap_or("modern"),
        )
        .unwrap_or(ui::ThemeId::Modern);

        match self.session.opts.config.pick_theme(arg.copied()) {
            Ok(selected) => {
                if selected == current {
                    println!(
                        "{}",
                        ui::dim(&format!("当前 UI 主题: {}", selected.label()))
                    );
                } else {
                    ui::set_theme(selected);
                    self.session.opts.config.ui.theme = Some(selected.as_str().to_string());
                    match self.session.opts.config.save() {
                        Ok(path) => println!(
                            "{}",
                            ui::success(&format!(
                                "UI 主题已切换为 {}（已保存: {}）",
                                selected.label(),
                                path.display()
                            ))
                        ),
                        Err(e) => println!("{}", ui::error(&e.to_string())),
                    }
                }
            }
            Err(_) => println!("{}", ui::dim("已取消")),
        }
        Ok(())
    }

    async fn select_and_run_skill(&mut self, name: Option<&str>) -> Result<()> {
        let skill_name = if let Some(n) = name {
            n.to_string()
        } else if self.skills.is_empty() {
            println!("{}", ui::warn("没有可用技能"));
            return Ok(());
        } else {
            let skills = self.skills.list();
            let labels: Vec<String> = skills
                .iter()
                .map(|s| format!("{}  —  {}", s.name, s.description))
                .collect();
            match ui::select_index("选择技能", &labels, 0) {
                Ok(idx) => skills[idx].name.clone(),
                Err(_) => {
                    println!("{}", ui::dim("已取消"));
                    return Ok(());
                }
            }
        };

        if self.skills.get(&skill_name).is_none() {
            println!(
                "{}",
                ui::warn(&format!("未找到技能: {skill_name}，输入 /skills list 查看"))
            );
            return Ok(());
        }

        let context: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("补充说明（可选，直接回车跳过）")
            .allow_empty(true)
            .interact_text()
            .unwrap_or_default();

        if let Some(prompt) = self.skills.activation_prompt(&skill_name, &context) {
            match self.session.run_turn(&prompt, true).await {
                Ok(_) => println!(),
                Err(e) => println!("{}\n", ui::error(&e.to_string())),
            }
        }
        Ok(())
    }

    fn format_skills_help(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        let rows: Vec<Vec<String>> = self
            .skills
            .list()
            .iter()
            .map(|skill| vec![format!("/{}", skill.name), skill.description.clone()])
            .collect();
        let table = crate::ui::render_table(&["命令", "说明"], &rows);
        format!(
            "\n{}\n{}\n",
            crate::ui::section_title("Agent Skills").trim(),
            table
        )
    }
}
