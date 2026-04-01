+++
status = "open"
opened = "2026-04-01"
+++

# Issue 40: Use standard nushell and bash config locations

## Goal

Eliminate Shannon's custom config directory (`~/.config/shannon/`) for shell
configuration. Use nushell's default config location (`~/.config/nushell/`) for
nushell config and bash's default config files (`~/.bash_profile`/`.bashrc`) for
bash setup. No Shannon-specific config file is needed at this time — there are
no Shannon-specific settings that aren't covered by nushell's `config.nu`.

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

## Experiments

### Experiment 1: Replace env.sh with bash --login, revert nu-path fork

Two changes in one experiment because they're tightly coupled — both eliminate
Shannon's custom config directory.

#### Changes

**`nushell/crates/nu-path/src/helpers.rs`** — Revert config dir name

Change `p.push("shannon")` back to `p.push("nushell")`. This makes
`nu_config_dir()` return `~/.config/nushell/` (stock behavior). Shannon now
shares nushell's config directory.

**`src/bash_process.rs`** — Launch bash as login shell

Change `Command::new("bash").args(["--norc", "--noprofile"])` to
`Command::new("bash").args(["--login"])`. The bash subprocess now self-
initializes via `.bash_profile` → `.bashrc`, loading the user's PATH, nvm,
homebrew, etc. automatically.

**`src/dispatcher.rs`** — Remove env.sh sourcing

Delete the `env.sh` sourcing block in `ShannonDispatcher::new()`. The bash
subprocess already initialized itself. Still call `capture_env()` to get the
bash env vars for injection into nushell.

Simplify to:

```rust
pub fn new() -> Self {
    let bash = BashProcess::new();
    ShannonDispatcher { bash }
}
```

**`src/run.rs`** — Keep env var injection, update comment

The `dispatcher.env_vars()` call stays — it captures bash's post-login env vars
and injects them into nushell's Stack. Update the comment to reflect that these
come from bash's login initialization, not env.sh.

**`src/executor.rs`** — Remove `run_startup_script` and related functions

Delete `run_startup_script()`, `run_startup_script_from()`, and all their tests.
Keep `parse_bash_env()`, `parse_declare_line()`, and `unescape_bash_value()` —
these are still used by `BashProcess` for sentinel parsing.

**`src/shell.rs`** — Remove `config_dir()` and `history_db()`

Delete the `config_dir()` and `history_db()` functions. Keep `ShellState` and
its impl/tests — those are still used everywhere.

**`src/lib.rs`** — No changes needed

The `executor` module is still needed (for `parse_bash_env`). The `shell` module
is still needed (for `ShellState`).

#### Verification

1. `cargo build` — compiles
2. `cargo test --lib` — all tests pass (executor tests removed, remaining pass)
3. `shannon` starts and loads user's nushell config from `~/.config/nushell/`
4. `$nu.config-path` shows `~/.config/nushell/config.nu`
5. Bash mode has env vars from `.bash_profile`/`.bashrc` (e.g., `echo $PATH`
   includes homebrew, cargo, nvm paths)
6. `export FOO=bar` in bash → `echo $env.FOO` in nu → `bar` (env propagation)
7. Shift+Tab mode switching still works
8. No references to `~/.config/shannon` remain in Shannon's source code

**Result:** Pass

All verification steps confirmed. One additional fix was needed: `src/main.rs`
had two `.join("shannon")` references in the XDG_CONFIG_HOME validation block
(lines 144 and 156) that needed reverting to `.join("nushell")`. Without this,
nushell's XDG validation compared `~/.config/nushell` against `~/.config/shannon`
and reported an invalid config error.

Env vars from both config systems confirmed working:
- `$env.SHANNON_NUSHELL` set in `~/.config/nushell/env.nu` → available in nu mode
- `$SHANNON_BASH` set in `.bash_profile` → available in bash mode

#### Conclusion

Shannon no longer maintains its own config directory. Nushell config lives in
`~/.config/nushell/` (stock location), bash config lives in `.bash_profile`/
`.bashrc` (standard login shell). The nu-path fork is eliminated — only the
1-line `"nushell"` string was reverted. The `env.sh` sourcing logic, `config_dir()`,
and `history_db()` helpers were all removed. The bash subprocess now initializes
via `bash --login` instead of `bash --norc --noprofile` + manual env.sh sourcing.
