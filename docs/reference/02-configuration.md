# Configuration

Shannon stores its files in `~/.config/shannon/`. The config directory respects
`XDG_CONFIG_HOME` — if set, shannon uses `$XDG_CONFIG_HOME/shannon/` instead.

## Files

| File         | Purpose                                            |
| ------------ | -------------------------------------------------- |
| `config.sh`  | Startup script — sets PATH, env vars, API keys     |
| `history.db` | SQLite database storing all command history         |

The config directory is created automatically on first run.

## Startup Script (config.sh)

Shannon can run an optional bash script at startup to configure the
environment. Create `~/.config/shannon/config.sh` with any environment setup
you need:

```bash
# ~/.config/shannon/config.sh

# Homebrew
eval "$(/opt/homebrew/bin/brew shellenv)"

# Custom paths
export PATH="$PATH:$HOME/.cargo/bin"
export PATH="$PATH:$HOME/.local/bin"

# Environment variables
export EDITOR="nvim"
export ANTHROPIC_API_KEY="sk-ant-..."
```

This script runs once when shannon starts. The resulting environment is
captured and used for all sub-shell commands. If the file doesn't exist,
shannon inherits the environment from the launching terminal.

The script is always executed by bash — shannon requires bash, and the primary
use case (setting PATH and env vars) works perfectly in bash. If the script
fails, shannon prints a warning and continues with the inherited environment.

## History Database (history.db)

Command history is stored in a SQLite database shared across all shells and
instances. See [Command History](../features/04-history.md) for details.

## Platform Notes

Shannon uses `XDG_CONFIG_HOME` if set, otherwise `~/.config`. This applies on
all platforms, including macOS (where the Apple convention would be
`~/Library/Application Support`, but CLI tools universally use `~/.config`).
