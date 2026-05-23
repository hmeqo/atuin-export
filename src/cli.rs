use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shell {
    Fish,
    Bash,
    Zsh,
}

#[derive(Parser)]
#[command(name = "atuin-export", version, about = "Export atuin history to shell history files")]
pub struct Cli {
    #[arg(value_parser = parse_shell, help = "Target shell: fish, bash, or zsh")]
    pub shell: Shell,

    #[arg(short, long, help = "Output history file path (defaults to shell's standard location)")]
    pub output: Option<PathBuf>,

    #[arg(short, long, value_name = "PATH", help = "Atuin database path (defaults to ~/.local/share/atuin/history.db)")]
    pub db: Option<PathBuf>,
}

pub fn default_history_path(shell: Shell, home: &std::path::Path) -> PathBuf {
    match shell {
        Shell::Fish => home.join(".local/share/fish/fish_history"),
        Shell::Bash => std::env::var("HISTFILE")
            .map(Into::into)
            .unwrap_or_else(|_| home.join(".bash_history")),
        Shell::Zsh => std::env::var("HISTFILE")
            .map(Into::into)
            .unwrap_or_else(|_| home.join(".zsh_history")),
    }
}

fn parse_shell(s: &str) -> Result<Shell, String> {
    match s.to_lowercase().as_str() {
        "fish" => Ok(Shell::Fish),
        "bash" => Ok(Shell::Bash),
        "zsh" => Ok(Shell::Zsh),
        _ => Err(format!("unknown shell: {s} (expected fish, bash, or zsh)")),
    }
}
