# atuin-export

Export [atuin](https://github.com/atuinsh/atuin) history to shell history files.

## Usage

```bash
atuin-export [<shell>] [options]
```

`<shell>` — optional, auto-detects from `$SHELL` when omitted (`fish`, `bash`, or `zsh`).

| Flag                  | Description                                                      |
| --------------------- | ---------------------------------------------------------------- |
| `-o, --output <path>` | Output file path (defaults to shell's standard history file)     |
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
