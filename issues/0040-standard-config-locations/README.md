+++
status = "open"
opened = "2026-04-01"
+++

# Issue 40: Use standard nushell and bash config locations

## Goal

Eliminate Shannon's custom config directory (`~/.config/shannon/`) for shell
configuration. Use nushell's default config location (`~/.config/nushell/`) for
nushell config and bash's default config files (`~/.bash_profile`/`.bashrc`) for
bash setup. No Shannon-specific config file is needed at this time — there are no
Shannon-specific settings that aren't covered by nushell's `config.nu`.

## Background

### Current setup

Shannon maintains its own config directory at `~/.config/shannon/` containing:

- `env.sh` — bash script for PATH, env vars, API keys (sourced at startup)
- `env.nu` — nushell env setup
- `config.nu` — nushell config (keybindings, colors, hooks, etc.)
- `login.nu` — login shell config
- `history.sqlite3` — SQLite command history

This requires users to copy or symlink their existing nushell and bash
configuration into Shannon's directory. It also requires forking `nu-path` to
change the config dir from `"nushell"` to `"shannon"` — a 1-line change that
cascades into a publishing nightmare because `nu-protocol` (and most of the
nushell crate tree) depends on `nu-path`.

### The problem

1. **Friction** — anyone using Shannon already has nushell and bash configured.
   Making them duplicate config into `~/.config/shannon/` is unnecessary work.
2. **Fork tax** — the nu-path fork exists solely to change the config dir name.
   Since `nu-protocol` depends on `nu-path`, this fork can't be published
   independently to crates.io without either publishing the entire nushell crate
   tree or creating conflicting crate versions in the dependency graph.
3. **env.sh is redundant** — Shannon's `env.sh` exists because nushell can't
   source `.bashrc`. But Shannon has a persistent bash subprocess. If that
   subprocess initializes like a normal login shell (sourcing `.bash_profile` →
   `.bashrc`), users get their bash setup for free.

### The solution

Use each tool's standard config location:

- **Nushell config**: `~/.config/nushell/` (stock `nu_config_dir()`, no fork)
- **Bash config**: `~/.bash_profile` (standard login shell init; conventionally
  sources `.bashrc`)
- **Shannon-specific**: nothing needed currently

### What this eliminates

- The `nu-path` fork (revert `"shannon"` → `"nushell"` in `helpers.rs`)
- Shannon's `env.sh` sourcing logic (`executor.rs`)
- Shannon's `config_dir()` and `history_db()` helpers in `shell.rs`
- Custom config file creation/scaffolding in Shannon's startup code

### Bash initialization

The persistent bash subprocess currently starts with `bash --norc --noprofile`,
then manually sources `env.sh`. Replace this with `bash --login`, which follows
the standard bash startup sequence:

1. `/etc/profile`
2. First found of: `~/.bash_profile`, `~/.bash_login`, `~/.profile`
3. `~/.bash_profile` conventionally sources `~/.bashrc`

This means users' existing bash setup (nvm, homebrew, cargo, pyenv, etc.) works
automatically. No `env.sh` needed.

Interactive-only config in `.bashrc` (PS1, aliases, `bind`, `shopt`) is harmless
— Shannon's bash subprocess is headless (stdin/stdout pipes with sentinel
protocol). Nushell/reedline owns the terminal. The only concern is `.bashrc`
scripts that print output unconditionally, which could interfere with sentinel
parsing. This is an edge case we can handle if it arises.

### Env propagation at startup

Currently, Shannon sources `env.sh` in the bash subprocess, captures the
resulting env vars, and injects them into nushell's Stack before `env.nu` runs.
With `bash --login`, the bash subprocess self-initializes. We still need to
capture its env vars and inject them into nushell — the mechanism is the same,
just the trigger changes from "source env.sh" to "bash already initialized
itself."

### What users gain

- **Zero config migration** — existing nushell and bash users change nothing
- **Standard instructions work** — "add this to your .bashrc" just works
- **Shared nushell config** — if you run standalone nushell too, same config
- **Simpler mental model** — nushell config is nushell, bash config is bash

### What changes for existing Shannon users

- `~/.config/shannon/env.sh` → move contents to `~/.bash_profile` or `.bashrc`
- `~/.config/shannon/env.nu` → move to `~/.config/nushell/env.nu`
- `~/.config/shannon/config.nu` → move to `~/.config/nushell/config.nu`
- `~/.config/shannon/history.sqlite3` → moves to
  `~/.config/nushell/history.sqlite3` (handled by stock nushell)

### Impact on crates.io publishing

This eliminates the nu-path fork entirely. The minimum fork set drops from 2
crates (`nu-cli` + `nu-path`) to 1 crate (`nu-cli` only). Since nothing else in
the nushell tree depends on `nu-cli`, it can be published as `shannon-nu-cli`
without conflicts.
