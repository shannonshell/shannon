+++
status = "open"
opened = "2026-03-30"
+++

# Issue 35: Signal handler conflict between nushell and brush

## Goal

Fix commands in bash mode that spawn child processes (like `nvm install`)
hanging until Ctrl+C is pressed. The root cause is competing SIGINT handlers
between nushell's `ctrlc` crate and brush's `tokio::signal`.

## Background

### Symptom

`nvm install 24` in bash mode hangs indefinitely. Pressing Ctrl+C unsticks it
and the command completes successfully. `nvm use 24` (which doesn't spawn long
subprocesses) works fine.

### Root cause

Two SIGINT handlers are registered in the same process:

1. **Nushell** — `ctrlc::set_handler()` in `signals.rs` sets an `AtomicBool` and
   runs nushell's signal handlers
2. **Brush** — `tokio::signal::ctrl_c()` in `processes.rs` waits for SIGINT
   inside a `tokio::select!` loop

When a child process (curl, spawned by nvm) is running:

- Brush waits via `tokio::select!` with `await_ctrl_c()` + `exec_future`
- The `exec_future` blocks until curl completes
- Ctrl+C fires nushell's ctrlc handler (sets the AtomicBool) but doesn't notify
  tokio's signal system
- Brush's `await_ctrl_c()` never fires because the signal was consumed by
  nushell's handler
- The child process doesn't receive SIGINT either

Pressing Ctrl+C disrupts the signal state enough for the process to continue.

### Options investigated

1. **Disable ctrlc handler during brush execution** — not viable, `ctrlc` crate
   has no unregister API
2. **Use tokio signal handling exclusively** — viable but requires making
   nushell async-aware, too large a change
3. **Propagate ctrlc to tokio** — not viable, `ctrlc` crate doesn't expose
   signal-hook registration handles

### Chosen approach

Replace `ctrlc` with `signal-hook` directly in Shannon's signal setup.
`signal-hook::register()` returns a registration ID that can be used with
`signal-hook::unregister()`. Before calling `dispatcher.execute()`, temporarily
unregister nushell's SIGINT handler so brush's tokio signal handlers work
uncontested. Re-register after brush returns.

### Implementation sketch

**`shannon/src/signals.rs`:**

- Replace `ctrlc::set_handler()` with `signal_hook_registry::register()`
- Return the registration `SigId` so it can be saved
- Store the `SigId` in `EngineState` or pass it through to the REPL

**`nushell/crates/nu-cli/src/repl.rs` (mode dispatch block):**

```rust
if mode != "nu" {
    // Temporarily unregister nushell's SIGINT handler
    signal_hook_registry::unregister(sigint_id);

    let result = dispatcher.execute(...);

    // Re-register after brush finishes
    signal_hook_registry::register(SIGINT, handler);
}
```

**`shannon/Cargo.toml`:**

- Add `signal-hook` dependency
- Remove `ctrlc` dependency (if no longer needed)

### Complications

- The `ctrlc` crate is also used by nushell's upstream code. Removing it may
  require changes to how nushell's signal handlers are registered.
- `signal-hook` registration IDs need to be threaded through to the REPL
  dispatch code. May need to store on `EngineState` or pass via `LoopContext`.
- Need to ensure the handler is always re-registered, even on panic (use a
  guard/drop pattern).

## Experiments

### Experiment 1: Replace ctrlc with signal-hook, unregister during brush

#### Description

Replace the `ctrlc` crate with `signal-hook` for SIGINT handling. Store the
registration `SigId` so it can be unregistered before brush execution and
re-registered after.

#### Changes

**`src/signals.rs`:**

- Replace `ctrlc::set_handler(closure)` with
  `signal_hook_registry::register(SIGINT, closure)`
- Return the `SigId` from the function
- Change function signature to return the SigId

**`src/main.rs`:**

