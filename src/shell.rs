use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shell {
    Fish,
    Bash,
    Zsh,
}

impl Shell {
    pub fn detect() -> Option<Shell> {
        let shell = std::env::var("SHELL").ok()?;
        let name = Path::new(&shell).file_name()?.to_str()?;
        name.parse().ok()
    }

    pub fn default_history_path(self, home: &Path) -> std::path::PathBuf {
        match self {
            Shell::Fish => home.join(".local/share/fish/fish_history"),
            Shell::Bash => std::env::var("HISTFILE")
                .map(Into::into)
                .unwrap_or_else(|_| home.join(".bash_history")),
            Shell::Zsh => std::env::var("HISTFILE")
                .map(Into::into)
                .unwrap_or_else(|_| home.join(".zsh_history")),
        }
    }
}

impl FromStr for Shell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fish" => Ok(Shell::Fish),
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            _ => Err(format!("unknown shell: {s} (expected fish, bash, or zsh)")),
        }
    }
}
