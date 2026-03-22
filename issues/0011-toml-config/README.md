+++
status = "open"
opened = "2026-03-22"
+++

# Issue 11: TOML configuration and custom shell support

## Goal

Add a `config.toml` for shannon-specific settings, support custom shells via
configuration (no code changes needed), and rename `config.sh` to `env.sh` to
clarify its purpose.

## Background

Shannon currently has two configuration mechanisms:

1. **`config.sh`** — a bash script that sets environment variables (PATH, API
   keys, etc.). This is for importing the user's environment, not for
   configuring shannon itself.
2. **`SHANNON_DEFAULT_SHELL` env var** — set in `config.sh` to pick the default
   shell. This works but conflates shannon settings with environment setup.

Additionally, shell support is hardcoded in Rust — adding a new shell requires
modifying `shell.rs`, `executor.rs`, `highlighter.rs`, `main.rs`, and
`prompt.rs`. This means users can't add shells (like zsh, elvish, tcsh) without
recompiling.

### Design

#### No files are generated

Shannon does NOT write config files to disk. Defaults live in the binary.
If no `config.toml` exists, shannon works exactly as it does today. The user
only creates files when they want to change something.

This avoids the upgrade problem: when shannon ships a better default wrapper,
users who haven't customized get the improvement automatically. Users who have
customized are in control of their own config.

Documentation shows what the defaults look like so users can copy and modify.

#### Two config files, two purposes

| File | Purpose | Format |
|------|---------|--------|
| `env.sh` (was `config.sh`) | Environment setup (PATH, API keys) | Bash script |
| `config.toml` | Shannon settings (shells, defaults) | TOML |

`env.sh` runs once at startup to set up the environment. `config.toml` is
read once at startup to configure shannon itself. They serve completely
different purposes and should not be conflated.

Shannon checks `env.sh` first, falls back to `config.sh` for backward
compatibility.

#### config.toml structure

The config is **partial by default**. The user only specifies what they want
to change. Everything else uses built-in defaults.

Minimal example — just change the default shell:

```toml
default_shell = "nu"
```

Full example — add zsh and customize:

```toml
default_shell = "nu"

[shells.zsh]
binary = "zsh"
init = "shells/zsh/init.zsh"
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

This adds zsh to the rotation alongside the built-in shells. To override a
built-in shell's wrapper, redefine its section (e.g. `[shells.bash]`).

#### Wrapper templates

Each shell's wrapper is a string with three placeholders:

| Placeholder | Replaced with |
|-------------|---------------|
| `{{command}}` | The user's command |
| `{{temp_path}}` | Path to the temp file for env capture |
| `{{init}}` | Contents of the init script (or empty) |

The wrapper is responsible for running the command, capturing env/cwd/exit
code to the temp file. The user can see exactly what runs.

Built-in defaults match what's currently hardcoded in `executor.rs`.

#### Per-shell init scripts

Optional scripts that run before each command inside the wrapper. External
files with the correct extension so editors highlight them:

```
~/.config/shannon/shells/bash/init.sh
~/.config/shannon/shells/nu/init.nu
~/.config/shannon/shells/fish/init.fish
~/.config/shannon/shells/zsh/init.zsh
```

The `init` field is a path relative to the config directory. If the file
doesn't exist or the field is omitted, `{{init}}` expands to nothing. No
error.

Use cases:
- Load nushell standard library: `use std *`
- Set bash options: `shopt -s globstar`
- Define fish abbreviations
- Set up shell-specific aliases

#### Env parsers

The `parser` field tells shannon how to read the temp file:

| Parser | Format | Used by |
|--------|--------|---------|
| `bash` | `declare -x KEY="VALUE"` lines + `__SHANNON_*` markers | bash |
| `nushell` | JSON object from `$env \| to json` | nushell |
| `env` | `KEY=VALUE` lines (from `env` command) + `__SHANNON_*` markers | fish, zsh, any POSIX shell |

The `env` parser is the generic default. Most new shells will use it.

#### Syntax highlighting

Tree-sitter grammars are compile-time dependencies. Shannon ships grammars for
bash, nushell, and fish. The `highlighter` field maps a shell to a built-in
grammar:

```toml
[shells.zsh]
highlighter = "bash"  # use bash grammar for zsh (close enough)
```

Valid values: `bash`, `nushell`, `fish`. If omitted, no highlighting (plain
text input). The shell still works — it just doesn't have colored input.

#### Error handling

| Situation | Behavior |
|-----------|----------|
| No `config.toml` | Built-in defaults (current behavior) |
| Bad TOML syntax | Error message, exit |
| Shell missing `binary` | Error, skip that shell |
| Shell missing `wrapper` | Error, skip that shell |
| Shell binary not installed | Silently skip |
| Init file missing | Silent, `{{init}}` is empty |
| Init file has errors | Command fails, user sees the error |
| Wrapper produces unparseable output | Fall back to previous state |
| No shells available | Error message, exit |

Principle: config errors are loud, runtime errors are graceful.

### What TOML replaces

| Before | After |
|--------|-------|
| `SHANNON_DEFAULT_SHELL` env var | `default_shell` in config.toml |
| Hardcoded shell list in `main.rs` | `[shells.*]` tables in config.toml |
| Hardcoded prompt colors in `prompt.rs` | Either dropped or configurable |
| Per-shell wrapper functions in `executor.rs` | Wrapper templates in config.toml |
| No per-shell init scripts | `init` field + external files |
| `config.sh` name | `env.sh` (with `config.sh` fallback) |

### Migration

- `config.sh` continues to work (checked as fallback if `env.sh` doesn't
  exist)
- `SHANNON_DEFAULT_SHELL` env var continues to work (overridden by config.toml
  if both are set)
- If no config.toml exists, built-in defaults match current behavior exactly
