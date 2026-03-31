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
