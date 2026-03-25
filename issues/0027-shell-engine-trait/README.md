+++
status = "closed"
opened = "2026-03-25"
closed = "2026-03-25"
+++

# Issue 27: ShellEngine trait — drop wrappers, support only embedded shells

## Goal

Define a `ShellEngine` trait that formalizes the interface nushell and brush
already implement. Remove the subprocess wrapper model (bash/fish/zsh wrappers,
temp files, env parsers). Shannon supports only nushell and brush as built-in
shells. The trait provides forward compatibility for adding more shells later.

## Background

Shannon originally supported four shells: bash, nushell, fish, zsh. Nushell was
embedded as a library. Bash, fish, and zsh used subprocess wrappers — each
command spawned a fresh process, ran a wrapper script that captured env vars to
a temp file, and shannon parsed the result.

Brush (embedded bash) was added in issue 24. With nushell + brush, the two
primary use cases are covered:

- **Nushell** — modern shell with structured data, used as the primary shell
- **Brush** — bash-compatible, for running bash scripts and following
  documentation/AI instructions that assume bash

The subprocess wrapper model is complex: wrapper templates, three env parsers
(bash/nushell/env), temp file management, SIG_IGN/SIG_DFL signal handling for
child processes, shell detection via PATH. All of this can be removed.

### The trait

Both `NushellEngine` and `BrushEngine` already implement the same informal
interface:

```rust
trait ShellEngine {
    fn new(...) -> Self;
    fn inject_state(&mut self, state: &ShellState);
    fn execute(&mut self, command: &str) -> ShellState;
}
```

Formalizing this as a trait makes the REPL shell-agnostic. It calls trait
methods without knowing what shell is behind them.

### What gets removed

- `src/executor.rs` — subprocess spawning, wrapper templates, env capture
  parsing. The entire file.
- `src/config.rs` — `ShellConfig` fields for `binary`, `wrapper`, `parser`,
  `init`. Shell configs become simpler (just a name + highlighter).
- Wrapper templates (bash, fish, zsh, nushell) in `config.rs`
- Three env parsers (`parse_bash_env`, `parse_nushell_env`, generic `parse_env`)
- Temp file creation and cleanup
- `SIG_IGN`/`SIG_DFL` signal handling for subprocess execution
- `pre_exec` for child signal restoration
- `restore_sigint_handler` after wrapper execution
- `shell_available()` binary detection
- Fish/zsh from the default shell rotation

### What stays

- `NushellEngine` and `BrushEngine` (implement the trait)
- Signal-hook integration for Ctrl+C
- Reedline break_signal for ExternalBreak
- `/ai`, `/switch`, `/help` meta-commands
- Theme, highlighting, completion, history
- `env.sh` startup script

### Forward compatibility

Future shells implement the `ShellEngine` trait. Options:

1. **Built-in** — add another engine like `NushellEngine` or `BrushEngine`
2. **Plugin** (future) — C ABI or dynamic loading, if there's demand

A "wrapper shell" engine could be built later to support external shells behind
the trait, reusing the subprocess model. But this is not needed now.

## Experiments

### Experiment 1: Define ShellEngine trait, remove wrapper model

#### Description

Define the `ShellEngine` trait. Have `NushellEngine` and `BrushEngine` implement
it. Refactor the REPL to use the trait instead of branching on shell names.
Remove the wrapper model and everything it depends on.

This is a large refactor but mostly deletion. The REPL gets simpler because it
no longer has three code paths (nushell, brush, wrapper) — just one trait call.

#### Changes

**`shannon/src/shell_engine.rs`** (new file):

Define the trait:
```rust
pub trait ShellEngine {
    fn inject_state(&mut self, state: &ShellState);
    fn execute(&mut self, command: &str) -> ShellState;
}
```

No `new()` in the trait — construction differs per engine (nushell needs the
interrupt Arc, brush needs a tokio runtime). Engines are created in `main.rs`
and passed to the REPL as `Box<dyn ShellEngine>`.

**`shannon/src/nushell_engine.rs`**:
- Add `impl ShellEngine for NushellEngine`

**`shannon/src/brush_engine.rs`**:
- Add `impl ShellEngine for BrushEngine`

**`shannon/src/repl.rs`**:
- Change shell list from `Vec<(String, ShellConfig)>` to a list of named
  engines: `Vec<(String, Box<dyn ShellEngine>)>` or similar
- Remove `run_command`'s branching on shell name — just call
  `engine.inject_state()` + `engine.execute()`
- Remove `restore_sigint_handler` after wrapper execution (no more wrappers)
- Remove `use crate::executor::execute_command`
- Update `handle_meta_command` — `/switch` works with engine names
- Simplify `build_editor` — no `ShellConfig` needed, just highlighter name

**`shannon/src/config.rs`**:
- Remove `ShellConfig` struct (or simplify to just `highlighter: Option<String>`)
- Remove `builtin_shells()`, wrapper templates, `expand_wrapper()`,
  `read_init_file()`
- Remove `parse_bash_env`, `parse_nushell_env`, `parse_env` parsers
- Default shells become just `["nu", "brush"]`
- `toggle` config still works but only with names of built-in engines

**`shannon/src/executor.rs`**:
- Delete entirely. Keep `run_startup_script()` if it's still used (it runs
  `env.sh` at startup via bash subprocess — this is separate from the wrapper
  model)

**`shannon/src/main.rs`**:
- Create engines, build the shell list as `Vec<(String, Box<dyn ShellEngine>)>`
- Remove `shell_available()` filtering (embedded shells are always available)
- Pass engine list to `repl::run`

**`shannon/src/lib.rs`**:
- Add `pub mod shell_engine;`
- Keep `executor` module only if `run_startup_script` stays

**Tests**:
- Remove wrapper-based integration tests (bash/fish/zsh `execute_command` tests)
- Keep nushell and brush engine tests
- Update config tests for simplified shell list

#### Verification

1. `cargo build` succeeds.
2. `cargo test` — all remaining tests pass.
3. Manual: nushell commands work, state persists.
4. Manual: brush commands work, state persists.
5. Manual: `/switch` between nu and brush, env vars propagate.
6. Manual: Ctrl+C works in both shells.
7. Manual: `/ai` works.
8. `src/executor.rs` is deleted (or reduced to just `run_startup_script`).

**Result:** Pass

All verification steps confirmed. 63 tests pass (53 unit + 10 integration).
Down from 95 — removed 32 wrapper-related tests. Both nushell and brush work,
shell switching propagates env, Ctrl+C works, `/ai` works.

#### Conclusion

The refactor removed significant complexity:
- `ShellConfig` struct (5 fields) → `ShellSlot` (name + highlighter + engine)
- `executor.rs` gutted from ~500 lines to ~120 (just `run_startup_script`)
- `config.rs` lost wrapper templates, parsers, `builtin_shells()`,
  `expand_wrapper()`, `read_init_file()`
- `repl.rs` `run_command()` went from 3 branches + 40 lines to 2 lines
- No more temp files, SIG_IGN/SIG_DFL dance, or shell binary detection

## Conclusion

Shannon now uses a `ShellEngine` trait for all shell execution. Only nushell
and brush are supported as embedded engines. The subprocess wrapper model is
gone. Future shells implement the trait to plug in.
