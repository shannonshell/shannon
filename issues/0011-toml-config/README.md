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
2. **`SHANNON_DEFAULT_SHELL` env var** — set in `config.sh` to pick the
   default shell. This works but conflates shannon settings with environment
   setup.

Additionally, shell support is hardcoded in Rust — adding a new shell requires
modifying `shell.rs`, `executor.rs`, `highlighter.rs`, `main.rs`, and
`prompt.rs`. This means users can't add shells (like zsh, elvish, tcsh) without
recompiling.

### What changes

**Rename `config.sh` → `env.sh`** — clearer name. It's an environment setup
script, not a shannon config file. Shannon should support both names during a
transition period (check `env.sh` first, fall back to `config.sh`).

**Add `config.toml`** — static configuration for shannon itself:

```toml
default_shell = "nu"

[shells.bash]
binary = "bash"
exit_code_var = "$?"

[shells.nu]
binary = "nu"
exit_code_var = "$?"

[shells.fish]
binary = "fish"
exit_code_var = "$status"

[shells.zsh]
binary = "zsh"
exit_code_var = "$?"
```

**Generic shell wrapper** — instead of per-shell wrapper functions
(`build_bash_wrapper`, `build_nushell_wrapper`, `build_fish_wrapper`), use a
single wrapper template that works with any POSIX-like shell:

```
{command}
set __shannon_ec {exit_code_var}
env > '{temp_path}'
echo "__SHANNON_CWD=$(pwd)" >> '{temp_path}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{temp_path}'
exit $__shannon_ec
```

The only per-shell variable is the exit code expression (`$?` vs `$status`).
The `env` command is POSIX and works from any shell. `pwd` is universal.

### Nushell is special

Nushell is the exception — it can't use the generic wrapper because:

- `env` outputs nushell-internal values that aren't strings
- Nushell's command chaining syntax is different (no `;` between commands
  in the same way)
- The current nushell wrapper uses `$env | to json` which is nushell-specific

We keep the nushell wrapper as a special case, either hardcoded or with a
config option to specify a custom wrapper template.

### Shell colors

Currently each shell has a hardcoded prompt color (bash=green, nushell=cyan,
fish=yellow). Options:

1. **Drop per-shell colors** — use a single prompt color for all shells. The
   `[shell_name]` text already identifies which shell is active.
2. **Make colors configurable in TOML** — `color = "green"` per shell.
3. **Auto-assign colors** — cycle through a palette based on shell order.

Option 1 is simplest. Option 2 adds config complexity for minimal value.
Option 3 is a reasonable middle ground. Decision can be made during
implementation.

### Syntax highlighting

Tree-sitter grammars are compile-time dependencies — they can't be added via
config. For shells with no grammar (zsh, elvish, tcsh, etc.), highlighting
falls back to the default foreground color. This is fine — the shell still
works, it just doesn't have colored input.

The three built-in grammars (bash, nushell, fish) cover the most common shells.
Zsh users get no highlighting, but zsh commands are mostly bash-compatible so
the bash grammar could optionally be used as a fallback.

### Config file location

`~/.config/shannon/config.toml` — same directory as `env.sh` and `history.db`.
Respects `XDG_CONFIG_HOME`.

### What TOML replaces

| Before | After |
|--------|-------|
| `SHANNON_DEFAULT_SHELL` env var | `default_shell` in config.toml |
| Hardcoded shell list in `main.rs` | `[shells.*]` tables in config.toml |
| Hardcoded prompt colors in `prompt.rs` | Either dropped or configurable |
| Per-shell wrapper functions in `executor.rs` | Generic wrapper + nushell special case |

### Migration

- `config.sh` continues to work (checked as fallback if `env.sh` doesn't exist)
- `SHANNON_DEFAULT_SHELL` env var continues to work (overridden by config.toml
  if both are set)
- Default config.toml is generated on first run with bash, nushell, and fish
  (matching current behavior)
