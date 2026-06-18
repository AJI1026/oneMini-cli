use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::Tool;
use crate::skills::{SkillRegistry, SkillSource};

pub struct ListSkillsTool {
    workdir: std::path::PathBuf,
}

impl ListSkillsTool {
    pub fn new(workdir: std::path::PathBuf) -> Self {
        Self { workdir }
    }
}

#[async_trait]
impl Tool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }

    fn description(&self) -> &str {
        "返回当前可用的 Agent Skills 结构化列表（名称、来源、描述）。用户询问技能/能力列表时使用，禁止手写 Markdown 表格。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        false
    }

    async fn execute(&self, _args: Value) -> Result<String> {
        let registry = SkillRegistry::discover(&self.workdir)?;
        let skills: Vec<Value> = registry
            .list()
            .iter()
            .map(|skill| {
                let source = match skill.source {
                    SkillSource::Builtin | SkillSource::Bundled => "builtin",
                    SkillSource::User => "user",
                    SkillSource::Project => "project",
                };
                json!({
                    "name": skill.name,
                    "description": skill.description,
                    "source": source,
                })
            })
            .collect();
        Ok(json!({
            "agent_name": "OneMini",
            "skills_count": skills.len(),
            "skills": skills,
        })
        .to_string())
    }
}
