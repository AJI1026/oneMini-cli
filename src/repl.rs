use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::agent::AgentSession;
use crate::agent::AgentOptions;
use crate::ui;

pub struct Repl {
    editor: DefaultEditor,
    session: AgentSession,
}

impl Repl {
    pub fn new(opts: AgentOptions) -> Result<Self> {
        let editor = DefaultEditor::new()?;
        let session = AgentSession::new(opts)?;
        Ok(Self { editor, session })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("{}", ui::banner());
        println!(
            "{}",
            ui::dim(&format!(
                "工作目录: {}",
                self.session.opts.config.workdir().display()
            ))
        );
        println!();

        loop {
            let prompt = format!("{} ", ui::user_prefix());
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let input = line.trim();
                    if input.is_empty() {
                        continue;
                    }
                    self.editor.add_history_entry(input)?;

                    if input.starts_with('/') {
                        if self.handle_slash_command(input).await? {
                            break;
                        }
                        continue;
                    }

                    match self.session.run_turn(input, true).await {
                        Ok(_) => println!(),
                        Err(e) => println!("{}\n", ui::error(&e.to_string())),
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!();
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!();
                    break;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    async fn handle_slash_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts.first().copied() {
            Some("/exit") | Some("/quit") => return Ok(true),
            Some("/help") => {
                println!(
                    "\n{}\n  /help   显示帮助\n  /clear  清空对话历史\n  /config 显示配置\n  /exit   退出\n",
                    ui::dim("可用命令:")
                );
            }
            Some("/clear") => {
                let workdir = self.session.opts.config.workdir().to_path_buf();
                let opts = self.session.opts.clone();
                self.session = AgentSession::new(AgentOptions {
                    config: {
                        let mut c = opts.config.clone();
                        c.workdir = Some(workdir);
                        c
                    },
                    max_rounds: opts.max_rounds,
                    auto_approve: opts.auto_approve,
                })?;
                println!("{}", ui::success("对话历史已清空"));
            }
            Some("/config") => {
                println!("{}", self.session.opts.config.display());
            }
            Some(cmd) => {
                println!("{}", ui::warn(&format!("未知命令: {cmd}，输入 /help 查看帮助")));
            }
            None => {}
        }
        Ok(false)
    }
}
