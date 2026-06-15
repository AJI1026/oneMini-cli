mod agent;
mod cli;
mod compress;
mod config;
mod git;
mod hooks;
mod llm;
mod mcp;
mod permissions;
mod repl;
mod session;
mod slash;
mod tools;
mod ui;
mod update;
mod usage;
mod subagent;

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
