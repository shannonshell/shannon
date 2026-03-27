# Architecture

This document explains how shannon works under the hood.

## Shannon IS Nushell

Shannon copies the nushell binary source code (~4,600 lines) and adds mode
dispatch for brush (bash) and AI. This gives shannon all nushell features for
free: terminal ownership, process groups, job control, signal handling,
multiline editing, completions, hooks, plugins, and more.

## Mode Dispatch

Shannon has three modes, cycled via Shift+Tab:

- **nu** — nushell's native evaluation (default)
- **brush** — bash commands via the brush crate
- **ai** — AI chat via an LLM

The mode is stored in `$env.SHANNON_MODE`. When the mode is "nu", commands
go through nushell's parser and evaluator as normal. When the mode is "brush"
or "ai", a `ModeDispatcher` trait intercepts the command in
`loop_iteration()` and routes it to the appropriate engine.

### ModeDispatcher Trait

Defined in nu-cli (our nushell fork):

```rust
pub trait ModeDispatcher: Send {
    fn execute(
        &mut self,
        mode: &str,
        command: &str,
        env_vars: HashMap<String, String>,
        cwd: PathBuf,
    ) -> ModeResult;
}
```

The dispatcher receives string env vars (converted from nushell's typed
values via `env_to_strings()`) and returns strings. Nushell's REPL writes
them back to the Stack. The dispatcher never touches nushell internals.

## Forked Dependencies

Shannon depends on three forked repos, maintained as git submodules:

| Submodule | Fork of | Changes |
|-----------|---------|---------|
| `nushell/` | nushell/nushell | ModeDispatcher trait, BashHighlighter, Shift+Tab keybinding, config dir, relaxed libc pin, crate renames |
| `brush/` | reubeno/brush | Crate renames only |
| `reedline/` | nushell/reedline | Crate rename only |

All forked crates are renamed to `shannon-*` and published to crates.io.
Each fork has a `shannon` branch with our changes. Upstream sync is done via
`git rebase upstream/main`.

## Environment Propagation

When switching modes, all exported environment variables and the cwd are
preserved:

**Nu to Brush:**
1. `env_to_strings()` converts nushell's typed values to strings
2. `ENV_CONVERSIONS` `to_string` closures handle typed vars (PATH as list)
3. Strings passed to `BrushEngine::inject_state()`

**Brush to Nu:**
1. `BrushEngine::execute()` returns string env vars
2. Strings written to nushell's Stack via `add_env_var(Value::string(...))`
3. Nushell's REPL automatically applies `from_string` conversions

## Configuration

Shannon uses `~/.config/shannon/` with nushell's native config system:

1. `env.sh` — bash environment setup via brush (runs first)
2. `env.nu` — nushell env setup
3. `config.nu` — nushell config (keybindings, colors, hooks)

Shannon-specific settings use `$env.SHANNON_CONFIG` as a nushell record.

## Syntax Highlighting

Each mode has its own highlighter, rebuilt every REPL iteration:

- **Nu mode:** `NuHighlighter` (nushell's native highlighter)
- **Brush mode:** `BashHighlighter` (tree-sitter-bash, Tokyo Night colors)
- **AI mode:** `NoOpHighlighter` (plain unstyled text)

## Source Code Layout

**Copied from nushell binary (startup, terminal, signals):**
`main.rs`, `run.rs`, `command.rs`, `command_context.rs`, `config_files.rs`,
`signals.rs`, `terminal.rs`, `logger.rs`, `ide.rs`,
`experimental_options.rs`, `test_bins.rs`

**Shannon-specific (engines and dispatch):**
`dispatcher.rs`, `brush_engine.rs`, `ai_engine.rs`, `shell_engine.rs`,
`shell.rs`, `executor.rs`, `ai/`
