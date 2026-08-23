//! Fish 4.x path detection: which arguments of a command resolve to existing
//! files. This mirrors fish's own `add_pending_with_file_detection` so exported
//! `paths:` blocks match what fish itself would record.

use std::path::{Path, PathBuf};
use std::str::Chars;

/// Commands for which fish skips file detection entirely: they either exit or
/// don't operate on file paths, and fish saves them synchronously.
const SYNC_WRITE_COMMANDS: [&str; 4] = ["echo", "exit", "reboot", "restart"];

/// Simulate fish 4.x `add_pending_with_file_detection`: collect the arguments of
/// `command` that resolve to an existing file or directory, resolved relative to
/// `cwd` (the directory the command ran in, which atuin records).
///
/// The original argument text is returned exactly as typed (`~`, quotes, relative
/// paths), matching what fish writes to `paths:`. This is an approximation:
/// existence is checked now, not at record time, and variables are expanded from
/// the current environment rather than the record-time one. Paths under
/// network/fuse mounts are recorded without a check, since a dead mount would
/// otherwise block the export.
pub fn detect_paths(command: &str, cwd: &str, home: &Path, mounts: &[PathBuf]) -> Vec<String> {
    let mut tokens = tokenize(command, cwd, home);
    classify_redirects(&mut tokens);

    if skips_detection(&tokens) {
        return Vec::new();
    }

    let mut paths = Vec::new();
    for t in &tokens {
        if t.is_sep || t.is_redirect || t.is_redirect_target || t.has_cmdsubst {
            continue;
        }
        if t.raw.is_empty() || t.raw.starts_with('-') || t.expanded.is_empty() {
            continue;
        }
        let candidate = if t.expanded.starts_with('/') {
            PathBuf::from(&t.expanded)
        } else {
            Path::new(cwd).join(&t.expanded)
        };
        let under_unreliable_mount = mounts
            .iter()
            .any(|m| normalize_lexically(&candidate).starts_with(m));
        if under_unreliable_mount || candidate.exists() {
            paths.push(t.raw.clone());
        }
    }
    paths
}

/// Mount points whose filesystem can hang on `stat` when unreachable. Paths
/// under these are recorded without an existence check. Linux-only: reads
/// `/proc/self/mounts`; other platforms return an empty list.
pub fn unreliable_mount_points() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/mounts")
            .map(|content| parse_mount_points(&content))
            .unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

/// Extract unreliable mount points from `/proc/self/mounts`-format text.
fn parse_mount_points(content: &str) -> Vec<PathBuf> {
    content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let (mount_point, fstype) = (parts.get(1)?, parts.get(2)?);
            is_unreliable_fs(fstype).then(|| PathBuf::from(mount_point))
        })
        .collect()
}

fn is_unreliable_fs(fstype: &str) -> bool {
    fstype.starts_with("fuse")
        || matches!(
            fstype,
            "nfs" | "nfs4" | "cifs" | "smb3" | "sshfs" | "davfs" | "davfs2"
        )
}

