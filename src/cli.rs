use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use std::io::{stdin, IsTerminal};
use std::path::PathBuf;
use std::str::FromStr;

use crate::agent::{run_agent, AgentOptions};
use crate::config::{Config, ConfigPatch, ConfigureOptions};
use crate::managed::ManagedSettings;
use crate::permissions::PermissionMode;
use crate::repl::Repl;
use crate::ui;

pub const AFTER_HELP: &str = "\
常用: onemini | onemini --resume | onemini -p \"任务\" | onemini config setup\n\
详细说明请使用: onemini --help";

pub const AFTER_LONG_HELP: &str = "\
常用用法:
  onemini                              进入交互会话（默认）
  onemini --resume                     恢复上次会话（含历史与任务状态）
  onemini -C /path/to/project          指定工作目录
  onemini -p \"运行 cargo test\"         一次性任务，执行后退出
  onemini \"修复登录接口报错\"             同上（位置参数 TASK）

子命令:
  onemini config setup                 交互式配置 API 密钥 / 接口地址 / 模型
  onemini config show                  查看当前配置
  onemini config set --api-key sk-...  命令行设置配置项
  onemini init                         初始化配置（等同 config setup）
  onemini install                      安装到 ~/.local/bin 并配置 PATH（Release 包可双击 exe/.app 自动安装）
  onemini uninstall                    卸载二进制并移除 PATH 配置
  onemini uninstall --purge            同时删除配置与缓存数据
  onemini update --check               检查是否有新版本
  onemini update                       更新到 GitHub Release 最新版
  onemini skills list                  列出内置与用户技能
  onemini skills catalog               Anthropic 官方技能目录
  onemini skills install pdf design    安装官方技能
  onemini skills show <name>           查看技能 SKILL.md 全文

交互模式会话命令（进入 onemini 后输入）:
  /help     显示帮助          /plan     查看当前任务计划
  /status   查看步骤与用量    /retry    重试最近失败步骤
  /model    选择模型          /reasoning  选择思考过程显示
  /mode     选择权限模式      /skills   从列表激活技能
  /compact  压缩历史消息      /clear    清空会话
  /config   查看配置          /exit     退出

环境变量（可覆盖 config.toml）:
  ONEMINI_API_KEY      API 密钥
  ONEMINI_BASE_URL     API 接口地址（OpenAI 兼容）
  ONEMINI_MODEL        模型名称";

#[derive(Parser, Debug)]
#[command(
    name = "onemini",
    version,
    about = "OneMini CLI — 终端 AI 编程助手",
    long_about = "在终端中与 AI 持续协作：读取/编辑文件、搜索代码库、执行命令、调试与重构。"
)]
pub struct Cli {
    /// 一次性执行任务后退出（与位置参数 TASK 二选一或同时使用）
    #[arg(short = 'p', long = "print")]
    pub prompt: Option<String>,

    /// 一次性任务描述（执行后退出），例如: onemini "修复登录接口报错"
    #[arg(value_name = "TASK", trailing_var_arg = true)]
    pub task: Vec<String>,

    /// 恢复上次交互会话（包含上下文与任务状态）
    #[arg(long)]
    pub resume: bool,

    /// 工作目录（默认当前目录）
    #[arg(short = 'C', long = "directory")]
    pub directory: Option<PathBuf>,

    /// 模型名称
    #[arg(short, long, env = "ONEMINI_MODEL")]
    pub model: Option<String>,

    /// API 接口地址（OpenAI 兼容）
    #[arg(long, env = "ONEMINI_BASE_URL")]
    pub base_url: Option<String>,

    /// API 密钥
    #[arg(long, env = "ONEMINI_API_KEY")]
    pub api_key: Option<String>,

    /// 最大工具调用轮次
    #[arg(long, default_value_t = 25)]
    pub max_rounds: u32,

    /// 自动批准所有工具操作（危险，仅用于 CI/脚本）
    #[arg(long, conflicts_with = "permission_mode")]
    pub dangerously_skip_permissions: bool,

    /// 权限模式：default | plan | accept-edits | auto | dont-ask
    #[arg(long, value_name = "MODE")]
    pub permission_mode: Option<String>,

