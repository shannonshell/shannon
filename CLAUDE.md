# olshell

A poly-shell that wraps multiple shell interpreters and lets you switch between
them mid-session using Shift+Tab.

## Build

```sh
cargo build
cargo run
```

## Architecture

olshell uses reedline (from crates.io) as its line editor. Each command spawns a
fresh subprocess — there are no persistent shell sessions.

### Source files

- `src/main.rs` — entry point, reedline loop, Shift+Tab shell switching
- `src/shell.rs` — `ShellKind` enum (Bash/Nushell), `ShellState` (env, cwd, exit code)
- `src/executor.rs` — subprocess spawning, wrapper scripts, env capture parsing
- `src/prompt.rs` — custom reedline `Prompt` impl showing active shell + cwd

### How command execution works

1. User types a command
2. olshell wraps it in a shell-specific script that captures env vars + cwd after execution
3. Subprocess runs with inherited stdio (output streams directly to terminal)
4. After exit, olshell reads captured state from a temp file
5. State (env vars, cwd, exit code) is injected into the next command's subprocess

### Key design decisions

- **Strings only** — only env vars (strings), cwd, and exit code cross the shell boundary. No shell-internal data structures.
- **One subprocess per command** — no persistent shell sessions. Type `bash` or `nu` for a full interactive session.
- **Vendor directory is for reference only** — vendored repos are for reading source code, not for building against. Use crates.io dependencies in Cargo.toml.
- **Nushell output rendering** — nushell's `echo` returns a Value rather than printing. The wrapper uses try/catch + explicit `print` to render output.

## Shells supported

Currently: bash, nushell. The architecture supports any shell — adding one means adding a wrapper script builder and an env parser in `executor.rs`.

## Config

History files are stored in `~/.config/olshell/` (per-shell).
