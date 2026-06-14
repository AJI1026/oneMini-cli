use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::agent::{run_agent, AgentOptions};
use crate::config::{Config, ConfigPatch};
use crate::repl::Repl;
use crate::ui;

#[derive(Parser, Debug)]
#[command(
    name = "onemini",
    version,
    about = "OneMini CLI — 终端 AI 编程助手（类似 Claude Code）",
    long_about = "在终端中与 AI 协作编写代码：读取/编辑文件、搜索代码库、执行命令。"
)]
pub struct Cli {
    /// 一次性执行任务后退出（非交互模式）
    #[arg(short = 'p', long = "print")]
    pub prompt: Option<String>,

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

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 交互式对话（默认）
    Chat,
    /// 配置管理
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// 初始化配置（交互式，等同于 config setup）
    Init {
        /// 强制覆盖已有配置
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 显示当前配置
    Show,
    /// 交互式配置 API Key、Base URL、模型
    Setup {
        /// 跳过确认，直接覆盖已有配置
        #[arg(long)]
        force: bool,
    },
    /// 通过命令行设置配置项并保存
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
    fn run_config_command(action: Option<ConfigAction>) -> Result<()> {
        match action {
            None | Some(ConfigAction::Show) => {
                let config = Config::load()?;
                println!("{}", config.display());
            }
            Some(ConfigAction::Setup { force }) => {
                let path = Config::configure_interactive(force)?;
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

    pub async fn run(self) -> Result<()> {
        let mut config = Config::load()?;
        config.merge_cli(&self);

        let workdir = self
            .directory
            .clone()
            .or(config.workdir.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        config.workdir = Some(workdir);

        match self.command {
            Some(Commands::Config { action }) => {
                return Self::run_config_command(action);
            }
            Some(Commands::Init { force }) => {
                let path = Config::init_file(force)?;
                println!("{}", ui::success(&format!("配置已保存: {}", path.display())));
                return Ok(());
            }
            _ => {}
        }

        if config.api_key.as_deref().unwrap_or("").is_empty() {
            bail!(
                "未配置 API Key。{}\n\
                 也可设置环境变量 ONEMINI_API_KEY，或使用 {} 临时指定",
                Config::setup_hint(),
                "--api-key <KEY>"
            );
        }

        let opts = AgentOptions {
            config,
            max_rounds: self.max_rounds,
            auto_approve: self.dangerously_skip_permissions,
        };

        if let Some(prompt) = self.prompt {
            let reply = run_agent(&opts, &prompt).await?;
            println!("{}", crate::ui::render_markdown(&reply));
            return Ok(());
        }

        let mut repl = Repl::new(opts)?;
        repl.run().await.context("REPL 退出异常")
    }
}
