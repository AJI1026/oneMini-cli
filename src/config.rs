use anyhow::{bail, Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Cli;

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

    pub fn load() -> Result<Self> {
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

    /// 终端交互式配置 API Key、Base URL、模型等
    pub fn configure_interactive(force: bool) -> Result<PathBuf> {
        let path = Self::config_path()?;
        if path.exists() && !force {
            let overwrite = Confirm::with_theme(&ColorfulTheme::default())
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

        println!("{}", crate::ui::dim("按 Enter 保留当前值，API Key 留空则不变"));
        println!();

        let current_key = cfg.api_key.as_deref().unwrap_or("");
        let key_hint = if current_key.is_empty() {
            "(未设置)".to_string()
        } else {
            "****（已设置）".to_string()
        };
        let api_key = Password::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("API Key [{key_hint}]"))
            .allow_empty_password(true)
            .interact()?;
        if !api_key.is_empty() {
            cfg.api_key = Some(api_key);
        }

        let base_url: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("API Base URL")
            .default(cfg.base_url.clone().unwrap_or_default())
            .interact_text()?;
        if !base_url.is_empty() {
            cfg.base_url = Some(base_url);
        }

        let model: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("模型名称")
            .default(cfg.model.clone().unwrap_or_default())
            .interact_text()?;
        if !model.is_empty() {
            cfg.model = Some(model);
        }

        if cfg.api_key.as_deref().unwrap_or("").is_empty() {
            bail!("API Key 不能为空，请重新运行 onemini config setup");
        }

        let saved = cfg.save()?;
        Ok(saved)
    }

    pub fn init_file(force: bool) -> Result<PathBuf> {
        Self::configure_interactive(force)
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

    pub fn display(&self) -> String {
        format!(
            "配置文件: {}\n\
             API Base: {}\n\
             模型: {}\n\
             API Key: {}\n\
             工作目录: {}",
            Self::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "?".into()),
            self.base_url.as_deref().unwrap_or("(未设置)"),
            self.model.as_deref().unwrap_or("(未设置)"),
            Self::mask_api_key(self.api_key.as_deref()),
            self.workdir().display(),
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
            "请运行 {} 在终端中配置 API Key 和 Base URL",
            "onemini config setup".cyan()
        )
    }
}