/// Resolve `.` and `..` components lexically, without touching the filesystem.
fn normalize_lexically(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// fish skips file detection for the whole pipeline if any statement would exit,
/// is `echo`, or is `exec`-decorated.
fn skips_detection(tokens: &[Token]) -> bool {
    let mut statement_start = true;
    for t in tokens {
        if t.is_sep {
            statement_start = true;
            continue;
        }
        if !statement_start || t.is_redirect || t.is_redirect_target {
            continue;
        }
        if t.raw == "exec" {
            return true;
        }
        if t.raw == "command" {
            continue; // fish decoration; the next token is the real command
        }
        statement_start = false;
        if SYNC_WRITE_COMMANDS.contains(&t.expanded.as_str()) {
            return true;
        }
    }
    false
}

#[derive(Default)]
struct Token {
    /// The argument exactly as typed, including quotes and escapes.
    raw: String,
    /// The argument with quotes/escapes resolved and `~`, `$HOME`, `$PWD`, and
    /// other environment variables expanded; used for existence checks.
    expanded: String,
    /// `|`, `;`, `&&`, `||`, `&` — statement/pipeline boundary.
    is_sep: bool,
    is_redirect: bool,
    is_redirect_target: bool,
    has_cmdsubst: bool,
}

/// A character stream with one-character lookahead, used by the tokenizer.
type PeekableChars<'a> = std::iter::Peekable<Chars<'a>>;

/// A fish-flavored tokenizer. Produces one token per word, following fish quoting
/// rules closely enough for path detection: single quotes take everything
/// literally, double quotes allow `$` and a few escapes, backslash escapes the
/// next char, `#` starts a comment at a word boundary.
fn tokenize(command: &str, cwd: &str, home: &Path) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cur = Token::default();
    let mut chars = command.chars().peekable();
    let (mut single, mut dbl) = (false, false);

    while let Some(c) = chars.next() {
        if single {
            if c == '\'' {
                single = false;
                cur.raw.push(c);
            } else {
                push_char(&mut cur, c);
            }
            continue;
        }
        if dbl {
            match c {
                '"' => {
                    dbl = false;
                    cur.raw.push(c);
                }
                '\\' => match chars.peek() {
                    Some('$') | Some('`') | Some('"') | Some('\\') => {
                        // Escaped char: keep the backslash in raw, drop it from
                        // the expanded value (`"\$HOME"` expands to `$HOME`).
                        let n = chars.next().unwrap();
                        cur.raw.push('\\');
                        cur.raw.push(n);
                        cur.expanded.push(n);
                    }
                    Some('\n') => {
                        cur.raw.push('\\');
                        cur.raw.push('\n');
                        cur.expanded.push('\n');
                        chars.next();
                    }
                    _ => push_char(&mut cur, '\\'), // literal backslash in the value
                },
                '$' => read_var(&mut cur, &mut chars, cwd, home),
                _ => push_char(&mut cur, c),
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                cur.raw.push(c);
                if c == '\'' {
                    single = true;
                } else {
                    dbl = true;
                }
            }
            '\\' => {
                let Some(n) = chars.next() else {
                    cur.raw.push('\\');
                    break;
                };
                cur.raw.push('\\');
                cur.raw.push(n);
                cur.expanded.push(n);
            }
            '$' => read_var(&mut cur, &mut chars, cwd, home),
            '~' if cur.raw.is_empty() => {
                cur.raw.push('~');
                // `~` expands at a word boundary: followed by `/`, whitespace, a
                // separator, or end of input. `~user` is left literal (unsupported)
                // and fails the existence check.
                match chars.peek() {
                    Some('/') | None => cur.expanded.push_str(&home.to_string_lossy()),
                    Some(&c) if c.is_whitespace() || ";&|<>^".contains(c) => {
                        cur.expanded.push_str(&home.to_string_lossy())
                    }
                    _ => cur.expanded.push('~'),
                }
            }
            '#' if cur.raw.is_empty() => break, // comment to end of line
            '|' | ';' => {
                flush(&mut tokens, &mut cur);
                tokens.push(sep_token(c));
            }
            '(' => {
                cur.raw.push('(');
                consume_cmdsubst(&mut cur, &mut chars);
            }
            '&' if ends_in_redirect_op(&cur.raw) => {
                // Part of an fd redirect like `2>&1` — not a separator.
                push_char(&mut cur, '&');
            }
            '&' => {
                flush(&mut tokens, &mut cur);
                let is_andand = chars.peek() == Some(&'&');
                if is_andand {
                    chars.next();
                }
                let mut sep = sep_token('&');
                if is_andand {
                    sep.raw.push('&');
                }
                tokens.push(sep);
            }
            c if c.is_whitespace() => flush(&mut tokens, &mut cur),
            c => push_char(&mut cur, c),
        }
    }
    flush(&mut tokens, &mut cur);
    tokens
}

fn push_char(cur: &mut Token, c: char) {
    cur.raw.push(c);
    cur.expanded.push(c);
}

