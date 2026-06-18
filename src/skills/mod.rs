//! Agent Skills — 兼容 [Agent Skills](https://github.com/anthropics/skills) 的 SKILL.md 格式。
//!
//! 内置技能编译进二进制；用户/项目技能从磁盘按需发现。
//! 系统提示词仅注入技能索引；完整指令通过 `/skill-name` 或 agent `read` 加载。

mod builtin;
pub mod bootstrap;
pub mod catalog;
pub mod install;
mod r#match;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_INLINE_SKILL_CHARS: usize = 24_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSource {
    Builtin,
    /// 随 CLI 分发的 skills/ 目录（含 scripts/）
    Bundled,
    User,
    Project,
}

/// 文档四件套 id（随包分发，用户技能可覆盖）
pub const BUNDLED_DOC_SKILL_IDS: &[&str] = &["pdf", "docx", "pptx", "xlsx"];

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
    /// 磁盘路径（内置技能为 None）
    pub path: Option<PathBuf>,
    /// 内置技能正文；磁盘技能在激活时 read
    pub body: Option<String>,
}

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn discover(workdir: &Path) -> Result<Self> {
        let mut skills = HashMap::new();

        for def in builtin::BUILTIN_SKILLS {
            skills.insert(
                def.name.to_string(),
                Skill {
                    name: def.name.to_string(),
                    description: def.description.to_string(),
                    source: SkillSource::Builtin,
                    path: None,
                    body: Some(def.body.to_string()),
                },
            );
        }

        merge_bundled_document_skills(&mut skills)?;

        if let Ok(config_dir) = crate::config::Config::config_dir() {
            merge_dir_skills(&mut skills, &config_dir.join("skills"), SkillSource::User)?;
        }
        merge_dir_skills(
            &mut skills,
            &workdir.join(".onemini/skills"),
            SkillSource::Project,
        )?;
        merge_dir_skills(&mut skills, &workdir.join("skills"), SkillSource::Project)?;

        Ok(Self { skills })
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&Skill> {
        let mut items: Vec<_> = self.skills.values().collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        items
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// 注入 system prompt 的简短索引（不含全文，避免 token 膨胀）
    pub fn format_catalog(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "\n## 可用技能（Agent Skills）\n\n\
             技能遵循 [Agent Skills](https://github.com/anthropics/skills) 规范（`SKILL.md` + YAML frontmatter）。\n\
             匹配任务意图时会**自动启用**对应技能；也可手动输入 `/技能名 [补充说明]`。\n\
             用户询问技能列表时调用 list_skills 或建议 `/skills list`。\n\n",
        );
        for skill in self.list() {
            let tag = match skill.source {
                SkillSource::Builtin | SkillSource::Bundled => "内置",
                SkillSource::User => "用户",
                SkillSource::Project => "项目",
            };
            out.push_str(&format!(
                "- **{}** ({tag}) — {}\n",
                skill.name, skill.description
            ));
        }
        out.push_str("\n示例：直接描述任务即可；或 `/commit-message` · `/debug 登录 500`\n");
        out
    }

    /// 为普通用户输入准备消息：必要时自动注入技能指令
    pub fn prepare_turn_input(&self, user_input: &str) -> (String, Option<String>) {
        if let Some(skill) = self.auto_match(user_input) {
            if let Some(prompt) = self.build_activation(skill, user_input, "技能自动启用") {
                return (prompt, Some(skill.name.clone()));
            }
        }
        (user_input.to_string(), None)
    }

    fn build_activation(&self, skill: &Skill, user_input: &str, label: &str) -> Option<String> {
        let request_block = if user_input.trim().is_empty() {
            "请按技能默认流程执行。".to_string()
        } else {
            format!("用户请求：\n{}", user_input.trim())
        };

        if let Some(body) = skill.body.as_deref() {
            return Some(format!(
                "[{label}: {}]\n\n{body}\n\n---\n\n{request_block}",
                skill.name
            ));
        }

        if let Some(path) = &skill.path {
            if let Ok(text) = fs::read_to_string(path) {
                if text.len() <= MAX_INLINE_SKILL_CHARS {
                    let skill_dir = path.parent().unwrap_or(path);
                    let mut msg = format!(
                        "[{label}: {}]\n\n{text}\n\n---\n\n{request_block}",
                        skill.name
                    );
                    if skill_dir.join("scripts").is_dir() {
                        msg.push_str("\n\n（该技能含 scripts/，可按 SKILL.md 用 bash 调用。）");
                    }
                    return Some(msg);
                }
            }
            let skill_dir = path.parent().unwrap_or(path);
            let mut msg = format!(
                "[{label}: {}]\n\n\
                 请先使用 read 工具读取 `{}` 获取完整技能指令，严格遵循后再执行。\n\
                 技能目录: `{}`",
                skill.name,
                path.display(),
                skill_dir.display()
            );
            if skill_dir.join("scripts").is_dir() {
                msg.push_str(&format!(
                    "\n脚本目录: `{}/scripts/`",
                    skill_dir.display()
                ));
                let shared = bundled_skills_root()
                    .map(|r| r.join("shared/office"))
                    .filter(|p| p.is_dir());
                if let Some(shared) = shared {
                    msg.push_str(&format!(
                        "\n共享 Office 工具: `{}`（unpack/pack/validate/soffice）",
                        shared.display()
                    ));
                }
            }
            for extra in [
                "reference.md",
                "forms.md",
                "editing.md",
                "pptxgenjs.md",
                "REFERENCE.md",
                "FORMS.md",
            ] {
                let p = skill_dir.join(extra);
                if p.is_file() {
                    msg.push_str(&format!("\n辅助文档: `{}`", p.display()));
                }
            }
            msg.push_str(&format!("\n\n{request_block}"));
            return Some(msg);
        }
        None
    }

    /// 斜杠命令激活技能，生成送入 agent 的 user 消息
    pub fn activation_prompt(&self, name: &str, user_args: &str) -> Option<String> {
        let skill = self.get(name)?;
        self.build_activation(skill, user_args, "技能激活")
    }

    pub fn format_cli_list(&self) -> String {
        let rows: Vec<Vec<String>> = self
            .list()
            .iter()
            .map(|skill| {
                let tag = match skill.source {
                    SkillSource::Builtin | SkillSource::Bundled => "内置",
                    SkillSource::User => "用户",
                    SkillSource::Project => "项目",
                };
                vec![format!("/{}", skill.name), tag.to_string(), skill.description.clone()]
            })
            .collect();
        let table = crate::ui::render_table(&["命令", "来源", "说明"], &rows);
        format!(
            "可用技能\n\n{table}\n\n\
             用法: /技能名 [说明]  ·  onemini skills show <名>  ·  onemini skills catalog"
        )
    }

    pub fn format_show(&self, name: &str) -> Result<String> {
        let skill = self
            .get(name)
            .with_context(|| format!("未找到技能: {name}"))?;
        let raw = if let Some(body) = &skill.body {
            body.clone()
        } else if let Some(path) = &skill.path {
            fs::read_to_string(path)
                .with_context(|| format!("读取技能失败: {}", path.display()))?
        } else {
            return bail_skill(name);
        };
        let (meta, body) = parse_frontmatter(&raw)?;
        let desc = meta
            .get("description")
            .map(String::as_str)
            .unwrap_or(skill.description.as_str());
        let tag = match skill.source {
            SkillSource::Builtin | SkillSource::Bundled => "内置",
            SkillSource::User => "用户",
            SkillSource::Project => "项目",
        };
        let info = crate::ui::render_table(
            &["技能", "来源", "说明"],
            &[vec![skill.name.clone(), tag.to_string(), desc.to_string()]],
        );
        Ok(format!("{info}\n\n{}", crate::ui::render_markdown(&body)))
    }
}

