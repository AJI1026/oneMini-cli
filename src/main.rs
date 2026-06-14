mod agent;
mod cli;
mod config;
mod llm;
mod repl;
mod tools;
mod ui;

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
