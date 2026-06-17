use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Cli;

/// 交互式配置选项
#[derive(Debug, Clone, Copy)]
pub struct ConfigureOptions {
    /// 跳过确认，直接覆盖已有配置
    pub force: bool,
    /// 首次启动且无 API Key（显示欢迎语、跳过覆盖确认）
    pub first_run: bool,
}

impl ConfigureOptions {
    pub fn setup(force: bool) -> Self {
        Self {
            force,
            first_run: false,
        }
    }

    pub fn first_run() -> Self {
        Self {
            force: false,
            first_run: true,
        }
    }
}

struct ModelOption {
    id: &'static str,
    desc: &'static str,
}

struct ProviderPreset {
    name: &'static str,
    base_url: &'static str,
    models: &'static [ModelOption],
}

const PROVIDER_PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "DeepSeek",
        base_url: "https://api.deepseek.com",
        models: &[
            ModelOption {
                id: "deepseek-chat",
                desc: "通用对话（推荐）",
            },
            ModelOption {
                id: "deepseek-reasoner",
                desc: "推理增强",
            },
        ],
    },
    ProviderPreset {
        name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        models: &[
            ModelOption {
                id: "gpt-4o",
                desc: "GPT-4o",
            },
            ModelOption {
                id: "gpt-4o-mini",
                desc: "GPT-4o Mini（更快更省）",
            },
        ],
    },
    ProviderPreset {
        name: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        models: &[
            ModelOption {
                id: "anthropic/claude-sonnet-4",
                desc: "Claude Sonnet 4",
            },
            ModelOption {
                id: "openai/gpt-4o",
                desc: "GPT-4o",
            },
        ],
    },
    ProviderPreset {
        name: "自定义（OpenAI 兼容 API）",
        base_url: "",
        models: &[],
    },
];

/// 可通过 `config set` 更新的配置字段
#[derive(Debug, Clone, Default)]
pub struct ConfigPatch {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub show_reasoning: Option<bool>,
    #[serde(default)]
    pub auto_git_checkpoint: Option<bool>,
    #[serde(default)]
    pub mcp_servers: Vec<crate::mcp::McpServerConfig>,
    #[serde(skip)]
    pub workdir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: Some("https://api.deepseek.com".into()),
            model: Some("deepseek-chat".into()),
            temperature: Some(0.2),
            max_tokens: Some(8192),
            show_reasoning: Some(true),
            auto_git_checkpoint: Some(true),
            mcp_servers: Vec::new(),
            workdir: None,
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("无法定位配置目录")?
            .join("onemini");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    fn resolve_onemini_env() -> String {
        let raw = std::env::var("ONEMINI_ENV").unwrap_or_else(|_| "development".to_string());
        let lower = raw.trim().to_lowercase();
        match lower.as_str() {
            "development" | "staging" | "production" => lower,
            _ => "development".to_string(),
        }
    }