- Capture the `SigId` returned from `ctrlc_protection()`
- Store it somewhere accessible to the REPL — either on `EngineState` (requires
  adding a field) or as a separate value passed through

**`src/run.rs`:**

- Pass the `SigId` to the dispatcher so it can unregister/re-register

**`src/dispatcher.rs`:**

- Accept the `SigId` in `ShannonDispatcher::new()` or `execute()`
- Before calling `brush.execute()`, call
  `signal_hook_registry::unregister(sigid)`
- After brush returns, re-register with
  `signal_hook_registry::register(SIGINT, closure)`
- Use a drop guard to ensure re-registration on panic

**`nushell/crates/nu-cli/src/repl.rs`:**

- The `ModeDispatcher::execute()` trait doesn't need to change — the
  unregister/re-register happens inside `ShannonDispatcher::execute()`, not in
  the REPL

**`Cargo.toml`:**

- Add `signal-hook` dependency
- Keep `ctrlc` (nushell's upstream code may still reference it)

**Actually, simpler approach:** Do the unregister/re-register inside
`ShannonDispatcher::execute()` itself, not in the REPL. The dispatcher owns the
SigId and the handler closure. This avoids threading anything through
LoopContext or EngineState.

```rust
impl ShannonDispatcher {
    pub fn new(sigid: SigId, handler: Arc<dyn Fn() + Send + Sync>) -> Self {
        ShannonDispatcher { brush, sigid, handler }
    }
}

impl ModeDispatcher for ShannonDispatcher {
    fn execute(&mut self, mode, command, env, cwd) -> ModeResult {
        // Unregister nushell's SIGINT handler
        signal_hook_registry::unregister(self.sigid);

        let result = /* brush execute */;

        // Re-register
        self.sigid = signal_hook_registry::register(
            libc::SIGINT, self.handler.clone()
        ).unwrap();

        result
    }
}
```

#### Verification

1. `cargo build` succeeds.
2. `nvm use 24` still works (no regression).
3. `nvm install 22` completes without hanging.
4. Ctrl+C during a long bash command kills the process (not ignored).
5. Ctrl+C in nushell mode still works (handler re-registered after brush).
6. `cargo test` passes.

**Result:** Fail

`nvm install 24` still hangs until Ctrl+C is pressed. Unregistering nushell's
signal-hook handler did not fix the issue. The `ctrlc` crate's handler is STILL
registered — we added a signal-hook handler but never removed the ctrlc one. The
`ctrlc` crate registers its own handler at startup (internally, via nushell's
upstream code or our own call), and that handler persists regardless of what we
do with signal-hook. Two handlers are now registered: the original ctrlc one and
our new signal-hook one.

The root cause may not be what we assumed. Possibilities:

1. The `ctrlc` crate handler is still active and consuming SIGINT
2. The issue isn't about handler competition but about how brush's tokio runtime
   handles signals inside `block_on()`
3. The child process isn't in the right process group to receive SIGINT

#### Conclusion

Simply adding signal-hook alongside ctrlc doesn't fix the issue. We need to
either fully remove ctrlc (but nushell's internal code depends on it) or
investigate whether the root cause is something else entirely.

### Experiment 2: Add debug logs to trace the hang

#### Description

We don't know where exactly the hang occurs. Possible locations:

1. `processes.rs` `wait()` — the `tokio::select!` loop never fires
2. The shell function execution path — `nvm install` doesn't go through
   `ChildProcess::wait()` at all
3. `block_on()` itself — the tokio runtime isn't delivering signals

Add `eprintln!` debug logs at key points to trace execution flow when
`nvm install 24` is run. No code changes — just temporary logging.

#### Changes

**`src/dispatcher.rs`** — log entry/exit of brush execution:

```rust
eprintln!("[shannon] entering brush execute: {command}");
// ... brush.execute(command) ...
eprintln!("[shannon] brush execute complete");
```

**`brush/brush-core/src/processes.rs`** — log inside the `wait()` loop:

```rust
eprintln!("[brush:processes] entering wait loop");
// Inside tokio::select!:
//   exec_future branch: eprintln!("[brush:processes] exec_future completed");
//   sigtstp branch: eprintln!("[brush:processes] SIGTSTP received");
//   sigchld branch: eprintln!("[brush:processes] SIGCHLD received");
//   ctrl_c branch: eprintln!("[brush:processes] SIGINT/ctrl_c received");
```

**`brush/brush-core/src/commands.rs`** — log command type detection:

```rust
// Where commands are dispatched (function vs external vs builtin)
eprintln!("[brush:commands] executing: {name} as {type}");
```

**`brush/brush-core/src/sys/unix/signal.rs`** — log signal listener creation:

```rust
eprintln!("[brush:signal] creating SIGTSTP listener");
eprintln!("[brush:signal] creating SIGCHLD listener");
eprintln!("[brush:signal] creating ctrl_c listener");
```

#### Verification

1. `cargo build` succeeds
2. Run `nvm install 24` in bash mode
3. Observe debug output to determine:
   - Does execution reach `processes.rs` `wait()`?
   - Does the `tokio::select!` loop start?
   - Which branch (if any) fires when Ctrl+C is pressed?
   - Is the hang before, during, or after the `wait()` loop?
4. Report findings and design next experiment based on results

**Result:** Pass (diagnostic success)

The logs revealed the hang is NOT a signal handler conflict. Key findings:

1. `processes.rs wait()` IS reached — many child processes start and complete
   normally during `nvm install`.
2. The `tokio::select!` loop works correctly — `exec_future completed` and
   `SIGCHLD received` fire as expected for most processes.
3. The hang occurs on the LAST child process nvm spawns — `entering select
   loop` appears without a corresponding `exec_future completed`.
4. `SIGINT/ctrl_c received` only fires when the user presses Ctrl+C to
   unstick it — confirming the child process is blocked, not the signal
   system.
5. After Ctrl+C: SIGCHLD fires (child dies), exec_future completes,
   `brush execute complete` follows — everything unblocks normally.

**The root cause is NOT competing signal handlers.** The last child process
spawned by nvm is blocked (probably waiting on stdin or a missing signal).
The signal-hook changes from experiment 1 were unnecessary.

#### Conclusion

The hang is caused by a specific child process that nvm spawns at the end
of its install sequence. This process blocks indefinitely until killed.
Next: identify which command is hanging by logging the command name when
`ChildProcess` is created.

### Experiment 3: Log which command hangs

#### Description

Add a debug log in `brush/brush-core/src/commands.rs` at the
`execute_external_command()` function (~line 585) where the command name
and args are available just before spawning. This will identify the exact
command that hangs at the end of `nvm install`.

Brush already has `tracing::debug!` at this location — add our
`debug_log()` call next to it.

#### Changes

**`brush/brush-core/src/commands.rs`** (~line 585):
- Add `debug_log()` call logging the command program and arguments
  right before `sys::process::spawn(cmd)`

#### Verification

1. `cargo build` succeeds
2. `rm -f /tmp/shannon-debug.log`
3. `tail -f /tmp/shannon-debug.log` in another terminal
4. Run `nvm install 24` in bash mode
5. The last command logged before the hang is the culprit

**Result:** Pass (diagnostic success)

The command logs combined with pipeline stage logs revealed the true root
cause. Key findings:

1. The hanging command is `/usr/bin/curl -q --fail --compressed -L -s
   https://nodejs.org/dist/index.tab -o -` — a curl downloading a file.

2. This curl runs as **pipeline stage 1/1** — a standalone command, NOT part
   of a multi-command pipeline. This means it's inside a **command
   substitution** like `$(curl ...)` in nvm's bash script.

3. The pipeline logs confirm: the curl `stage 1/1 returned: StartedProcess`,
   then `entering wait()` / `entering select loop` with no further progress
   until Ctrl+C.

4. **Root cause identified: held pipe writer in command substitution.**
   Brush's `invoke_command_in_subshell_and_get_output()` in `commands.rs`
   (lines 727-763) creates a pipe, moves the writer into `params`, spawns
   the command via `tokio::spawn(run_substitution_command(subshell, params, s))`,
   then reads from the pipe with `async_reader.read_to_string().await`.

   The problem: the pipe writer is held inside `params` which lives in the
   spawned task. When curl finishes writing and closes its stdout fd, the
   reader still doesn't see EOF because the writer copy in `params` is
   still alive. `read_to_string()` blocks forever waiting for EOF.

   This is a classic **held pipe writer** bug — the reader blocks because
   not all writers have been dropped.

5. **This is NOT a signal handling conflict.** The original issue title is
   wrong. The root cause is a brush bug in command substitution pipe
   management. The signal-hook changes from experiment 1 were unnecessary.

#### Conclusion

The hang is caused by a held pipe writer in brush's command substitution
code. When `$(curl ...)` runs, brush creates a pipe, passes the writer to
the child process, but also keeps a copy in the execution parameters struct.
After curl finishes, the reader never sees EOF because the extra writer is
still alive. Fix: ensure the writer is dropped from params after the child
process is spawned. This is a brush-core bug that we can fix directly in
our monorepo.

### Experiment 4: Log command substitution internals

#### Description

Experiment 3 found that `curl` hangs inside a command substitution `$(curl ...)`.
The code path is `invoke_command_in_subshell_and_get_output()` (commands.rs:727),
which creates a pipe, spawns a tokio task running `run_substitution_command`,
and concurrently reads from the pipe via `read_to_string().await`.

The theory is a held pipe writer: the writer lives in `params` inside the
spawned task, and doesn't drop until `run_substitution_command` returns. If
`run_parsed_result` blocks waiting for the child, and the reader blocks waiting
for EOF, we have a deadlock.

But `run_substitution_command` owns `params` — when it returns, `params` drops,
the writer closes, and the reader should see EOF. So either:
- `run_parsed_result` never returns (something inside pipeline execution hangs)
- `run_parsed_result` returns but something else holds the writer open

Add debug logs to determine which case we're in.

#### Changes

**`brush/brush-core/src/commands.rs`** — 5 debug_log calls:

In `invoke_command_in_subshell_and_get_output` (line ~727):
```rust
// after line 747 (pipe creation):
debug_log("[brush:subst] pipe created, spawning task");
// after line 751 (tokio::spawn):
debug_log("[brush:subst] task spawned, reading output...");
// after line 753 (read_to_string):
debug_log("[brush:subst] read_to_string completed");
```

In `run_substitution_command` (line ~765):
```rust
// at function entry:
debug_log("[brush:subst] entering run_substitution_command");
// after run_parsed_result returns:
debug_log("[brush:subst] run_parsed_result returned");
```

#### Verification

1. `cargo build` succeeds
2. `rm -f /tmp/shannon-debug.log && tail -f /tmp/shannon-debug.log`
3. Run `nvm install 24` in bash mode
4. Check the log for the hanging substitution:
   - If `run_parsed_result returned` appears but NOT `read_to_string completed`:
     pipe writer theory confirmed — something else holds the writer open
   - If `run_parsed_result returned` does NOT appear:
     `run_parsed_result` itself hangs — issue is inside pipeline execution

**Result:** Pass (diagnostic success)

All command substitutions completed normally — every `pipe created` had a
matching `read_to_string completed`. The held pipe writer theory was wrong
for command substitutions.

The actual hang: curl spawned as a direct pipeline stage (NOT inside a
command substitution). At line 6024 in the log: `spawning: /usr/bin/curl -q
--fail --compressed -L -s https://nodejs.org/dist/index.tab -o -`. Then
`entering wait()` / `entering select loop` with no completion until Ctrl+C
(SIGINT) at line 6028. After SIGINT, curl exits and everything proceeds.

#### Conclusion

The hang is not in command substitution pipes. It's a direct external command
(`curl`) that blocks indefinitely when spawned by brush. Ctrl+C kills it and
everything unsticks. Next: log the fd configuration at spawn time to
understand why curl blocks — is stdout connected to a pipe nobody reads? Is
stdin connected to something that doesn't close?

### Experiment 5: Log fd configuration at external command spawn

#### Description

Experiment 4 showed that curl hangs as a direct pipeline stage, not inside
a command substitution. The hang is in `processes.rs` `wait()` — the child
process never exits on its own.

In `compose_std_command` (commands.rs:237-262), fds from `params` are cloned
onto the `Command` via `context.try_fd()`. Each `try_fd` call clones the
underlying fd (`PipeWriter::try_clone()`, etc.) — the original stays in
`params`. After spawn, the child holds its clone, but `params` still holds
the original.

If stdout is a `PipeWriter` and the read end of that pipe is never consumed,
curl will block when the pipe buffer fills. We need to know:
1. What fd types (Stdin/Stdout/Stderr/PipeWriter/PipeReader/File) are
   configured in `params` when this curl is spawned
2. Whether stdout is redirected to a pipe

#### Changes

**`brush/brush-core/src/commands.rs`** — enhance the existing spawn log
(~line 605) to include fd type information:

```rust
// Replace the existing debug_log block at spawn with:
{
    let stdin_type = context.try_fd(OpenFiles::STDIN_FD)
        .map(|f| format!("{:?}", std::mem::discriminant(&f)))
        .unwrap_or("inherited".to_string());
    let stdout_type = context.try_fd(OpenFiles::STDOUT_FD)
        .map(|f| format!("{:?}", std::mem::discriminant(&f)))
        .unwrap_or("inherited".to_string());
    let stderr_type = context.try_fd(OpenFiles::STDERR_FD)
        .map(|f| format!("{:?}", std::mem::discriminant(&f)))
        .unwrap_or("inherited".to_string());
    debug_log(&format!(
        "[brush:commands] spawning: {} {} | stdin={} stdout={} stderr={}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args().map(|a| a.to_string_lossy().to_string()).join(" "),
        stdin_type, stdout_type, stderr_type,
    ));
}
```

Note: `OpenFile` may not implement `Debug` with useful variant names. If so,
add a helper method or match on variants to produce readable names like
"PipeWriter", "Stdout", "File", etc.

#### Verification

1. `cargo build` succeeds
2. `rm -f /tmp/shannon-debug.log && tail -f /tmp/shannon-debug.log`
3. Run `nvm install 24` in bash mode
4. Find the hanging curl line in the log — check its stdin/stdout/stderr types
5. Compare with non-hanging commands to see what's different

**Result:** Pass (diagnostic success)

The hanging curl line in the log:

```
[brush:commands] spawning: /usr/bin/curl -q --fail --compressed -L -s
  https://nodejs.org/dist/index.tab -o - | stdin=Stdin stdout=PipeWriter stderr=Stderr
```

The fd types alone don't explain the hang — a later curl with identical fds
(`iojs.org/dist/index.tab`) completes fine. But cross-referencing with the
pipeline logs and `nvm.sh` source (line 1700) reveals the true structure:

```bash
VERSION_LIST="$(nvm_download -L -s "${MIRROR}/index.tab" -o - \
    | command sed "
        1d;
        s/^/${PREFIX}/;
      " \
)"
```

This is a **two-stage pipeline inside a command substitution**: `curl | sed`.
The logs confirm: after Ctrl+C kills the hanging curl, brush immediately
spawns `stage 2/2` (sed) and then both `run_parsed_result returned` and
`read_to_string completed` fire — everything completes.

**Root cause: brush serializes pipeline stages instead of running them
concurrently.** Curl is spawned first and brush calls `wait()` on it before
spawning sed. Curl writes to a pipe whose read end is connected to sed — but
sed hasn't started yet. When curl's output exceeds the OS pipe buffer
(~64KB), `write()` blocks. Deadlock: curl waits for sed to drain the pipe,
brush waits for curl to exit before starting sed.

The `iojs.org` curl doesn't hang because its output is small enough to fit
in the pipe buffer, so it completes before the buffer fills.

#### Conclusion

The bug is in brush's pipeline execution: it runs stages sequentially (spawn
stage 1, wait for it, spawn stage 2) instead of concurrently (spawn all
stages, then wait for the last one). This is a fundamental pipeline semantics
bug — Unix pipelines must run all stages concurrently so data can flow
through the pipe. The fix is in brush's pipeline executor, not in command
substitution or signal handling.

### Experiment 6: Spawn non-last pipeline stages concurrently

#### Description

In `spawn_pipeline_processes` (interp.rs:443), the loop at line 467 awaits
each `execute_in_pipeline` call sequentially. For external commands this is
fine — they return `StartedProcess` immediately. But shell functions return
`Completed` only after fully executing. When stage 1 is a shell function
(e.g., `nvm_download` which internally runs curl) and stage 2 is `sed`,
stage 1 fills the pipe buffer and blocks because stage 2 hasn't started.

The fix has two parts:

**Part 1: Add `Clone` to AST types.**

`ast::Command` does not implement `Clone`. We need it to clone commands
into `tokio::spawn` tasks. The AST types are pure data (parse trees) and
should be cloneable. Add `#[derive(Clone)]` to `Command` and any
constituent types that need it in `brush-parser/src/ast.rs`.

If adding `Clone` fails due to non-Clone fields deep in the AST, bail
and report which types block it.

**Part 2: Spawn non-last stages in tokio tasks.**

For non-last pipeline stages (`run_in_current_shell=false`), instead of
awaiting `execute_in_pipeline` sequentially:

1. Clone the shell and the command
2. `tokio::spawn` an async block that creates a `PipelineExecutionContext`
   with `ParentShell(&mut owned_shell)` and runs `execute_in_pipeline`
   to completion (converting any `StartedProcess` or `StartedTask` result
   to `ExecutionResult` via `.wait()`)
3. Push `StartedTask(join_handle)` into `spawn_results`

For `run_in_current_shell=true` (last stage or single command), keep the
current sequential `.await` behavior unchanged.

#### Changes

**`brush/brush-parser/src/ast.rs`** — add `Clone` derive to `Command` and
any types it contains that aren't already `Clone`.

**`brush/brush-core/src/interp.rs`** — modify `spawn_pipeline_processes`
loop (line 467) to split on `run_in_current_shell`:

- `true`: keep current `.await` path
- `false`: clone shell + command, `tokio::spawn`, push `StartedTask`

#### Verification

1. `cargo build` succeeds
2. `cargo test` passes
3. `nvm install 24` completes without hanging (the primary test)
4. `echo hello | cat` works (basic pipeline)
5. `seq 1 100000 | head -5` works (large output, early termination)
6. `ls | sort | head` works (3-stage pipeline)
7. Ctrl+C during a pipeline kills all stages

**Result:** Fail

`nvm install 24` still hangs. Worse: Ctrl+C no longer unsticks it — the
process freezes permanently, requiring the terminal to be killed. The
`tokio::spawn` approach broke signal delivery: the spawned task runs in a
different tokio context where SIGINT doesn't reach the child process, and
the parent shell's signal handling can't communicate with the detached task.

#### Conclusion

Spawning non-last pipeline stages in `tokio::spawn` is not the right fix.
The task runs in a separate async context that doesn't receive signals
properly, making Ctrl+C unable to kill the child process. The sequential
pipeline execution is more deeply embedded in brush's architecture than
a simple spawn-and-wait can solve. Need a different approach.
