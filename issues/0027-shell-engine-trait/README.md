+++
status = "open"
opened = "2026-03-25"
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
