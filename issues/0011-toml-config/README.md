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

### What changes

**Rename `config.sh` → `env.sh`** — clearer name. It's an environment setup
script, not a shannon config file. Shannon should support both names during a
transition period (check `env.sh` first, fall back to `config.sh`).

**Add `config.toml`** — static configuration for shannon itself. Shell
definitions use configurable wrapper templates with `{{placeholder}}` syntax:

```toml
default_shell = "nu"

[shells.bash]
binary = "bash"
init = "shells/bash/init.sh"
wrapper = """
{{init}}
{{command}}
__shannon_ec=$?
(export -p; echo "__SHANNON_CWD=$(pwd)"; echo "__SHANNON_EXIT=$__shannon_ec") > '{{temp_path}}'
exit $__shannon_ec
"""
parser = "bash"

[shells.nu]
binary = "nu"
init = "shells/nu/init.nu"
wrapper = """
{{init}}
let __shannon_out = (try { {{command}} } catch { |e| $e.rendered | print -e; null })
if ($__shannon_out != null) and (($__shannon_out | describe) != "nothing") { $__shannon_out | print }
let shannon_exit = (if ($env | get -o LAST_EXIT_CODE | is-not-empty) { $env.LAST_EXIT_CODE } else { 0 })
$env | reject config? | insert __SHANNON_CWD (pwd) | insert __SHANNON_EXIT ($shannon_exit | into string) | to json --serialize | save --force '{{temp_path}}'
"""
parser = "nushell"

[shells.fish]
binary = "fish"
init = "shells/fish/init.fish"
wrapper = """
{{init}}
{{command}}
set __shannon_ec $status
env > '{{temp_path}}'
echo "__SHANNON_CWD="(pwd) >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
"""
parser = "env"

[shells.zsh]
binary = "zsh"
init = "shells/zsh/init.zsh"
wrapper = """
{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
"""
parser = "env"
```

### Wrapper templates

Each shell defines its own wrapper script as a TOML string with three
placeholders:

| Placeholder | Replaced with |
|-------------|---------------|
| `{{command}}` | The user's command |
| `{{temp_path}}` | Path to the temp file for env capture |
| `{{init}}` | Contents of the init script (or empty) |

The wrapper is responsible for:
1. Running the init script (if any)
2. Running the user's command
3. Capturing environment variables, cwd, and exit code to the temp file

This makes each shell's behavior explicit and transparent. The user can see
exactly what runs and modify it if needed.

### Per-shell init scripts

Optional scripts that run before each command inside the wrapper. These are
external files so editors can highlight them with the correct syntax:

```
~/.config/shannon/shells/bash/init.sh
~/.config/shannon/shells/nu/init.nu
~/.config/shannon/shells/fish/init.fish
~/.config/shannon/shells/zsh/init.zsh
```

The `init` field in config.toml is a path relative to the config directory.
If the file doesn't exist or the field is omitted, `{{init}}` expands to
nothing.

Use cases:
- Load nushell standard library: `use std *`
- Set bash options: `shopt -s globstar`
- Define fish abbreviations
- Set up shell-specific aliases

### Env parsers

The `parser` field tells shannon how to read the temp file. Three parsers
cover all cases:

| Parser | Format | Used by |
|--------|--------|---------|
| `bash` | `declare -x KEY="VALUE"` lines + `__SHANNON_*` markers | bash |
| `nushell` | JSON object from `$env \| to json` | nushell |
| `env` | `KEY=VALUE` lines (from `env` command) + `__SHANNON_*` markers | fish, zsh, and any POSIX shell |

The `env` parser is the generic default. Most new shells will use it.

### Syntax highlighting

Tree-sitter grammars are compile-time dependencies — they can't be added via
config. Shannon ships grammars for bash, nushell, and fish. For other shells,
a `highlighter` field maps to a built-in grammar:

```toml
[shells.zsh]
highlighter = "bash"  # use bash grammar for zsh (close enough)
```

If omitted, no highlighting (plain text). Valid values: `bash`, `nushell`,
`fish`, or omitted for none.

### Shell colors

Currently each shell has a hardcoded prompt color (bash=green, nushell=cyan,
fish=yellow). Options:

1. **Drop per-shell colors** — use a single prompt color for all shells. The
   `[shell_name]` text already identifies which shell is active.
2. **Make colors configurable in TOML** — `color = "green"` per shell.
3. **Auto-assign colors** — cycle through a palette based on shell order.

Decision can be made during implementation.

### Config file location

`~/.config/shannon/config.toml` — same directory as `env.sh` and `history.db`.
Respects `XDG_CONFIG_HOME`.

### What TOML replaces

| Before | After |
|--------|-------|
| `SHANNON_DEFAULT_SHELL` env var | `default_shell` in config.toml |
| Hardcoded shell list in `main.rs` | `[shells.*]` tables in config.toml |
| Hardcoded prompt colors in `prompt.rs` | Either dropped or configurable |
| Per-shell wrapper functions in `executor.rs` | Wrapper templates in config.toml |
| No per-shell init scripts | `init` field + external files |

### Migration

- `config.sh` continues to work (checked as fallback if `env.sh` doesn't exist)
- `SHANNON_DEFAULT_SHELL` env var continues to work (overridden by config.toml
  if both are set)
- If no config.toml exists, shannon uses built-in defaults matching current
  behavior (bash, nushell, fish with their current wrappers)