fn bail_skill(name: &str) -> Result<String> {
    anyhow::bail!("技能 {name} 无可用内容")
}

/// 随包 skills/ 根目录：环境变量 > 可执行文件旁 > 用户数据目录 > 开发 crate 根
pub fn bundled_skills_root() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("ONEMINI_SKILLS_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() && bootstrap::document_skills_ready(&p) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("skills");
            if p.is_dir() && bootstrap::document_skills_ready(&p) {
                return Some(p);
            }
        }
    }
    let data = bootstrap::document_skills_data_dir();
    if data.is_dir() && bootstrap::document_skills_ready(&data) {
        return Some(data);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills");
    if manifest.is_dir() && bootstrap::document_skills_ready(&manifest) {
        return Some(manifest);
    }
    None
}

fn merge_bundled_document_skills(map: &mut HashMap<String, Skill>) -> Result<()> {
    let Some(root) = bundled_skills_root() else {
        return Ok(());
    };
    for id in BUNDLED_DOC_SKILL_IDS {
        let skill_md = root.join(id).join("SKILL.md");
        if !skill_md.is_file() {
            continue;
        }
        if let Ok(parsed) = parse_skill_file(&skill_md, SkillSource::Bundled) {
            map.insert(parsed.name.clone(), parsed);
        }
    }
    Ok(())
}