    fn load_env_files() {
        let env = Self::resolve_onemini_env();
        let names = [format!(".env.{env}"), format!(".env.{env}.local")];

        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd);
        }
        if let Ok(user_dir) = Self::config_dir() {
            dirs.push(user_dir);
        }

        for dir in dirs {
            for (index, name) in names.iter().enumerate() {
                let path = dir.join(name);
                if !path.is_file() {
                    continue;
                }
                let result = if index == 0 {
                    dotenvy::from_filename(&path)
                } else {
                    dotenvy::from_filename_override(&path)
                };
                if let Err(err) = result {
                    eprintln!("警告: 加载 {} 失败: {err}", path.display());
                }
            }
        }
    }

    pub fn load() -> Result<Self> {
        Self::load_env_files();
        let path = Self::config_path()?;
        if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("读取配置失败: {}", path.display()))?;
            let mut cfg: Config = toml::from_str(&text).map_err(|e| {
                anyhow::anyhow!(
                    "解析 config.toml 失败 ({}): {e}\n\
                        请使用 TOML 格式，例如:\n\
                        api_key = \"sk-...\"\n\
                        base_url = \"https://api.deepseek.com\"\n\
                        model = \"deepseek-chat\"",
                    path.display()
                )
            })?;
            if cfg.base_url.is_none() {
                cfg.base_url = Config::default().base_url;
            }
            if cfg.model.is_none() {
                cfg.model = Config::default().model;
            }
            Ok(cfg)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::config_path()?;
        let mut to_save = self.clone();
        to_save.workdir = None;
        let text = toml::to_string_pretty(&to_save).context("序列化配置失败")?;
        fs::write(&path, text)
            .with_context(|| format!("写入配置失败: {}", path.display()))?;
        Ok(path)
    }

    pub fn apply_patch(&mut self, patch: &ConfigPatch) {
        if let Some(ref k) = patch.api_key {
            self.api_key = Some(k.clone());
        }
        if let Some(ref u) = patch.base_url {
            self.base_url = Some(u.clone());
        }
        if let Some(ref m) = patch.model {
            self.model = Some(m.clone());
        }
        if let Some(t) = patch.temperature {
            self.temperature = Some(t);
        }
        if let Some(n) = patch.max_tokens {
            self.max_tokens = Some(n);
        }
    }

    /// 终端交互式配置：选择服务商 → 输入模型 / Base URL → 输入 API Key
    pub fn configure_interactive(opts: ConfigureOptions) -> Result<PathBuf> {
        let path = Self::config_path()?;
        let theme = ColorfulTheme::default();

        if opts.first_run {
            println!("{}", crate::ui::banner());
            println!(
                "{}",
                crate::ui::warn("首次使用 OneMini CLI，请完成以下配置（约 1 分钟）")
            );
            println!();
        } else if path.exists() && !opts.force {
            let overwrite = Confirm::with_theme(&theme)
                .with_prompt(format!(
                    "配置文件已存在 ({})\n是否重新配置？",
                    path.display()
                ))
                .default(false)
                .interact()?;
            if !overwrite {
                bail!("已取消配置");
            }
        }

        let mut cfg = if path.exists() {
            Self::load()?
        } else {
            Config::default()
        };

        // 1. 选择 API 服务商
        let preset_labels: Vec<String> = PROVIDER_PRESETS
            .iter()
            .map(|p| {
                if p.base_url.is_empty() {
                    p.name.to_string()
                } else {
                    format!("{}  ({})", p.name, p.base_url)
                }
            })
            .collect();
        let preset_idx = Select::with_theme(&theme)
            .with_prompt("选择 API 服务商")
            .items(&preset_labels)
            .default(0)
            .interact()?;
        let preset = &PROVIDER_PRESETS[preset_idx];

        // 2. Base URL
        let base_default = if preset.base_url.is_empty() {
            cfg.base_url.clone().unwrap_or_default()
        } else {
            preset.base_url.to_string()
        };
        let base_url: String = Input::with_theme(&theme)
            .with_prompt("API 接口地址")
            .default(base_default)
            .interact_text()?;
        if base_url.trim().is_empty() {
            bail!("API 接口地址不能为空");
        }
        cfg.base_url = Some(base_url.trim().to_string());

        // 3. 模型 ID
        cfg.model = Some(prompt_model(&theme, preset, cfg.model.as_deref())?);

        // 4. API Key
        println!();
        if opts.first_run {
            println!("{}", crate::ui::dim("请输入 API 密钥（输入时不显示）"));
        } else {
            println!(
                "{}",
                crate::ui::dim("按 Enter 保留当前密钥；首次配置或更换密钥时请重新输入")
            );
        }
        let current_key = cfg.api_key.as_deref().unwrap_or("");
        let key_hint = if current_key.is_empty() {
            "(未设置)".to_string()
        } else {
            "****（已设置）".to_string()
        };
        let api_key = Password::with_theme(&theme)
            .with_prompt(format!("API 密钥 [{key_hint}]"))
            .allow_empty_password(!opts.first_run)
            .interact()?;
        if !api_key.is_empty() {
            cfg.api_key = Some(api_key);
        }

        if cfg.api_key.as_deref().unwrap_or("").is_empty() {
            bail!("API 密钥不能为空");
        }

        // 5. 确认保存
        println!("{}", crate::ui::section_title("配置预览"));
        println!("{}", cfg.display_summary());
        let save = Confirm::with_theme(&theme)
            .with_prompt("保存以上配置？")
            .default(true)
            .interact()?;
        if !save {
            bail!("已取消配置");
        }

        let saved = cfg.save()?;
        Ok(saved)
    }

    pub fn init_file(force: bool) -> Result<PathBuf> {
        Self::configure_interactive(ConfigureOptions::setup(force))
    }

    pub fn merge_cli(&mut self, cli: &Cli) {
        if let Some(ref k) = cli.api_key {
            self.api_key = Some(k.clone());
        }
        if let Some(ref u) = cli.base_url {
            self.base_url = Some(u.clone());
        }
        if let Some(ref m) = cli.model {
            self.model = Some(m.clone());
        }
    }

    pub fn workdir(&self) -> &Path {
        self.workdir
            .as_deref()
            .unwrap_or_else(|| Path::new("."))
    }

    pub fn show_reasoning(&self) -> bool {
        self.show_reasoning.unwrap_or(true)
    }

    pub fn auto_git_checkpoint(&self) -> bool {
        self.auto_git_checkpoint.unwrap_or(true)
    }

    pub fn model_name(&self) -> &str {
        self.model.as_deref().unwrap_or("deepseek-chat")
    }

    pub fn display_summary(&self) -> String {
        format!(
            "{}\n{}\n{}",
            crate::ui::status_pair(
                "API 接口地址",
                self.base_url.as_deref().unwrap_or("(未设置)"),
            ),
            crate::ui::status_pair(
                "模型 ID",
                self.model.as_deref().unwrap_or("(未设置)"),
            ),
            crate::ui::status_pair("API 密钥", Self::mask_api_key(self.api_key.as_deref())),
        )
    }

    pub fn display(&self) -> String {
        format!(
            "{}\n{}\n{}",
            crate::ui::status_pair(
                "配置文件",
                &Self::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "?".into()),
            ),
            self.display_summary(),
            crate::ui::status_pair("工作目录", &self.workdir().display().to_string()),
        )
    }

    fn mask_api_key(key: Option<&str>) -> &'static str {
        match key {
            Some(k) if !k.is_empty() => "****",
            _ => "(未设置)",
        }
    }

    pub fn setup_hint() -> String {
        format!(
            "请运行 {} 或再次执行 {} 进入配置向导",
            crate::ui::hint("onemini config setup"),
            crate::ui::hint("onemini")
        )
    }
}

