use anyhow::{Context, Result};
use clap::Parser;

mod atuin;
mod cli;
mod export;
mod shell;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let home = dirs::home_dir().context("failed to get home dir")?;

    let shell = args
        .shell
        .or_else(shell::Shell::detect)
        .context("shell argument required and $SHELL is not set")?;

    let db = args
        .db
        .unwrap_or_else(|| home.join(".local/share/atuin/history.db"));
    let entries = atuin::read_history(db)?;

    let output = args
        .output
        .unwrap_or_else(|| shell.default_history_path(&home));

    match shell {
        shell::Shell::Fish => export::export_fish(&entries, output)?,
        shell::Shell::Bash => export::export_bash(&entries, output)?,
        shell::Shell::Zsh => export::export_zsh(&entries, output)?,
    }

    Ok(())
}
