# Configuration

Shannon stores its files in `~/.config/shannon/`. The config directory respects
`XDG_CONFIG_HOME` — if set, shannon uses `$XDG_CONFIG_HOME/shannon/` instead.

## Files

| File           | Purpose                                        |
| -------------- | ---------------------------------------------- |
| `config.toml`  | Shannon settings — default shell, custom shells |
| `env.sh`       | Environment setup — PATH, env vars, API keys   |
| `history.db`   | SQLite database storing all command history     |

None of these files are required. Shannon works out of the box with no
configuration.

## Shannon Settings (config.toml)

`config.toml` configures shannon itself. If the file doesn't exist, built-in
defaults are used. You only need to create it when you want to change
something.

### Change the default shell

```toml
default_shell = "nu"
```

### Add a custom shell

Add any shell that supports `-c` for command execution:

```toml
[shells.zsh]
binary = "zsh"
highlighter = "bash"
parser = "env"
wrapper = """
{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
"""
```

This adds zsh to the Shift+Tab rotation alongside the built-in shells.

### Shell config fields

| Field | Required | Description |
|-------|----------|-------------|
| `binary` | Yes | Path or name of the shell binary |
| `wrapper` | Yes | Wrapper template (see below) |
| `parser` | No | Output parser: `bash`, `nushell`, or `env` (default: `env`) |
| `highlighter` | No | Syntax highlighting: `bash`, `nushell`, `fish`, or omitted |
| `init` | No | Path to init script, relative to config dir |

### Wrapper templates

The wrapper is a script that runs the user's command and captures the
resulting environment. It uses three placeholders:

| Placeholder | Replaced with |
|-------------|---------------|
| `{{command}}` | The user's command |
| `{{temp_path}}` | Path to the temp file for env capture |
| `{{init}}` | Contents of the init script (or empty) |

### Per-shell init scripts

Optional scripts that run before each command. Create them with the correct
file extension for editor highlighting:

```
~/.config/shannon/shells/nu/init.nu
~/.config/shannon/shells/bash/init.sh
~/.config/shannon/shells/fish/init.fish
```

Reference them in config.toml:

```toml
[shells.nu]
init = "shells/nu/init.nu"
```

Use cases: load nushell stdlib (`use std *`), set bash options (`shopt -s
globstar`), define aliases.

### Overriding built-in shells

To change a built-in shell's behavior, redefine its section:

```toml
[shells.bash]
binary = "/opt/homebrew/bin/bash"
wrapper = "..."
parser = "bash"
```

## Environment Script (env.sh)

Shannon runs an optional bash script at startup to set up the environment.
Create `~/.config/shannon/env.sh`:

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

This runs once when shannon starts. The resulting environment is captured and
used for all sub-shell commands. If the file doesn't exist, shannon checks
for `config.sh` as a fallback (backward compatibility).

The script is always executed by bash. If it fails, shannon prints a warning
and continues with the inherited environment.

## Shannon Environment Variables

Shannon recognizes these env vars (from `env.sh` or the inherited environment):

| Variable | Values | Default | Purpose |
|----------|--------|---------|---------|
| `SHANNON_DEFAULT_SHELL` | `bash`, `nu`, `fish`, etc. | `bash` | Fallback if no config.toml |
| `SHANNON_DEPTH` | (set automatically) | — | Nesting depth, shown as `>>` in prompt |

`config.toml`'s `default_shell` takes precedence over the env var.

## History Database (history.db)

Command history is stored in a SQLite database shared across all shells and
instances. See [Command History](../features/04-history.md) for details.

## Platform Notes

Shannon uses `XDG_CONFIG_HOME` if set, otherwise `~/.config`. This applies on
all platforms, including macOS (where the Apple convention would be
`~/Library/Application Support`, but CLI tools universally use `~/.config`).
