use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub description: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlashCommandFile {
    #[serde(flatten)]
    pub commands: HashMap<String, SlashCommand>,
}

pub struct SlashRegistry {
    commands: HashMap<String, SlashCommand>,
}

impl SlashRegistry {
    pub fn load(workdir: &Path) -> Result<Self> {
        let mut commands = HashMap::new();
        let paths = [
            workdir.join(".onemini/commands.toml"),
            crate::config::Config::config_dir()?.join("commands.toml"),
        ];
        for path in paths {
            if path.exists() {
                if let Ok(text) = fs::read_to_string(&path) {
                    if let Ok(file) = toml::from_str::<SlashCommandFile>(&text) {
                        commands.extend(file.commands);
                    }
                }
            }
        }
        Ok(Self { commands })
    }

    pub fn resolve(&self, name: &str) -> Option<&SlashCommand> {
        self.commands.get(name)
    }

    pub fn list(&self) -> Vec<(&String, &SlashCommand)> {
        let mut items: Vec<_> = self.commands.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        items
    }

    pub fn format_help(&self) -> String {
        if self.commands.is_empty() {
            return String::new();
        }
        let mut out = String::from(&format!("\n{}\n", crate::ui::section_title("自定义命令")));
        for (name, cmd) in self.list() {
            out.push_str(&format!(
                "  {}  {}\n",
                crate::ui::hint(&format!("/{name}")),
                crate::ui::dim(&cmd.description)
            ));
        }
        out
    }
}