fn merge_dir_skills(
    map: &mut HashMap<String, Skill>,
    root: &Path,
    source: SkillSource,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let skill_md = if path.is_dir() {
            path.join("SKILL.md")
        } else if path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md") {
            path.clone()
        } else {
            continue;
        };
        if !skill_md.is_file() {
            continue;
        }
        if let Ok(parsed) = parse_skill_file(&skill_md, source) {
            map.insert(parsed.name.clone(), parsed);
        }
    }
    Ok(())
}

fn parse_skill_file(path: &Path, source: SkillSource) -> Result<Skill> {
    let text = fs::read_to_string(path)?;
    let (meta, _body) = parse_frontmatter(&text)?;
    let name = meta
        .get("name")
        .cloned()
        .or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
        .with_context(|| format!("SKILL.md 缺少 name: {}", path.display()))?;
    let description = meta
        .get("description")
        .cloned()
        .unwrap_or_else(|| "无描述".to_string());
    Ok(Skill {
        name: name.clone(),
        description,
        source,
        path: Some(path.to_path_buf()),
        body: None,
    })
}

/// 解析 YAML frontmatter（仅支持简单的 `key: value` 行）
fn parse_frontmatter(text: &str) -> Result<(HashMap<String, String>, String)> {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return Ok((HashMap::new(), text.to_string()));
    }
    let rest = trimmed.strip_prefix("---").unwrap_or(trimmed).trim_start();
    let Some((front, body)) = rest.split_once("\n---") else {
        return Ok((HashMap::new(), text.to_string()));
    };
    let mut meta = HashMap::new();
    for line in front.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            meta.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok((meta, body.trim_start().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_basic() {
        let text = "---\nname: demo\ndescription: A demo skill\n---\n\n# Hello\n";
        let (meta, body) = parse_frontmatter(text).unwrap();
        assert_eq!(meta.get("name").map(String::as_str), Some("demo"));
        assert!(body.contains("# Hello"));
    }

    #[test]
    fn builtin_skills_loaded() {
        let reg = SkillRegistry::discover(Path::new(".")).unwrap();
        assert!(reg.get("commit-message").is_some());
        assert!(reg.get("debug").is_some());
    }

    #[test]
    fn design_bundle_ids_match_catalog() {
        let ids = super::install::parse_install_args(&["design".to_string()]).unwrap();
        assert_eq!(ids.len(), catalog::DEFAULT_DESIGN_SKILL_IDS.len());
    }

    #[test]
    fn bundled_document_skills_loaded_when_present() {
        let reg = SkillRegistry::discover(Path::new(".")).unwrap();
        if super::bundled_skills_root()
            .map(|r| r.join("pdf/SKILL.md").is_file())
            .unwrap_or(false)
        {
            assert!(reg.get("pdf").is_some(), "pdf bundled skill");
            assert!(reg.get("docx").is_some(), "docx bundled skill");
            assert!(reg.get("pptx").is_some(), "pptx bundled skill");
            assert!(reg.get("xlsx").is_some(), "xlsx bundled skill");
        }
    }
}
