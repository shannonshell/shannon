# Architecture

This document explains how shannon works under the hood. It's aimed at someone
who wants to understand the design, not just use the shell.

## The Subprocess Model

Shannon spawns a fresh subprocess for every command. There are no persistent
shell sessions. When you type `ls -la`, shannon runs:

```
<shell> -c '<wrapper script containing ls -la>'
```

The subprocess inherits stdio directly — you see output in real time, and
interactive commands like `vim` or `htop` work normally.

After the subprocess exits, shannon reads the captured state and uses it for
the next command.

## Wrapper Templates

Shannon doesn't run your command directly. It wraps it in a script that
captures state after execution. Each shell has its own wrapper template
defined in the built-in defaults or in `config.toml`. Templates use
`{{placeholder}}` syntax:

| Placeholder     | Replaced with                                    |
| --------------- | ------------------------------------------------ |
| `{{command}}`   | The user's command                               |
| `{{temp_path}}` | Path to the temp file for env capture            |
| `{{init}}`      | Contents of the per-shell init script (or empty) |

### Bash Wrapper (built-in default)

```bash
{{init}}
{{command}}
__shannon_ec=$?
(export -p; echo "__SHANNON_CWD=$(pwd)"; echo "__SHANNON_EXIT=$__shannon_ec") > '{{temp_path}}'
exit $__shannon_ec
```

### Fish/Zsh Wrapper (generic POSIX pattern)

```
{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
```

Most POSIX shells can use this pattern. The `env` command outputs `KEY=VALUE`
lines, which shannon's generic `env` parser reads.

### Nushell Wrapper (special case)

Nushell has unique syntax and captures env as JSON via `$env | to json`. See
the built-in default in `src/config.rs`.

## State Capture and Parsing

After the subprocess exits, shannon reads the temp file using the parser
specified for that shell:

- **`bash` parser:** reads `declare -x KEY="VALUE"` lines, unescaping quotes
  and backslashes.
- **`nushell` parser:** reads JSON. List values (like PATH) are joined with
  `:`. Non-string values are dropped.
- **`env` parser:** reads `KEY=VALUE` lines from the `env` command. Used by
  fish, zsh, and any POSIX shell.

Special markers (`__SHANNON_CWD`, `__SHANNON_EXIT`) are extracted and removed
from the env map. The resulting state — env vars, cwd, exit code — is stored
and injected into the next subprocess.

If parsing fails, the previous state is preserved. Shannon doesn't crash on a
bad parse — it degrades gracefully.

## Configuration

Shannon has two configuration files with distinct purposes:

- **`env.sh`** — a bash script that runs once at startup to set up PATH, env
  vars, and API keys. This is for importing your existing environment.
- **`config.toml`** — static settings for shannon itself: default shell, custom
  shell definitions, wrapper templates, init scripts.

Neither file is required. Without them, shannon uses built-in defaults.

## The Strings-Only Boundary

Only three things cross the shell boundary:

1. **Environment variables** — always strings
2. **Working directory** — a path
3. **Exit code** — an integer

This is deliberate. Bash arrays, nushell tables, shell functions, and aliases
are internal to their shell. Trying to translate them between shells would be
fragile and lossy. The strings-only policy keeps the boundary clean and
predictable.

## Why Not Persistent Sessions?

A persistent shell session (keeping bash running in the background) would avoid
the wrapper script overhead. But it creates problems:

- **State capture is harder** — you'd need to query the running shell for its
  env after every command, which is fragile.
- **Shell switching is complex** — you'd need to manage multiple background
  processes and multiplex stdio.
- **Interactive commands** — programs like `vim` need direct terminal access,
  which is harder to provide through a multiplexer.

The subprocess-per-command model is simpler and more reliable. The overhead of
spawning a process is negligible for interactive use.

## Line Editor

Shannon uses [reedline](https://github.com/nushell/reedline) as its line
editor. Reedline provides:

- Emacs keybindings
- SQLite-backed history with cross-instance sharing
- Autosuggestions (ghost text from history)
- Syntax highlighting via the `Highlighter` trait
- Tab completion via the `Completer` trait and menus
- Bracketed paste mode

Shannon implements custom `Highlighter` and `Completer` traits backed by
tree-sitter and fish completion files, respectively. The Shift+Tab shell
switch is a custom keybinding that triggers a host command.
