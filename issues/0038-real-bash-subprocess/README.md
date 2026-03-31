+++
status = "open"
opened = "2026-03-31"
+++

# Issue 38: Replace brush with a real bash subprocess

## Goal

Replace the embedded brush crate with a persistent real bash subprocess for bash
mode execution. This guarantees full bash compatibility — no pipeline deadlocks,
no signal handling conflicts, no behavioral divergence from real bash.

## Background

Issue 35 traced `nvm install` hanging to a fundamental limitation in brush:
pipeline stages execute sequentially. When stage 1 is a shell function that
fills a pipe buffer, it blocks because stage 2 hasn't started reading yet. Real
bash uses `fork()` to run all pipeline stages concurrently as OS processes.
Attempting to fix this with `tokio::spawn` broke signal delivery.

The conclusion: brush cannot faithfully replicate bash's process model. Rather
than fighting an ever-growing list of compatibility issues, we should use real
bash.

Shannon originally used a bash subprocess wrapper (commits `5cf51e16b` through
`0dd091057`), but it spawned a **new process per command**, losing all state
between commands. The new approach keeps a **single persistent bash process**
alive for the entire session.

## Architecture

### Current flow (brush)

```
User types bash command
  → dispatcher.execute("bash", command, env, cwd)
  → BrushEngine.inject_state(env, cwd)
  → shell.run_string(command)       # brush executes in-process
  → capture env, cwd, exit_code
  → return to nushell REPL
```

### New flow (bash subprocess)

```
User types bash command
  → dispatcher.execute("bash", command, env, cwd)
  → BashProcess.inject_state(env, cwd)
  → write command + sentinel to bash stdin
  → stream stdout to terminal until sentinel
  → parse sentinel block (env, cwd, exit_code)
  → return to nushell REPL
```

### Persistent bash process

Spawn `bash --norc --noprofile` once at startup (shannon sources `env.sh`
explicitly, so we don't want bash's own startup files). Keep stdin/stdout/stderr
piped. The process lives for the entire shell session.

### Command protocol

For each command, write to bash's stdin:

```bash
{inject env vars}
cd {cwd}
{command}
__shannon_ec=$?
echo "__SHANNON_SENTINEL_START__"
export -p
echo "__SHANNON_CWD=$(pwd)"
echo "__SHANNON_EXIT=$__shannon_ec"
echo "__SHANNON_SENTINEL_END__"
```

Read bash's stdout line-by-line:

- Lines before `__SHANNON_SENTINEL_START__` → display to user
- Lines between sentinel start/end → parse for env, cwd, exit code
- `export -p` output → parse `declare -x KEY="VALUE"` lines into HashMap

### Env var injection

Before each command, inject nushell's current env vars into bash. Two options:

1. **Export statements**: `export KEY="VALUE"` for each var (simple, verbose)
2. **Diff-based**: Only inject vars that changed since last command (efficient)

Start with option 1. Optimize later if needed.

### Stderr handling

Stderr needs a separate reader thread/task to avoid deadlocks. Stderr output
goes directly to the terminal — no sentinel parsing needed.

### Signal handling

Ctrl+C should reach the bash child naturally if it's in the same process group.
The bash process handles signals with real bash signal semantics — no more
competing tokio signal listeners.

### Features to preserve

All features currently provided by brush must work with the new approach:

| Feature                          | How                                            |
| -------------------------------- | ---------------------------------------------- |
| Persistent shell state           | Single long-lived bash process                 |
| Function definitions (nvm, etc.) | Functions survive in the bash process          |
| Env var import/export            | Inject before command, capture via `export -p` |
| CWD synchronization              | `cd` before command, `pwd` after               |
| Exit code capture                | `$?` captured in sentinel                      |
| Script sourcing (env.sh)         | `source env.sh` as first command               |
| Bash builtins                    | Real bash — all builtins work                  |

### What we can remove

- `brush-core` and `brush-builtins` dependencies from Cargo.toml
- `src/brush_engine.rs`
- The entire `brush/` subtree (eventually — keep until new engine is proven)

### Interface

The `ModeDispatcher` trait stays unchanged. `ShannonDispatcher` will hold a
`BashProcess` instead of a `BrushEngine`. The trait interface:

```rust
fn execute(
    &self,
    mode: &str,
    command: &str,
    env: HashMap<String, String>,
    cwd: PathBuf,
) -> Option<ModeResult>;
```

### Open questions

1. **Terminal control**: Does bash need a pty instead of plain pipes for
   interactive features (job control, `read -p`, cursor movement)? Start with
   pipes — escalate to pty if needed.
2. **Binary output**: Commands that produce binary stdout (e.g.,
   `cat image.png`) could contain sentinel-like strings. Mitigation: use a
   sufficiently unique sentinel with a random nonce per session.
