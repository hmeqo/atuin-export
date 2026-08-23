# atuin-export

Export [atuin](https://github.com/atuinsh/atuin) history to shell history files.

## Usage

```bash
atuin-export [<shell>] [options]
```

`<shell>` — optional, auto-detects from `$SHELL` when omitted (`fish`, `bash`, or `zsh`).

| Flag                  | Description                                                  |
| --------------------- | ------------------------------------------------------------ |
| `-o, --output <path>` | Output file path (defaults to shell's standard history file) |
| `-d, --db <path>`     | Atuin database path (default: `~/.local/share/atuin/history.db`) |

```bash
# Auto-detect shell from $SHELL
atuin-export

# Explicit shell
atuin-export fish

# Custom output
atuin-export bash --output ~/exported_history.txt

# Custom database
atuin-export zsh --db /path/to/atuin/history.db
```

The output file is regenerated from atuin: entries deleted in atuin
(`deleted_at` set) are removed from the file.

For fish, each entry includes a `paths:` block (arguments of the command that resolve
to existing files/directories, resolved against the directory the command ran in),
matching what fish itself records. Paths are checked against the filesystem at export
time, so files deleted since the command ran are omitted; paths under network or FUSE
mounts are recorded without a check, so an unreachable mount cannot stall the export.

For fish, consecutive duplicate commands within one shell session merge into a
single entry (keeping the latest run's timestamp), as fish itself does; the same
command in a new shell session stays a separate entry.

### Default output locations

| Shell | Default path                       |
| ----- | ---------------------------------- |
| fish  | `~/.local/share/fish/fish_history` |
| bash  | `$HISTFILE` or `~/.bash_history`   |
| zsh   | `$HISTFILE` or `~/.zsh_history`    |

## Dev

```bash
# build
cargo build --release

# cross-compile all platforms
./cross-build.sh
# outputs to ./dist/
```
