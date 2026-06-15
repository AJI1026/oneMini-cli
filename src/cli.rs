use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use std::io::{stdin, IsTerminal};
use std::path::PathBuf;

use crate::agent::{run_agent, AgentOptions};
use crate::config::{Config, ConfigPatch, ConfigureOptions};
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
  onemini config setup                 交互式配置 API Key / Base URL / 模型
  onemini config show                  查看当前配置
  onemini config set --api-key sk-...  命令行设置配置项
  onemini init                         初始化配置（等同 config setup）
  onemini update --check               检查是否有新版本
  onemini update                       更新到 GitHub Release 最新版

交互模式会话命令（进入 onemini 后输入）:
  /help     显示帮助          /plan     查看当前任务计划
  /status   查看步骤与用量    /retry    重试最近失败步骤
  /compact  压缩历史消息      /clear    清空会话
  /config   查看配置          /exit     退出

环境变量:
  ONEMINI_API_KEY      API Key
  ONEMINI_BASE_URL     API Base URL（OpenAI 兼容）
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

    /// API Base URL（OpenAI 兼容）
    #[arg(long, env = "ONEMINI_BASE_URL")]
    pub base_url: Option<String>,

    /// API Key
    #[arg(long, env = "ONEMINI_API_KEY")]
    pub api_key: Option<String>,

    /// 最大工具调用轮次
    #[arg(long, default_value_t = 25)]
    pub max_rounds: u32,

    /// 自动批准所有工具操作（危险，仅用于 CI/脚本）
    #[arg(long)]
    pub dangerously_skip_permissions: bool,

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
    /// 配置管理（API Key、Base URL、模型等）
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
        /// 允许下载 versions.json 中标记为 deprecated 的版本（默认拒绝）
        #[arg(long)]
        ignore_deprecated: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 显示当前配置
    #[command(after_long_help = "示例:\n  onemini config show\n  onemini config")]
    Show,
    /// 交互式配置 API Key、Base URL、模型
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
        /// API Key
        #[arg(long, env = "ONEMINI_API_KEY")]
        api_key: Option<String>,

        /// API Base URL（OpenAI 兼容）
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
                };
                if patch.api_key.is_none()
                    && patch.base_url.is_none()
                    && patch.model.is_none()
                    && patch.temperature.is_none()
                    && patch.max_tokens.is_none()
                {
                    bail!(
                        "请至少指定一个配置项，例如:\n\
                         onemini config set --api-key sk-... --base-url https://api.deepseek.com"
                    );
                }
                let mut config = Config::load()?;
                config.apply_patch(&patch);
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

    pub async fn run(self) -> Result<()> {
        let mut config = Config::load()?;
        config.merge_cli(&self);

        let workdir = self
            .directory
            .clone()
            .or(config.workdir.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        config.workdir = Some(workdir.clone());

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
            _ => {}
        }

        if config.api_key.as_deref().unwrap_or("").is_empty() {
            if stdin().is_terminal() {
                let path = Config::configure_interactive(ConfigureOptions::first_run())?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                println!();
                config = Config::load()?;
                config.merge_cli(&self);
                config.workdir = Some(workdir.clone());
            } else {
                bail!(
                    "未配置 API Key。{}\n\
                     也可设置环境变量 ONEMINI_API_KEY，或使用 {} 临时指定",
                    Config::setup_hint(),
                    "--api-key <KEY>"
                );
            }
        }

        let resume = self.resume || matches!(self.command, Some(Commands::Resume));
        let opts = AgentOptions {
            config,
            max_rounds: self.max_rounds,
            auto_approve: self.dangerously_skip_permissions,
            resume,
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
