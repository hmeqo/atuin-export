use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::atuin::HistoryEntry;

pub fn export_fish<P: AsRef<Path>>(entries: &[HistoryEntry], output: P) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)
        .context("failed to open fish_history")?;

    for entry in entries {
        let when = entry.timestamp_ns / 1_000_000_000;
        writeln!(file, "- cmd: {}\n  when: {}", entry.command, when)
            .context("failed to write fish_history")?;
    }

    Ok(())
}

pub fn export_bash<P: AsRef<Path>>(entries: &[HistoryEntry], output: P) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)
        .context("failed to open bash_history")?;

    for entry in entries {
        let when = entry.timestamp_ns / 1_000_000_000;
        writeln!(file, "#{}\n{}", when, entry.command)
            .context("failed to write bash_history")?;
    }

    Ok(())
}

pub fn export_zsh<P: AsRef<Path>>(entries: &[HistoryEntry], output: P) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(output)
        .context("failed to open zsh_history")?;

    for entry in entries {
        let when = entry.timestamp_ns / 1_000_000_000;
        writeln!(file, ": {}:0;{}", when, entry.command)
            .context("failed to write zsh_history")?;
    }

    Ok(())
}