fn prompt_model(
    theme: &ColorfulTheme,
    preset: &ProviderPreset,
    current: Option<&str>,
) -> Result<String> {
    if preset.models.is_empty() {
        let default = current.unwrap_or("").to_string();
        let model: String = Input::with_theme(theme)
            .with_prompt("模型 ID / 名称")
            .default(default)
            .interact_text()?;
        if model.trim().is_empty() {
            bail!("模型 ID 不能为空");
        }
        return Ok(model.trim().to_string());
    }

    let mut items: Vec<String> = preset
        .models
        .iter()
        .map(|m| format!("{}  —  {}", m.id, m.desc))
        .collect();
    items.push("自定义输入...".to_string());

    let default_idx = current
        .and_then(|cur| preset.models.iter().position(|m| m.id == cur))
        .unwrap_or(0);

    let choice = Select::with_theme(theme)
        .with_prompt("选择模型（或自定义输入）")
        .items(&items)
        .default(default_idx)
        .interact()?;

    if choice < preset.models.len() {
        Ok(preset.models[choice].id.to_string())
    } else {
        let default = current.unwrap_or(preset.models[0].id).to_string();
        let model: String = Input::with_theme(theme)
            .with_prompt("模型 ID / 名称")
            .default(default)
            .interact_text()?;
        if model.trim().is_empty() {
            bail!("模型 ID 不能为空");
        }
        Ok(model.trim().to_string())
    }
}
