use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Done,
    Failed,
}

impl Default for StepStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub description: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub label: String,
    pub command: Option<String>,
    pub passed: Option<bool>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskState {
    pub goal: String,
    pub plan_steps: Vec<PlanStep>,
    pub current_step: usize,
    pub verification_checks: Vec<VerificationCheck>,
    pub last_errors: Vec<String>,
    pub changed_files: Vec<String>,
    pub last_bash_command: Option<String>,
    pub last_bash_failed: bool,
    pub checkpoint_suggested: bool,
}

impl TaskState {
    pub fn is_complex_task(input: &str) -> bool {
        let lower = input.to_lowercase();
        let keywords = [
            "实现", "修复", "重构", "调试", "优化", "添加", "删除", "迁移", "测试", "bug",
            "implement", "fix", "refactor", "debug", "optimize", "migrate", "test",
        ];
        input.chars().count() > 40 || keywords.iter().any(|k| lower.contains(k))
    }

    pub fn begin_turn(&mut self, user_input: &str, workdir: &Path) {
        self.goal = user_input.to_string();
        self.last_bash_failed = false;
        self.last_errors.clear();

        if Self::is_complex_task(user_input) {
            self.plan_steps = default_plan_steps();
            self.current_step = 0;
            if let Some(step) = self.plan_steps.get_mut(0) {
                step.status = StepStatus::InProgress;
            }
            self.verification_checks = detect_verification_checks(workdir);
        } else {
            self.plan_steps.clear();
            self.current_step = 0;
            self.verification_checks.clear();
        }
    }

    pub fn record_file_change(&mut self, path: &str) {
        if !self.changed_files.iter().any(|p| p == path) {
            self.changed_files.push(path.to_string());
        }
        if self.changed_files.len() >= 3 && !self.checkpoint_suggested {
            self.checkpoint_suggested = true;
        }
    }

    pub fn record_bash_result(&mut self, command: &str, success: bool, message: Option<String>) {
        self.last_bash_command = Some(command.to_string());
        self.last_bash_failed = !success;
        if !success {
            if let Some(msg) = message {
                self.last_errors.push(msg);
            } else {
                self.last_errors.push(format!("命令执行失败: {command}"));
            }
            self.mark_current_failed();
        }
    }

    pub fn advance_after_turn(&mut self, had_tool_calls: bool) {
        if self.plan_steps.is_empty() {
            return;
        }
        if had_tool_calls {
            if self.current_step < self.plan_steps.len().saturating_sub(1) {
                if let Some(step) = self.plan_steps.get_mut(self.current_step) {
                    step.status = StepStatus::Done;
                }
                self.current_step += 1;
                if let Some(step) = self.plan_steps.get_mut(self.current_step) {
                    step.status = StepStatus::InProgress;
                }
            }
        } else if let Some(step) = self.plan_steps.get_mut(self.current_step) {
            if step.status == StepStatus::InProgress {
                step.status = StepStatus::Done;
            }
        }
    }

    pub fn mark_current_failed(&mut self) {
        if let Some(step) = self.plan_steps.get_mut(self.current_step) {
            step.status = StepStatus::Failed;
        }
    }

    pub fn retry_prompt(&self) -> Option<String> {
        if !self.last_bash_failed {
            return None;
        }
        let cmd = self.last_bash_command.as_deref().unwrap_or("未知命令");
        let err = self.last_errors.last().cloned().unwrap_or_default();
        Some(format!(
            "上一步命令失败，请继续修复并重试。\n\
             失败命令: {cmd}\n\
             错误信息: {err}\n\
             要求: 先定位根因，做最小修复，然后重新运行验证命令。"
        ))
    }

    pub fn format_plan(&self) -> String {
        if self.plan_steps.is_empty() {
            return "当前没有结构化任务计划（简单问答模式）。".into();
        }
        let mut out = format!("任务目标: {}\n\n计划步骤:\n", self.goal);
        for (i, step) in self.plan_steps.iter().enumerate() {
            let marker = match step.status {
                StepStatus::Pending => "[ ]",
                StepStatus::InProgress => "[→]",
                StepStatus::Done => "[✓]",
                StepStatus::Failed => "[✗]",
            };
            let current = if i == self.current_step { " ← 当前" } else { "" };
            out.push_str(&format!("  {marker} {}. {}{}\n", i + 1, step.description, current));
        }
        out
    }

