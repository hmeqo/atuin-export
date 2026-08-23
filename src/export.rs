use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::atuin::HistoryEntry;
use crate::fish_paths;

/// Export entries to a fish history file, matching fish's own format: each
/// entry renders `- cmd:` / `  when:` plus a `paths:` block when the command's
/// arguments resolve to existing files.
pub fn export_fish<P: AsRef<Path>>(entries: &[HistoryEntry], home: &Path, output: P) -> Result<()> {
    let output = output.as_ref();
    let mounts = fish_paths::unreliable_mount_points();
    let mut file = temp_output(output)?;

    // fish merges consecutive duplicate commands within one session, keeping
    // the latest timestamp — mirror that by writing only the last entry of
    // each same-session run.
    let mut pending: Option<&HistoryEntry> = None;
    for entry in entries {
        if pending.is_some_and(|p| p.session == entry.session && p.command == entry.command) {
            pending = Some(entry);
            continue;
        }
        if let Some(prev) = pending.take() {
            write_fish_entry(&mut file, prev, home, &mounts)?;
        }
        pending = Some(entry);
    }
    if let Some(last) = pending {
        write_fish_entry(&mut file, last, home, &mounts)?;
    }

    commit_atomic(output, file)
}

/// Render one entry in fish's history format: `- cmd:` / `  when:` plus a
/// `paths:` block when the command's arguments resolve to existing files.
fn write_fish_entry(
    file: &mut std::fs::File,
    entry: &HistoryEntry,
    home: &Path,
    mounts: &[PathBuf],
) -> Result<()> {
    writeln!(
        file,
        "- cmd: {}\n  when: {}",
        escape_fish_yaml(&entry.command),
        entry.when()
    )
    .context("failed to write fish_history")?;
    let paths = fish_paths::detect_paths(&entry.command, &entry.cwd, home, mounts);
    if !paths.is_empty() {
        writeln!(file, "  paths:").context("failed to write fish_history")?;
        for path in paths {
            writeln!(file, "    - {}", escape_fish_yaml(&path))
                .context("failed to write fish_history")?;
        }
    }
    Ok(())
}

/// Export entries to a bash history file: `#<when>` timestamp line per entry.
pub fn export_bash<P: AsRef<Path>>(entries: &[HistoryEntry], output: P) -> Result<()> {
    let output = output.as_ref();
    let mut file = temp_output(output)?;

    for entry in entries {
        writeln!(file, "#{}\n{}", entry.when(), entry.command)
            .context("failed to write bash_history")?;
    }

    commit_atomic(output, file)
}

/// Export entries to a zsh history file: `: <when>:0;<cmd>` per entry.
pub fn export_zsh<P: AsRef<Path>>(entries: &[HistoryEntry], output: P) -> Result<()> {
    let output = output.as_ref();
    let mut file = temp_output(output)?;

    for entry in entries {
        writeln!(file, ": {}:0;{}", entry.when(), entry.command)
            .context("failed to write zsh_history")?;
    }

    commit_atomic(output, file)
}

/// Escape a string the way fish serializes history: `\` -> `\\`, newline -> literal `\n`.
fn escape_fish_yaml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

/// Create a temporary sibling of `output`; [`commit_atomic`] moves it into place.
fn temp_output(output: &Path) -> Result<std::fs::File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(temp_path(output))
        .context("failed to create temporary output file")
}

/// Flush and atomically replace `output` with the temporary file, so a failed
/// export never leaves a truncated history file.
fn commit_atomic(output: &Path, file: std::fs::File) -> Result<()> {
    file.sync_all()
        .context("failed to flush temporary output file")?;
    std::fs::rename(temp_path(output), output).context("failed to replace history file")
}

