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

### Experiment 4: Fix held pipe writer in command substitution

#### Description

In `brush/brush-core/src/commands.rs`, `invoke_command_in_subshell_and_get_output()`
creates a pipe writer, stores it in `params` via `set_fd`, then spawns a tokio
task with `params`. Inside that task, `run_substitution_command` calls
`shell.run_parsed_result()` which spawns child processes. The child processes
get a CLONE of the writer (via `try_fd` → `f.clone()` at `interp.rs:130`).
When the child exits, the clone closes, but the original writer in `params`
stays alive until `run_substitution_command` returns. The reader blocks on
`read_to_string()` waiting for EOF that never arrives until the task completes.

The fix: in `run_substitution_command`, clear the stdout fd from `params`
AFTER `run_parsed_result` has spawned its child processes but BEFORE waiting
for them to complete. Or simpler: clear it right after `run_parsed_result`
returns, which would allow the reader to see EOF. But that doesn't help —
we need the reader to see EOF WHILE the pipeline is still running.

Actually, the simplest fix: clear the stdout fd from `params` BEFORE calling
`run_parsed_result`. The child processes don't need `params` to hold the
writer — they get their own clone when they're spawned (via `compose_std_command`
→ `try_fd` → `clone`). So we can drop it from params right after the parse
and before execution.

Wait — `run_parsed_result` passes `params` to the pipeline executor, which
passes it to each command. If we clear stdout from params, the commands won't
get the pipe writer at all.

Better approach: clear the stdout fd from `params` inside
`run_substitution_command` AFTER `run_parsed_result` returns. The issue is
that `read_to_string` runs concurrently and needs to see EOF. Since
`run_parsed_result` awaits until all pipeline processes complete, and the
child processes hold their own cloned fds, by the time `run_parsed_result`
returns all child fds should be closed. But `params` still holds the
original writer, preventing EOF.

So: drop `params` (or clear the fd) after `run_parsed_result` returns,
BEFORE the function returns. Then the reader sees EOF and `read_to_string`
completes.

Actually wait — the reader runs concurrently. `read_to_string` is at line
753, awaited in the CALLER. `run_substitution_command` is in the tokio task.
When the task returns, `params` drops, writer closes, reader sees EOF. This
SHOULD work already.

Unless `run_parsed_result` itself is blocked. If the pipeline waits for the
LAST command in the pipeline, and that last command reads from stdin (piped
from the previous command), and the previous command is still writing...
but that's a different issue.

Let me reconsider: the hang happens with a **single command** (`curl` as
stage 1/1). There's no pipeline. `run_parsed_result` runs curl, waits for
curl to complete. Curl writes to stdout (the pipe). The reader runs
concurrently. Curl should complete, `run_parsed_result` returns, params
drops, writer closes, reader sees EOF.

So why does it hang? Maybe `run_parsed_result` is NOT the bottleneck.
Maybe the tokio spawn task itself is blocking. Let me add a log inside
`run_substitution_command` to confirm.

#### Changes

**`brush/brush-core/src/commands.rs`** — add debug logs in
`run_substitution_command` (line ~765):

```rust
async fn run_substitution_command(...) -> ... {
    debug_log("[brush:subst] entering run_substitution_command");
    let parse_result = shell.parse_string(command);
    debug_log("[brush:subst] parsed, executing...");
    let result = shell.run_parsed_result(parse_result, &source_info, &params).await;
    debug_log("[brush:subst] run_parsed_result returned");
    result
}
```

Also add a log right after the writer is created in
`invoke_command_in_subshell_and_get_output`:

```rust
debug_log("[brush:subst] pipe created, spawning task");
// after tokio::spawn:
debug_log("[brush:subst] task spawned, reading output...");
// after read_to_string:
debug_log("[brush:subst] read_to_string completed");
```

#### Verification

1. `cargo build` succeeds
2. Run `nvm install 24`, check /tmp/shannon-debug.log
3. Determine: does `run_parsed_result` return? Or does it hang?
4. If it hangs: the issue is inside pipeline execution, not the pipe writer
5. If it returns but reader blocks: the pipe writer theory is confirmed
