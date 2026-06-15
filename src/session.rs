use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::TaskState;
use crate::llm::ChatMessage;

const SESSION_FILE: &str = "latest.json";
const MAX_MESSAGES: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub workdir: PathBuf,
    pub messages: Vec<ChatMessage>,
    pub task_state: TaskState,
    #[serde(default)]
    pub session_usage: crate::usage::SessionUsage,
    pub updated_at: String,
}

pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Result<Self> {
        let dir = crate::config::Config::config_dir()?;
        fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join(SESSION_FILE),
        })
    }

    pub fn load(&self) -> Result<Option<PersistedSession>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&self.path)
            .with_context(|| format!("读取会话失败: {}", self.path.display()))?;
        let session: PersistedSession = serde_json::from_str(&text).context("解析会话失败")?;
        Ok(Some(session))
    }

    pub fn save(
        &self,
        workdir: &Path,
        messages: &[ChatMessage],
        task_state: &TaskState,
        session_usage: &crate::usage::SessionUsage,
    ) -> Result<()> {
        let trimmed = trim_messages(messages);
        let session = PersistedSession {
            workdir: workdir.to_path_buf(),
            messages: trimmed,
            task_state: task_state.clone(),
            session_usage: session_usage.clone(),
            updated_at: chrono_lite_now(),
        };
        let text = serde_json::to_string_pretty(&session).context("序列化会话失败")?;
        fs::write(&self.path, text)
            .with_context(|| format!("写入会话失败: {}", self.path.display()))?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

fn trim_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    if messages.len() <= MAX_MESSAGES {
        return messages.to_vec();
    }
    let system = messages
        .first()
        .filter(|m| m.role == "system")
        .cloned();
    let start = messages.len() - (MAX_MESSAGES - 1);
    let mut out = Vec::with_capacity(MAX_MESSAGES);
    if let Some(sys) = system {
        out.push(sys);
        out.extend_from_slice(&messages[start..]);
    } else {
        out.extend_from_slice(&messages[start..]);
    }
    out
}

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}
