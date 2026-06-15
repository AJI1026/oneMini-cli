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
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Err(e) = cli.run().await {
        eprintln!("{}", ui::error(&e.to_string()));
        std::process::exit(1);
    }
    Ok(())
}