/// Consume a balanced `( ... )` group into the current token and mark it as
/// command substitution (fish skips such arguments, `FAIL_ON_CMDSUBST`). Only
/// the raw text is kept; the token is excluded from path detection.
fn consume_cmdsubst(cur: &mut Token, chars: &mut PeekableChars) {
    cur.has_cmdsubst = true;
    let mut depth = 1;
    for c in chars.by_ref() {
        cur.raw.push(c);
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
}

fn sep_token(c: char) -> Token {
    Token {
        raw: c.to_string(),
        is_sep: true,
        ..Token::default()
    }
}

fn flush(tokens: &mut Vec<Token>, cur: &mut Token) {
    if !cur.raw.is_empty() {
        tokens.push(std::mem::take(cur));
    }
}

/// Expand `$NAME` / `${NAME}`. Raw text keeps the original `$...`. HOME and PWD
/// resolve from the entry's context; other variables are expanded from the
/// current environment (approximating record-time values). Unset variables
/// expand to a literal `$`, so the existence check fails and they are excluded.
fn read_var(cur: &mut Token, chars: &mut PeekableChars, cwd: &str, home: &Path) {
    cur.raw.push('$');
    if chars.peek() == Some(&'(') {
        cur.raw.push('(');
        chars.next();
        consume_cmdsubst(cur, chars);
        return;
    }
    let mut name = String::new();
    if chars.peek() == Some(&'{') {
        cur.raw.push('{');
        chars.next();
        while let Some(&c) = chars.peek() {
            if c == '}' {
                break;
            }
            cur.raw.push(c);
            name.push(c);
            chars.next();
        }
        if chars.peek() == Some(&'}') {
            cur.raw.push('}');
            chars.next();
        }
    } else {
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                cur.raw.push(c);
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
    }
    match name.as_str() {
        "HOME" => cur.expanded.push_str(&home.to_string_lossy()),
        "PWD" => cur.expanded.push_str(cwd),
        _ => match std::env::var(&name) {
            // fish expands every variable at record time; we approximate with the
            // current environment (same class of approximation as the existence check).
            Ok(v) => cur.expanded.push_str(&v),
            Err(_) => cur.expanded.push('$'),
        },
    }
}

/// Mark redirect operators and their targets so they are excluded from paths.
fn classify_redirects(tokens: &mut [Token]) {
    for i in 0..tokens.len() {
        if tokens[i].is_sep {
            continue;
        }
        if is_redirect_start(&tokens[i].raw) {
            tokens[i].is_redirect = true;
            if !self_contained_redirect(&tokens[i].raw)
                && let Some(next) = tokens.get_mut(i + 1)
                && !next.is_sep
            {
                next.is_redirect_target = true;
            }
        }
    }
}

/// Index of the redirect operator (`>`, `<`, `^`) after an optional fd prefix,
/// e.g. `2` in `2>&1`. `None` if the token doesn't start a redirect.
fn redirect_op_index(raw: &str) -> Option<usize> {
    let b = raw.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    matches!(b.get(i), Some(b'>') | Some(b'<') | Some(b'^')).then_some(i)
}

fn is_redirect_start(raw: &str) -> bool {
    redirect_op_index(raw).is_some()
}

/// Redirects that carry their own target and don't consume the next token:
/// `>|`, `N>|`, `N>&M`.
fn self_contained_redirect(raw: &str) -> bool {
    let Some(op) = redirect_op_index(raw) else {
        return false;
    };
    let rest = &raw.as_bytes()[op + 1..];
    rest == b"|" || is_fd_redirect(rest)
}

/// `&<digits>` — an fd-to-fd redirect target like `2>&1`.
fn is_fd_redirect(rest: &[u8]) -> bool {
    rest.len() >= 2 && rest[0] == b'&' && rest[1..].iter().all(|c| c.is_ascii_digit())
}

/// True when the token so far is an unfinished redirect operator (`>`, `<`, `^`,
/// optionally fd-prefixed), so a following `&` continues it into `2>&1`.
fn ends_in_redirect_op(raw: &str) -> bool {
    raw.ends_with(['>', '<', '^'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a fixture tree: `<tmp>/home` with `x`, `sub/`; `<tmp>/work` with
    /// `.backlog`, `a.txt`, `b.txt`, `my file.txt`.
    fn fixture() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let work = dir.path().join("work");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(home.join("sub")).unwrap();
        fs::write(home.join("x"), b"").unwrap();
        fs::create_dir_all(&work).unwrap();
        for f in [".backlog", "a.txt", "b.txt", "my file.txt"] {
            fs::write(work.join(f), b"").unwrap();
        }
        let cwd = work.to_str().unwrap().to_string();
        (dir, home.to_str().unwrap().to_string(), cwd)
    }

    #[test]
    fn detect_paths_matches_fish_semantics() {
        let (dir, home, cwd) = fixture();
        let cases: &[(&str, &[&str])] = &[
            ("ll .backlog", &[".backlog"]),
            ("git pull", &[]),
            ("echo hello", &[]),
            ("cat a.txt | cat > out.txt", &["a.txt"]),
            ("rm *", &[]),
            ("cat \"my file.txt\"", &["\"my file.txt\""]),
            ("rg x ~ 2>/dev/null", &["~"]),
            ("cat $(ls a.txt)", &[]),
            ("cd ~; ll", &["~"]),
            ("cat ~/x", &["~/x"]),
            ("cd ~/sub", &["~/sub"]),
            ("exec foo bar", &[]),
            ("--flag a.txt", &["a.txt"]),
            ("sudo rm -rf /nonexistent-xyz/", &[]),
            ("printf 'a\\nb'", &[]),
            ("cat ./a.txt", &["./a.txt"]),
            ("ls 'my file.txt'", &["'my file.txt'"]),
            ("cmd 2>&1 > out.txt", &[]),
            ("sleep 5 & echo a.txt", &[]),
        ];
        for (cmd, expected) in cases {
            let actual = detect_paths(cmd, &cwd, Path::new(&home), &[]);
            assert_eq!(actual, expected.to_vec(), "command: {cmd}");
        }
        drop(dir);
    }

    #[test]
    fn detect_paths_unknown_cwd_drops_relative_paths() {
        let (dir, home, _) = fixture();
        assert_eq!(
            detect_paths("go mod init .", "unknown", Path::new(&home), &[]),
            Vec::<String>::new()
        );
        drop(dir);
    }

    #[test]
    fn detect_paths_expands_environment_variables() {
        let (dir, home, cwd) = fixture();
        let sub = Path::new(&home).join("sub");
        // SAFETY: test-only env mutation with a unique variable name; no other
        // test reads it.
        unsafe { std::env::set_var("ATUIN_EXPORT_TEST_DIR", &sub) };
        assert_eq!(
            detect_paths(
                "mv a.txt $ATUIN_EXPORT_TEST_DIR/",
                &cwd,
                Path::new(&home),
                &[],
            ),
            vec!["a.txt", "$ATUIN_EXPORT_TEST_DIR/"]
        );
        drop(dir);
    }

    #[test]
    fn detect_paths_records_paths_under_unreliable_mounts() {
        let (dir, home, cwd) = fixture();
        // The path doesn't exist, but its mount is flagged unreliable (e.g. a
        // dead rclone mount): record it instead of stat-ing (which would hang).
        let mounts = [PathBuf::from("/fake/mnt")];
        assert_eq!(
            detect_paths("cat /fake/mnt/gone.txt", &cwd, Path::new(&home), &mounts),
            vec!["/fake/mnt/gone.txt"]
        );
        // `..` inside the mount subtree is resolved before the prefix check.
        assert_eq!(
            detect_paths(
                "cat /fake/mnt/sub/../gone.txt",
                &cwd,
                Path::new(&home),
                &mounts
            ),
            vec!["/fake/mnt/sub/../gone.txt"]
        );
        drop(dir);
    }

    #[test]
    fn normalize_lexically_resolves_dotdot() {
        assert_eq!(
            normalize_lexically(Path::new("/home/user/../mnt")),
            PathBuf::from("/home/mnt")
        );
        assert_eq!(
            normalize_lexically(Path::new("/mnt/dav/sub/..")),
            PathBuf::from("/mnt/dav")
        );
    }

    #[test]
    fn double_quoted_escapes_drop_backslash_in_expanded() {
        // `"\$HOME"` — escaped `$` is literal; raw keeps the backslash, the
        // expanded value drops it.
        let tokens = tokenize("cat \"\\$HOME\"", "/tmp", Path::new("/home"));
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1].raw, "\"\\$HOME\"");
        assert_eq!(tokens[1].expanded, "$HOME");
    }

    #[test]
    fn classifies_unreliable_filesystems() {
        for fs in [
            "fuse",
            "fuse.rclone",
            "fuse.sshfs",
            "fuseblk",
            "nfs",
            "nfs4",
            "cifs",
            "smb3",
            "sshfs",
            "davfs",
            "davfs2",
        ] {
            assert!(is_unreliable_fs(fs), "{fs} should be unreliable");
        }
        for fs in ["ext4", "btrfs", "tmpfs", "overlay", "xfs", "vfat"] {
            assert!(!is_unreliable_fs(fs), "{fs} should be reliable");
        }
    }

    #[test]
    fn parses_mount_points() {
        let content = concat!(
            "/dev/nvme0n1p2 / ext4 rw 0 0\n",
            "/dev/nvme0n1p1 /mnt/data1 btrfs rw 0 0\n",
            "rclone: /mnt/dav fuse.rclone rw,nofail 0 0\n",
            "192.168.1.5:/srv /mnt/nas nfs rw 0 0\n",
            "tmpfs /tmp tmpfs rw 0 0\n",
        );
        assert_eq!(
            parse_mount_points(content),
            vec![PathBuf::from("/mnt/dav"), PathBuf::from("/mnt/nas")]
        );
    }
}
