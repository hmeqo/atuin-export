use std::path::PathBuf;

use clap::Parser;

use crate::shell::Shell;

#[derive(Parser)]
#[command(name = "atuin-export", version, about = "Export atuin history to shell history files")]
pub struct Cli {
    #[arg(help = "Target shell: fish, bash, or zsh (defaults to $SHELL)")]
    pub shell: Option<Shell>,

    #[arg(short, long, help = "Output history file path (defaults to shell's standard location)")]
    pub output: Option<PathBuf>,

    #[arg(short, long, value_name = "PATH", help = "Atuin database path (defaults to ~/.local/share/atuin/history.db)")]
    pub db: Option<PathBuf>,
}