    /// 非交互模式下仅允许已匹配 allow 规则的操作（不全局 bypass）
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// 子 Agent 委派使用 git worktree 隔离
    #[arg(long)]
    pub worktree_delegate: bool,

    /// 一次性任务输出 JSON（仅与 -p/--print 联用）
    #[arg(long)]
    pub output_json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 交互式会话（默认，直接运行 onemini 等效）
    #[command(after_long_help = "示例:\n  onemini\n  onemini -C /path/to/project")]
    Chat,
    /// 恢复上次交互会话（含上下文与任务状态）
    #[command(after_long_help = "示例:\n  onemini resume\n  onemini --resume")]
    Resume,
    /// 配置管理（API 密钥、接口地址、模型等）
    #[command(
        after_long_help = "\
示例:\n  \
  onemini config show\n  \
  onemini config setup\n  \
  onemini config set --api-key sk-... --base-url https://api.deepseek.com --model deepseek-chat"
    )]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// 初始化配置（交互式，等同于 config setup）
    #[command(after_long_help = "示例:\n  onemini init\n  onemini init --force")]
    Init {
        /// 强制覆盖已有配置
        #[arg(long)]
        force: bool,
    },
    /// 检查并更新到 GitHub Release 最新版
    #[command(
        after_long_help = "\
示例:\n  \
  onemini update --check\n  \
  onemini update\n  \
  onemini update --version 0.1.1\n  \
  onemini update --force\n  \
  onemini update --ignore-deprecated   允许安装已弃用版本"
    )]
    Update {
        /// 仅检查是否有新版本，不下载安装
        #[arg(long)]
        check: bool,
        /// 安装指定版本，如 0.1.1 或 v0.1.1
        #[arg(long)]
        version: Option<String>,
        /// 即使版本相同也强制重新安装
        #[arg(long)]
        force: bool,
        /// 允许下载 versions.json 中标记为已弃用的版本（默认拒绝）
        #[arg(long)]
        ignore_deprecated: bool,
    },
    /// 将当前可执行文件安装到用户 bin 目录并配置 PATH
    #[command(after_long_help = "\
示例:\n  \
  onemini install\n  \
  onemini install --dir C:\\Users\\you\\.local\\bin\n  \
  onemini install --skip-path\n\n\
Release 离线包：Windows 双击 onemini.exe；macOS 双击 OneMini.app，首次启动自动安装并验签")]
    Install {
        /// 自定义安装目录（默认 %USERPROFILE%\\.local\\bin 或 ~/.local/bin）
        #[arg(long)]
        dir: Option<PathBuf>,
        /// 跳过自动 PATH 配置
        #[arg(long)]
        skip_path: bool,
    },
    /// 卸载已安装的 onemini 二进制并移除 PATH 配置
    #[command(after_long_help = "\