    pub fn format_status(&self) -> String {
        let mut out = self.format_plan();
        out.push_str("\n验证状态:\n");
        if self.verification_checks.is_empty() {
            out.push_str("  （无）\n");
        } else {
            for check in &self.verification_checks {
                let status = match check.passed {
                    Some(true) => "通过",
                    Some(false) => "失败",
                    None => "待执行",
                };
                out.push_str(&format!(
                    "  - [{}] {}: {}\n",
                    status, check.label, check.message
                ));
            }
        }
        if !self.changed_files.is_empty() {
            out.push_str("\n已修改文件:\n");
            for f in &self.changed_files {
                out.push_str(&format!("  - {f}\n"));
            }
        }
        if self.checkpoint_suggested {
            out.push_str(
                "\n安全提示: 改动较多，建议先创建检查点（例如 git commit 或 git stash）。\n",
            );
        }
        if !self.last_errors.is_empty() {
            out.push_str("\n最近错误:\n");
            for e in self.last_errors.iter().rev().take(3).rev() {
                out.push_str(&format!("  - {e}\n"));
            }
        }
        out
    }

    pub fn turn_context_block(&self) -> Option<String> {
        if self.plan_steps.is_empty() {
            return None;
        }
        Some(format!(
            "[任务流上下文]\n\
             目标: {}\n\
             当前步骤: {}/{}\n\
             请遵循: 先计划(3-5步) -> 执行 -> 验证 -> 总结。\n\
             若命令失败，给出可执行修复路径并重试验证。",
            self.goal,
            self.current_step + 1,
            self.plan_steps.len().max(1)
        ))
    }

    pub fn finish_summary(&self) -> String {
        if self.plan_steps.is_empty() {
            return String::new();
        }
        let mut summary = String::from("\n\n---\n任务流摘要\n");
        summary.push_str(&format!("目标: {}\n", self.goal));
        if !self.changed_files.is_empty() {
            summary.push_str(&format!("改动文件: {}\n", self.changed_files.join(", ")));
        }
        if !self.verification_checks.is_empty() {
            summary.push_str("建议验证:\n");
            for v in &self.verification_checks {
                if let Some(cmd) = &v.command {
                    summary.push_str(&format!("  - {}: `{cmd}`\n", v.label));
                }
            }
        }
        if self.last_bash_failed {
            summary.push_str("状态: 最近命令失败，可使用 /retry 继续修复。\n");
        } else {
            summary.push_str("状态: 本回合执行完成，可继续下一任务或 /status 查看进度。\n");
        }
        summary
    }
}

fn default_plan_steps() -> Vec<PlanStep> {
    [
        "理解需求并定位相关代码",
        "实施最小必要改动",
        "运行构建/测试验证",
        "总结结果与后续建议",
    ]
    .into_iter()
    .map(|d| PlanStep {
        description: d.into(),
        status: StepStatus::Pending,
    })
    .collect()
}

fn detect_verification_checks(workdir: &std::path::Path) -> Vec<VerificationCheck> {
    let mut checks = Vec::new();
    if workdir.join("Cargo.toml").exists() {
        checks.push(VerificationCheck {
            label: "编译检查".into(),
            command: Some("cargo build --release --locked".into()),
            passed: None,
            message: "Rust 项目建议运行".into(),
        });
        checks.push(VerificationCheck {
            label: "测试检查".into(),
            command: Some("cargo test".into()),
            passed: None,
            message: "如有测试，建议运行".into(),
        });
    } else if workdir.join("package.json").exists() {
        checks.push(VerificationCheck {
            label: "构建检查".into(),
            command: Some("npm run build".into()),
            passed: None,
            message: "Node 项目建议运行".into(),
        });
        checks.push(VerificationCheck {
            label: "测试检查".into(),
            command: Some("npm test".into()),
            passed: None,
            message: "如有测试，建议运行".into(),
        });
    } else if workdir.join("go.mod").exists() {
        checks.push(VerificationCheck {
            label: "编译检查".into(),
            command: Some("go build ./...".into()),
            passed: None,
            message: "Go 项目建议运行".into(),
        });
        checks.push(VerificationCheck {
            label: "测试检查".into(),
            command: Some("go test ./...".into()),
            passed: None,
            message: "建议运行".into(),
        });
    } else {
        checks.extend(default_verification_checks());
    }
    checks
}

fn default_verification_checks() -> Vec<VerificationCheck> {
    vec![VerificationCheck {
        label: "验证".into(),
        command: None,
        passed: None,
        message: "根据项目类型手动验证".into(),
    }]
}
