mod agent;
mod cli;
mod compress;
mod config;
mod fs_util;
mod git;
mod hooks;
mod install;
mod llm;
mod managed;
mod mcp;
mod permissions;
mod repl;
mod sandbox;
mod session;
mod session_crypto;
mod skills;
mod slash;
mod tools;
mod ui;
mod update;
mod usage;
mod subagent;
mod worktree;

use anyhow::Result;
use clap::FromArgMatches;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Cli::command_with_hints();
    let matches = match cmd.try_get_matches() {
        Ok(matches) => matches,
        Err(err) => err.exit(),
    };
    let cli = Cli::from_arg_matches(&matches)?;
    if let Err(e) = cli.run().await {
        eprintln!("{}", ui::error(&e.to_string()));
        std::process::exit(1);
    }
    Ok(())
}
