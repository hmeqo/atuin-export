use anyhow::{Context, Result};
use clap::Parser;

mod atuin;
mod cli;
mod export;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let home = dirs::home_dir().context("failed to get home dir")?;

    let db = args
        .db
        .unwrap_or_else(|| home.join(".local/share/atuin/history.db"));
    let entries = atuin::read_history(db)?;

    let output = args
        .output
        .unwrap_or_else(|| cli::default_history_path(args.shell, &home));

    match args.shell {
        cli::Shell::Fish => export::export_fish(&entries, output)?,
        cli::Shell::Bash => export::export_bash(&entries, output)?,
        cli::Shell::Zsh => export::export_zsh(&entries, output)?,
    }

    Ok(())
}