/// A sibling temporary path used for atomic replacement.
fn temp_path(output: &Path) -> PathBuf {
    PathBuf::from(format!("{}.tmp", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_entries(dir: &Path) -> Vec<HistoryEntry> {
        vec![
            HistoryEntry {
                timestamp_ns: 100_000_000_000,
                command: "git pull".into(),
                cwd: String::new(),
                session: "s1".into(),
            },
            HistoryEntry {
                timestamp_ns: 200_000_000_000,
                command: "ll .backlog".into(),
                cwd: dir.to_str().unwrap().into(),
                session: "s1".into(),
            },
        ]
    }

    #[test]
    fn fish_export_writes_fish_format_with_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".backlog"), b"").unwrap();
        let out = dir.path().join("fish_history");
        export_fish(&sample_entries(dir.path()), dir.path(), &out).unwrap();
        let content = fs::read_to_string(&out).unwrap();
        assert_eq!(
            content,
            "- cmd: git pull\n  when: 100\n- cmd: ll .backlog\n  when: 200\n  paths:\n    - .backlog\n"
        );
        assert!(!temp_path(&out).exists(), "no temporary file left behind");
    }

    #[test]
    fn fish_export_regenerates_file() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("fish_history");
        // Stale entry from a previous export: full regeneration removes it.
        fs::write(&out, "- cmd: stale command\n  when: 50\n").unwrap();
        let entries = sample_entries(dir.path());
        export_fish(&entries, dir.path(), &out).unwrap();
        let content = fs::read_to_string(&out).unwrap();
        assert!(!content.contains("stale command"));
        assert_eq!(content.matches("- cmd:").count(), 2);

        // Idempotent: a second export produces identical content.
        export_fish(&entries, dir.path(), &out).unwrap();
        assert_eq!(fs::read_to_string(&out).unwrap(), content);
    }

    #[test]
    fn bash_and_zsh_export_render_their_formats() {
        let dir = tempfile::tempdir().unwrap();
        let entries = sample_entries(dir.path());

        let bash = dir.path().join("bash_history");
        export_bash(&entries, &bash).unwrap();
        assert_eq!(
            fs::read_to_string(&bash).unwrap(),
            "#100\ngit pull\n#200\nll .backlog\n"
        );

        let zsh = dir.path().join("zsh_history");
        export_zsh(&entries, &zsh).unwrap();
        assert_eq!(
            fs::read_to_string(&zsh).unwrap(),
            ": 100:0;git pull\n: 200:0;ll .backlog\n"
        );
    }

    #[test]
    fn escapes_backslashes_and_newlines() {
        assert_eq!(escape_fish_yaml("plain"), "plain");
        assert_eq!(escape_fish_yaml("a\\b"), "a\\\\b");
        assert_eq!(escape_fish_yaml("l1\nl2"), "l1\\nl2");
    }

    #[test]
    fn fish_export_merges_consecutive_duplicates_in_one_session() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("fish_history");
        let entries = vec![
            HistoryEntry {
                timestamp_ns: 100_000_000_000,
                command: "ll".into(),
                cwd: String::new(),
                session: "s1".into(),
            },
            HistoryEntry {
                timestamp_ns: 110_000_000_000,
                command: "ll".into(),
                cwd: String::new(),
                session: "s1".into(),
            },
            HistoryEntry {
                timestamp_ns: 120_000_000_000,
                command: "ll".into(),
                cwd: String::new(),
                session: "s1".into(),
            },
        ];
        export_fish(&entries, dir.path(), &out).unwrap();
        // One entry, carrying the latest run's `when` (120), matching fish's merge.
        assert_eq!(
            fs::read_to_string(&out).unwrap(),
            "- cmd: ll\n  when: 120\n"
        );
    }

    #[test]
    fn fish_export_keeps_duplicates_across_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("fish_history");
        // Same command, adjacent in time, but in different sessions (e.g. after
        // reopening a shell): fish does not merge across sessions.
        let entries = vec![
            HistoryEntry {
                timestamp_ns: 100_000_000_000,
                command: "ll".into(),
                cwd: String::new(),
                session: "s1".into(),
            },
            HistoryEntry {
                timestamp_ns: 110_000_000_000,
                command: "ll".into(),
                cwd: String::new(),
                session: "s2".into(),
            },
        ];
        export_fish(&entries, dir.path(), &out).unwrap();
        assert_eq!(
            fs::read_to_string(&out).unwrap(),
            "- cmd: ll\n  when: 100\n- cmd: ll\n  when: 110\n"
        );
    }
}