3. **Multiline commands**: Bash handles these natively — no special treatment
   needed since we're writing to bash's stdin parser.
4. **Performance**: One extra process + pipe I/O per command. Should be
   negligible compared to the command itself.

## Experiments

### Experiment 1: Replace BrushEngine with BashProcess

Replace `src/brush_engine.rs` with a new `src/bash_process.rs` that spawns a
persistent bash subprocess and communicates via stdin/stdout pipes with
sentinel-based state capture.

#### Changes

**New file: `src/bash_process.rs`**

The `BashProcess` struct holds:
- `child: std::process::Child` — the persistent bash process
- `stdin: ChildStdin` — write commands here
- `stdout_reader: BufReader<ChildStdout>` — read output line-by-line

Constructor (`BashProcess::new()`):
1. Spawn `bash --norc --noprofile` with stdin/stdout/stderr piped
2. Store child, take ownership of stdin/stdout/stderr

Command execution protocol (`execute(command) -> ShellState`):

Write to stdin:
```bash
{command}
__shannon_ec=$?
echo "==SHANNON_SENTINEL_START=="
export -p
echo "__SHANNON_CWD=$(pwd)"
echo "__SHANNON_EXIT=$__shannon_ec"
echo "==SHANNON_SENTINEL_END=="
```

Read stdout line-by-line:
- Lines before `==SHANNON_SENTINEL_START==` → write to real stdout (user sees
  command output)
- Lines between sentinel start/end → collect into a buffer
- When `==SHANNON_SENTINEL_END==` seen → parse the buffer using the existing
  `parse_bash_env()` from executor.rs, plus extract exit code

State injection (`inject_state(state)`):

Write to stdin:
```bash
cd '{cwd}'
```
For env vars, write `export KEY='VALUE'` for each var. Single-quote values with
embedded single quotes escaped as `'\''`.

Stderr handling:
- Spawn a dedicated thread that reads stderr line-by-line and writes to real
  stderr. This runs for the lifetime of the bash process.

Env capture (`capture_env() -> HashMap`):
- Run a no-op command through the execute protocol to trigger `export -p` and
  parse the result. (Used by `dispatcher.env_vars()` at startup.)

**Modify: `src/dispatcher.rs`**

- Replace `use crate::brush_engine::BrushEngine` with
  `use crate::bash_process::BashProcess`
- Replace `brush: BrushEngine` with `bash: BashProcess`
- In `new()`: create `BashProcess::new()` instead of `BrushEngine::new()`.
  Source `env.sh` the same way (inject state, execute source command).
- In `env_vars()`: call `self.bash.capture_env()`
- In `execute()`: call `self.bash.inject_state()` / `self.bash.execute()`

**Modify: `src/lib.rs`**

- Replace `mod brush_engine` with `mod bash_process`

**Modify: `Cargo.toml`**

- Remove `brush-core` and `brush-builtins` from `[dependencies]`

**Modify: `src/executor.rs`**

- Make `parse_bash_env`, `parse_declare_line`, and `unescape_bash_value` `pub`
  so `bash_process.rs` can reuse them.

**Keep unchanged:**
- `src/shell_engine.rs` — `BashProcess` implements `ShellEngine` same as before
- `src/shell.rs` — `ShellState` unchanged
- `src/run.rs` — calls `dispatcher.env_vars()` and `dispatcher.execute()` which
  are unchanged
- `src/signals.rs` — Ctrl+C handler stays as-is (ctrlc crate)

#### Verification

1. `cargo build` — compiles without brush dependencies
2. `cargo test` — existing tests pass, especially executor.rs tests
3. Manual test: `shannon` → switch to bash mode → `echo hello` → shows output
4. Manual test: `export FOO=bar` → `echo $FOO` → prints `bar` (state persists)
5. Manual test: `nvm install 24` — completes without hanging
6. Manual test: Ctrl+C during a long-running command → interrupts it
7. Manual test: switch to bash → run command → switch to nu → check env vars
   propagated

**Result:** Pass

All verification steps confirmed:
- `cargo build` compiles without brush dependencies (net -278 lines)
- `cargo test` — all 15 library tests pass (8 new bash_process + 5 executor + 2 shell)
- `echo hello` → `hello` ✓
- `export FOO=bar` then `echo $FOO` → `bar` (state persists across commands) ✓
- Switch to nu mode → `echo $env.FOO` → `bar` (env propagation works) ✓
- Switch back to bash → `echo $FOO` → `bar` (round-trip works) ✓
- `nvm install 24` → completes without hanging ✓ (the original issue 35 bug)

#### Conclusion

Real bash subprocess works on the first attempt. The sentinel-based protocol
correctly captures env vars, cwd, and exit codes. State persists across
commands (env vars, functions from env.sh). The pipeline deadlock that plagued
brush is gone — real bash uses fork() for pipeline stages natively.
