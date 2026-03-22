+++
status = "open"
opened = "2026-03-22"
+++

# Issue 15: Remove nushell wrapper, use embedded-only

## Goal

Remove all external nushell (`nu` binary) invocations. Nushell is now
embedded via crates — the system `nu` binary should never be called.
The embedded version is always available, regardless of whether `nu` is
installed.

## Background

Issue 14 added nushell as a library via `eval_source()`. But the old wrapper
code still exists: nushell is listed in `builtin_shells()` with a binary,
wrapper, and parser. The `shell_available("nu")` check still looks for the
system binary. This creates two problems:

1. The system `nu` may be a different version than the embedded 0.111.0,
   causing confusing behavior.
2. If `nu` isn't installed, nushell doesn't appear in the rotation — even
   though it's embedded and always works.

## What changes

- Remove nushell from `builtin_shells()` in `config.rs` — no wrapper needed
- Nushell is always available (embedded) — skip `shell_available()` for it
- In `main.rs`, always include "nu" in the shell list
- Nushell still appears in `toggle` lists — users can exclude it if they want
- Remove the nushell wrapper template and nushell parser from `executor.rs`
- Integration tests for nushell should use `NushellEngine` directly, not
  `execute_command` with a `ShellConfig`

## Experiments

### Experiment 1: Remove nushell wrapper code

#### Description

Remove nushell from the wrapper system. Nushell is always embedded and
always available. No external `nu` binary is ever invoked.

#### Changes

**`src/config.rs`**:

- Remove nushell entry from `builtin_shells()`. Only bash, fish, zsh remain.
- Remove `NUSHELL_WRAPPER` const.
- Keep the nushell parser in `executor.rs` (it's still used by the wrapper
  fallback config — users who override `[shells.nu]` in config.toml could
  still use it). Actually no — if someone overrides `[shells.nu]` they'd be
  calling the system binary, which we want to avoid. Remove the nushell
  parser too.

Wait — we should keep the nushell parser. A user might define a custom shell
that uses JSON env output (nushell-style). The parser is generic enough.
Keep `parse_nushell_env` in executor.rs but remove the nushell wrapper.

**`src/main.rs`**:

- Always create `NushellEngine` (no conditional on "nu" in shell list)
- After filtering shells by `shell_available()`, insert "nu" into the list
  (it's always available since it's embedded)
- Position "nu" according to the toggle list or default order

Actually simpler: the shell list comes from `config.shells()`. We add a
special "nu" entry that has no binary (or binary = "embedded") and is
always included. Then `shell_available()` skips it.

Simplest approach:
1. `config.shells()` still returns "nu" (hardcoded, not from builtin_shells)
2. `main.rs` filters by `shell_available()` but skips "nu" (always available)
3. `repl.rs` already handles "nu" specially in `run_command()`

**`src/config.rs`** — add "nu" to the shell list separately:

In `shells()`, after building the list from builtins + user config, always
ensure "nu" is present with a minimal ShellConfig (highlighter only — no
binary, wrapper, or parser needed since it uses the engine).

**`src/main.rs`** — skip `shell_available` for "nu":

```rust
let shells = all_shells
    .into_iter()
    .filter(|(name, cfg)| name == "nu" || shell_available(&cfg.binary))
    .collect::<Vec<_>>();
```

**`tests/integration.rs`**:

- Remove nushell tests that use `execute_command` with `nushell_config()`
- Those tests are now redundant — nushell is tested via `NushellEngine`
- Keep cross-shell tests (bash→nushell) but route through the engine

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Shannon starts with nushell available even if `nu` binary is not installed.
4. Nushell mode works (pwd, ls, vim, env).
5. `toggle = ["bash"]` excludes nushell from rotation.
6. Bash/fish/zsh still work via wrappers.
