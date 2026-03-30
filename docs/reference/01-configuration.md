# Configuration

Shannon stores its files in `~/.config/shannon/`. The config directory respects
`XDG_CONFIG_HOME` — if set, shannon uses `$XDG_CONFIG_HOME/shannon/` instead.

## Files

| File | Purpose |
|------|---------|
| `env.sh` | Bash environment setup — PATH, env vars, API keys (runs first) |
| `env.nu` | Nushell env setup (runs after env.sh) |
| `config.nu` | Nushell config — keybindings, colors, hooks, completions |
| `login.nu` | Login shell config |
| `history.sqlite3` | SQLite command history |

None of these files are required. Shannon works out of the box.

## env.sh (Bash Environment)

Shannon runs an optional bash script at startup via brush to set up the
environment. Create `~/.config/shannon/env.sh`:

```bash
# ~/.config/shannon/env.sh

# Homebrew
eval "$(/opt/homebrew/bin/brew shellenv)"

# Custom paths
export PATH="$PATH:$HOME/.cargo/bin"
export PATH="$PATH:$HOME/.local/bin"

# Environment variables
export EDITOR="nvim"
export ANTHROPIC_API_KEY="sk-ant-..."
```

This runs once when shannon starts, before `env.nu` and `config.nu`. The
resulting environment variables are injected into nushell's Stack.

This is critical for compatibility — tutorials and AI always give instructions
as "add this to your .bashrc." Shannon's `env.sh` lets you follow those
instructions directly.

## env.nu and config.nu (Nushell Config)

These are nushell's native config files. See the
[Nushell configuration docs](https://nushell.sh/book/configuration.html)
for full details.

Common settings:

```nushell
# ~/.config/shannon/config.nu

$env.config.show_banner = false          # disable startup banner
$env.config.edit_mode = "vi"             # vi or emacs keybindings
$env.config.history.file_format = "sqlite"
```

## Banner

The startup banner respects nushell's `$env.config.show_banner` setting:

```nushell
$env.config.show_banner = false    # no banner
$env.config.show_banner = "short"  # just startup time
$env.config.show_banner = true     # full welcome message (default)
```
