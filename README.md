# atuin-export

Export [atuin](https://github.com/atuinsh/atuin) history to shell history files.

## Usage

```bash
atuin-export <fish|bash|zsh> [options]
```

| Flag                  | Description                                                      |
| --------------------- | ---------------------------------------------------------------- |
| `-o, --output <path>` | Output file path (defaults to shell's standard history file)     |
| `-d, --db <path>`     | Atuin database path (default: `~/.local/share/atuin/history.db`) |

```bash
# Append to fish history (default location)
atuin-export fish

# Export to a custom file
atuin-export bash --output ~/exported_history.txt

# Use a different atuin database
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
