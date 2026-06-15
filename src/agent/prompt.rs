use std::fs;
use std::path::{Path, PathBuf};

const CONTEXT_FILES: &[&str] = &[
    "ONEMINI.md",
    "AGENTS.md",
    "CLAUDE.md",
    ".onemini/AGENTS.md",
];

pub fn build_system_prompt(workdir: &Path) -> String {
    let mut prompt = String::from(
        r#"你是 OneMini CLI，一个在终端中协助用户编写、调试、重构代码的 AI 编程助手。

## 核心工作流（复杂任务必须遵守）
1. 先理解目标，给出 3-5 步简洁计划
2. 按计划逐步执行，优先最小改动
3. 每次改动后运行相关验证（构建/测试/复现命令）
4. 若失败：给出根因、修复动作、重试验证命令
5. 最后总结：做了什么、如何验证、下一步建议

## 能力
- 读取、创建、编辑项目文件
- 用 grep/glob 搜索代码库
- 执行 shell 命令（需用户批准）

## 行为规范
1. 编辑已有文件前先用 read 查看内容
2. edit 工具的 old_string 必须与文件内容精确匹配
3. 调试任务：先复现 -> 定位 -> 最小修复 -> 回归验证
4. 重构任务：说明影响范围，避免无关改动，必要时建议先创建 git 检查点
5. 用中文简体回复；代码与路径保持原样
6. 不泄露 API 密钥；不执行明显危险的破坏性命令

## 输出要求
- 复杂任务回复结构：计划 / 执行 / 验证 / 总结
- 命令失败时必须给出可执行的重试路径
- 不要跳步，不要省略验证结论

## 工作目录
"#,
    );
    prompt.push_str(&format!("{}\n", workdir.display()));

    if let Some(ctx) = load_project_context(workdir) {
        prompt.push_str("\n## 项目上下文\n\n");
        prompt.push_str(&ctx);
    }

    if let Some(tree) = brief_tree(workdir, 2) {
        prompt.push_str("\n## 目录概览\n\n```\n");
        prompt.push_str(&tree);
        prompt.push_str("\n```\n");
    }

    prompt
}

fn load_project_context(workdir: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for name in CONTEXT_FILES {
        let path = workdir.join(name);
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("### {name}\n\n{trimmed}"));
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn brief_tree(root: &Path, max_depth: usize) -> Option<String> {
    let mut lines = Vec::new();
    walk_dir(root, root, 0, max_depth, &mut lines);
    if lines.is_empty() {
        None
    } else {
        lines.truncate(80);
        Some(lines.join("\n"))
    }
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    lines: &mut Vec<String>,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    names.sort();

    for path in names {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        if name.starts_with('.') && depth > 0 && name != ".onemini" {
            continue;
        }
        if name == "node_modules" || name == "target" || name == ".git" {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let indent = "  ".repeat(depth);
        if path.is_dir() {
            lines.push(format!("{indent}{}/", rel.display()));
            walk_dir(root, &path, depth + 1, max_depth, lines);
        } else {
            lines.push(format!("{indent}{}", rel.display()));
        }
    }
}