示例:\n  \
  onemini uninstall\n  \
  onemini uninstall --purge\n  \
  onemini uninstall --dir C:\\Users\\you\\.local\\bin\n  \
  onemini uninstall --keep-path --yes")]
    Uninstall {
        /// 自定义安装目录（默认与 install 相同）
        #[arg(long)]
        dir: Option<PathBuf>,
        /// 保留 shell / 用户 PATH 中的 onemini 条目
        #[arg(long)]
        keep_path: bool,
        /// 同时删除配置目录与缓存技能数据
        #[arg(long)]
        purge: bool,
        /// 非交互确认（脚本/CI 使用）
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Agent Skills 管理（兼容 anthropics/skills 格式）
    #[command(after_long_help = "\
示例:\n  \
  onemini skills list\n  \
  onemini skills show debug")]
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum SkillsAction {
    /// 列出当前已安装/内置技能
    List,
    /// 查看 Anthropic 官方技能目录（可 install 的 id）
    Catalog,
    /// 从 anthropics/skills 安装技能到用户目录
    Install {
        /// 技能 id，或快捷组 `docs` / `design`
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// 查看技能 SKILL.md 全文
    Show {
        /// 技能名（SKILL.md frontmatter 中的 name）
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 显示当前配置
    #[command(after_long_help = "示例:\n  onemini config show\n  onemini config")]
    Show,
    /// 交互式配置 API 密钥、接口地址、模型
    #[command(after_long_help = "示例:\n  onemini config setup\n  onemini config setup --force")]
    Setup {
        /// 跳过确认，直接覆盖已有配置
        #[arg(long)]
        force: bool,
    },
    /// 通过命令行设置配置项并保存
    #[command(
        after_long_help = "\
示例:\n  \
  onemini config set --api-key sk-...\n  \
  onemini config set --base-url https://api.deepseek.com --model deepseek-chat"
    )]
    Set {
        /// API 密钥
        #[arg(long, env = "ONEMINI_API_KEY")]
        api_key: Option<String>,

        /// API 接口地址（OpenAI 兼容）
        #[arg(long, env = "ONEMINI_BASE_URL")]
        base_url: Option<String>,

        /// 模型名称
        #[arg(long, env = "ONEMINI_MODEL")]
        model: Option<String>,

        /// 采样温度
        #[arg(long)]
        temperature: Option<f32>,

        /// 最大输出 token 数
        #[arg(long)]
        max_tokens: Option<u32>,
    },
}

impl Cli {
    /// 构建带使用说明的 clap Command
    pub fn command_with_hints() -> clap::Command {
        Self::command()
            .after_help(AFTER_HELP)
            .after_long_help(AFTER_LONG_HELP)
    }

    fn run_skills_command(action: SkillsAction, workdir: &PathBuf) -> Result<()> {
        match action {
            SkillsAction::Catalog => {
                print!("{}", crate::skills::catalog::format_catalog_table());
                Ok(())
            }
            SkillsAction::Install { ids } => {
                let parsed = crate::skills::install::parse_install_args(&ids)?;
                let dest = crate::skills::install::user_skills_dir()?;
                let installed = crate::skills::install::install_skills(&parsed, &dest)?;
                println!(
                    "\n已安装 {} 个技能。重启 onemini 或输入 /skills 查看。",
                    installed.len()
                );
                Ok(())
            }
            SkillsAction::List => {
                let registry = crate::skills::SkillRegistry::discover(workdir)?;
                print!("{}", registry.format_cli_list());
                Ok(())
            }
            SkillsAction::Show { name } => {
                let registry = crate::skills::SkillRegistry::discover(workdir)?;
                print!("{}", registry.format_show(&name)?);
                Ok(())
            }
        }
    }

    fn run_config_command(action: Option<ConfigAction>) -> Result<()> {
        match action {
            None | Some(ConfigAction::Show) => {
                let config = Config::load()?;
                println!("{}", config.display());
            }
            Some(ConfigAction::Setup { force }) => {
                let path = Config::configure_interactive(ConfigureOptions::setup(force))?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                let config = Config::load()?;
                println!();
                println!("{}", config.display());
            }
            Some(ConfigAction::Set {
                api_key,
                base_url,
                model,
                temperature,
                max_tokens,
            }) => {
                let patch = ConfigPatch {
                    api_key,
                    base_url,
                    model,
                    temperature,
                    max_tokens,
                    show_reasoning: None,
                };
                if patch.api_key.is_none()
                    && patch.base_url.is_none()
                    && patch.model.is_none()
                    && patch.temperature.is_none()
                    && patch.max_tokens.is_none()
                    && patch.show_reasoning.is_none()
                {
                    bail!(
                        "请至少指定一个配置项，例如:\n\
                         onemini config set --api-key sk-... --base-url https://api.deepseek.com"
                    );
                }
                let mut config = Config::load()?;
                config.apply_patch(&patch)?;
                let path = config.save()?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                println!();
                println!("{}", config.display());
            }
        }
        Ok(())
    }

    fn one_shot_prompt(&self) -> Option<String> {
        if let Some(p) = &self.prompt {
            return Some(p.clone());
        }
        if self.task.is_empty() {
            None
        } else {
            Some(self.task.join(" "))
        }
    }

    fn should_trigger_installer_mode(&self) -> bool {
        if self.command.is_some() {
            return false;
        }
        if self.one_shot_prompt().is_some() {
            return false;
        }
        if self.resume {
            return false;
        }
        if self.directory.is_some()
            || self.model.is_some()
            || self.base_url.is_some()
            || self.api_key.is_some()
            || self.permission_mode.is_some()
            || self.dangerously_skip_permissions
            || self.yes
            || self.worktree_delegate
            || self.output_json
        {
            return false;
        }
        crate::install::should_auto_install()
    }

    pub async fn run(self) -> Result<()> {
        if self.should_trigger_installer_mode() {
            return crate::install::run(crate::install::InstallOptions {
                install_dir: None,
                skip_path: false,
            });
        }

        let mut config = Config::load()?;
        config.merge_cli(&self);

        let workdir = self
            .directory
            .clone()
            .or(config.workdir.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        config.workdir = Some(workdir.clone());
        ui::init_theme(config.ui.theme.as_deref());

        match self.command {
            Some(Commands::Config { action }) => {
                return Self::run_config_command(action);
            }
            Some(Commands::Init { force }) => {
                let path = Config::init_file(force)?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                return Ok(());
            }
            Some(Commands::Update {
                check,
                version,
                force,
                ignore_deprecated,
            }) => {
                return crate::update::run(crate::update::UpdateOptions {
                    check_only: check,
                    version,
                    force,
                    ignore_deprecated,
                })
                .await;
            }
            Some(Commands::Install { dir, skip_path }) => {
                return crate::install::run(crate::install::InstallOptions {
                    install_dir: dir,
                    skip_path,
                });
            }
            Some(Commands::Uninstall {
                dir,
                keep_path,
                purge,
                yes,
            }) => {
                return crate::install::run_uninstall(crate::install::UninstallOptions {
                    install_dir: dir,
                    keep_path,
                    purge,
                    yes,
                });
            }
            Some(Commands::Skills { action }) => {
                return Self::run_skills_command(action, &workdir);
            }
            _ => {}
        }

        if config.api_key.as_deref().unwrap_or("").is_empty() {
            if stdin().is_terminal() {
                let opts = if Config::config_path()?.exists() {
                    ConfigureOptions::setup(false)
                } else {
                    ConfigureOptions::first_run()
                };
                let path = Config::configure_interactive(opts)?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                println!();
                config = Config::load()?;
                config.merge_cli(&self);
                config.workdir = Some(workdir.clone());
            } else {
                bail!(
                    "未配置 API 密钥。{}\n\
                     也可设置环境变量 ONEMINI_API_KEY，或使用 {} 临时指定",
                    Config::setup_hint(),
                    "--api-key <密钥>"
                );
            }
        }

        if !matches!(
            self.command,
            Some(Commands::Update { .. })
                | Some(Commands::Install { .. })
                | Some(Commands::Uninstall { .. })
        ) {
            let _ = crate::skills::bootstrap::ensure_document_skills(stdin().is_terminal());
        }

        let resume = self.resume || matches!(self.command, Some(Commands::Resume));

        let managed = ManagedSettings::load()?;
        if self.dangerously_skip_permissions && managed.disable_bypass_permissions {
            bail!("托管策略已禁用 --dangerously-skip-permissions");
        }

        let permission_mode = if self.dangerously_skip_permissions {
            PermissionMode::Bypass
        } else if let Some(ref m) = self.permission_mode {
            PermissionMode::from_str(m).map_err(anyhow::Error::msg)?
        } else {
            PermissionMode::Default
        };

        if permission_mode == PermissionMode::Auto && managed.disable_auto_mode {
            bail!("托管策略已禁用 auto 权限模式");
        }

        let opts = AgentOptions {
            config,
            max_rounds: self.max_rounds,
            permission_mode,
            non_interactive_yes: self.yes,
            resume,
            worktree_delegate: self.worktree_delegate,
        };

        if let Some(prompt) = self.one_shot_prompt() {
            let stream = !self.output_json;
            let reply = run_agent(&opts, &prompt, stream).await?;
            if self.output_json {
                let json = serde_json::json!({
                    "response": reply,
                    "model": opts.config.model_name(),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else if !stream {
                println!("{}", crate::ui::render_markdown(&reply));
            }
            return Ok(());
        }

        let mut repl = Repl::new(opts).await?;
        if resume {
            println!("{}", ui::success("已恢复上次会话上下文"));
        }
        repl.run().await.context("REPL 退出异常")
    }
}
