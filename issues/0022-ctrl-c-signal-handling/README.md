+++
status = "closed"
opened = "2026-03-24"
closed = "2026-03-24"
+++

# Issue 22: Fix Ctrl+C killing shannon during subprocess execution

## Goal

Ctrl+C during a running command (e.g. `npm run dev`) should kill only the child
process, not shannon itself. Shannon should return to the prompt.

## Background

When the user presses Ctrl+C, the terminal sends `SIGINT` to the entire
foreground process group. Both the child process and shannon receive the signal.
Shannon should ignore `SIGINT` while a subprocess is running and let only the
child handle it.

This is how bash, zsh, and fish work — the shell ignores `SIGINT` during command
execution and only handles it at the prompt (to cancel input).

### Current behavior

1. User runs `npm run dev` in shannon
2. User presses Ctrl+C
3. Both `npm` AND shannon receive SIGINT
4. Shannon exits — the shell is gone

### Expected behavior

1. User runs `npm run dev` in shannon
2. User presses Ctrl+C
3. `npm` receives SIGINT and exits
4. Shannon returns to the prompt

### Two execution paths

**Wrapper path (bash/fish/zsh):**

`execute_command` in `executor.rs` uses `Command::new(...).status()` which
blocks until the child exits. Shannon needs to ignore SIGINT before spawning the
child and restore it after.

```rust
// Before spawn
unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN); }

let status = Command::new(binary).args(["-c", &wrapper]).status();

// After spawn
unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL); }
```

Or use the `signal-hook` or `nix` crate for a safer API.

**Nushell embedded path:**

`eval_source` runs nushell commands in-process. When the command spawns external
processes (like `npm`), nushell's own signal handling should manage SIGINT. Need
to verify if nushell handles this correctly or if we need to add signal
management around `eval_source` too.

### Platform considerations

- `SIGINT` / `SIG_IGN` are POSIX (macOS and Linux)
- Windows uses a different mechanism (`SetConsoleCtrlHandler`)
- For MVP, POSIX is sufficient (macOS + Linux)

## Experiments

### Experiment 1: TDD — failing test, then fix

#### Description

Write a test that proves SIGINT during subprocess execution kills the process
(the bug), then fix it so the test passes.

#### The test

In `tests/integration.rs`, add a test that:

1. Spawns a bash subprocess via `execute_command` that sleeps briefly then exits
2. In a separate thread, sends SIGINT to our own process after a delay
3. Verifies that `execute_command` returns normally (process survived)

```rust
#[test]
fn test_sigint_during_subprocess() {
    use std::thread;
    use std::time::Duration;

    let state = initial_state();
    let pid = std::process::id();

    // Send SIGINT to ourselves after 200ms
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        unsafe { libc::kill(pid as i32, libc::SIGINT); }
    });

    // Run a command that takes 500ms — SIGINT arrives mid-execution
    let result = execute_command(&bash_config(), "sleep 0.5", &state);

    // If we get here, the process survived SIGINT
    assert!(result.is_ok(), "execute_command should survive SIGINT");
}
```

This test will FAIL with the current code (SIGINT kills the process).

#### The fix

In `executor.rs`, wrap the `Command::new(...).status()` call:

```rust
// Ignore SIGINT while child runs (let child handle it)
unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN); }

let status = Command::new(&shell_config.binary)
    .args(["-c", &wrapper])
    .env_clear()
    .envs(&state.env)
    .current_dir(&state.cwd)
    .status();

// Restore default SIGINT handling
unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL); }
```

Add `libc` as a dependency in Cargo.toml.

For the nushell embedded path in `repl.rs`, wrap `engine.execute()` with the
same signal ignore/restore. Nushell's internal signal handling should already
manage SIGINT for external commands, but wrapping it ensures consistency.

#### Verification

1. Run the test WITHOUT the fix — it fails (process killed by SIGINT).
2. Apply the fix.
3. Run the test WITH the fix — it passes.
4. `cargo test` — all tests pass.
5. Manual: run `npm run dev` (or `sleep 10`) in shannon, Ctrl+C, shannon
   survives and shows a new prompt.

**Result:** Fail

The integration test (sending SIGINT directly to the process) passed, but the
real scenario still fails. The integration test doesn't reproduce the actual
problem: in a real terminal, SIGINT goes to the entire foreground process group,
not just to our process. The `SIG_IGN` fix in `execute_command` is not
sufficient.

An external shell script test (`scripts/test-sigint.sh`) correctly reproduces
the failure. The fix needs to address process group behavior, not just
per-process signal handling.

#### Conclusion

The integration test approach was wrong — it tested signal delivery to a single
process, not process group behavior. The external script test confirms the bug
still exists. Need a different fix approach in experiment 2.

### Experiment 2: External script test + process group fix

#### Description

Remove the broken integration test. Use `scripts/test-sigint.sh` as the test.
Fix the actual problem: shannon needs to either put child processes in their own
process group, or become a session leader so that terminal SIGINT doesn't kill
it.

#### The real problem

When you press Ctrl+C, the terminal sends SIGINT to the **foreground process
group**. Shannon and its child subprocess are in the same group. Both receive
SIGINT. `SIG_IGN` on shannon's side isn't enough because reedline or the Rust
runtime may have their own signal handlers that override it.

#### The fix: pre_exec to set child process group

Use Rust's `Command::pre_exec` (Unix-only) to put the child in its own process
group via `setpgid(0, 0)`. Then give the child's group foreground control of the
terminal via `tcsetpgrp`. When Ctrl+C is pressed, SIGINT goes to the child's
process group only. Shannon doesn't receive it.

After the child exits, shannon reclaims foreground control.

```rust
use std::os::unix::process::CommandExt;

let mut cmd = Command::new(&shell_config.binary);
cmd.args(["-c", &wrapper])
    .env_clear()
    .envs(&state.env)
    .current_dir(&state.cwd);

unsafe {
    cmd.pre_exec(|| {
        // Put child in its own process group
        libc::setpgid(0, 0);
        Ok(())
    });
}

let child = cmd.spawn()?;
let child_pid = child.id() as i32;

// Give the child's process group foreground control of the terminal
unsafe {
    libc::tcsetpgrp(libc::STDIN_FILENO, child_pid);
}

let status = child.wait();

// Reclaim foreground control for shannon
unsafe {
    libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpgrp());
}
```

This is exactly what bash does — child gets its own process group, gets terminal
foreground, receives SIGINT on Ctrl+C. Shell stays in background, never gets
SIGINT.

#### Changes

**`shannon/src/executor.rs`**:

- Replace `Command::new(...).status()` with spawn + wait pattern
- Add `pre_exec` with `setpgid(0, 0)`
- Add `tcsetpgrp` before and after child execution
- Remove the `SIG_IGN`/`SIG_DFL` approach from experiment 1 (no longer needed)

**`shannon/tests/integration.rs`**:

- Remove `test_sigint_during_subprocess` test and the `libc`/`thread`/
  `Duration` imports it added
- Signal handling is tested via the external script, not integration tests

**`scripts/test-sigint.sh`**:

- Already exists from experiment 1
- Should pass after the fix: shannon survives SIGINT during subprocess

#### Verification

1. `scripts/test-sigint.sh` passes (shannon survives SIGINT).
2. `cargo test` passes (no regressions, broken test removed).
3. Manual: run `sleep 10` in shannon, Ctrl+C, shannon survives.
4. Manual: run `npm run dev`, Ctrl+C, npm stops, shannon survives.

**Result:** Partial

Shannon now survives Ctrl+C (SIG_IGN + SIGTTOU handling works). But the child
process (sleep) does NOT receive SIGINT — it continues running instead of being
killed. The process group + tcsetpgrp approach is not correctly forwarding
SIGINT to the child's group.

The problem is likely:

- The child's process group is set correctly (setpgid works)
- But tcsetpgrp may not be making it the true foreground group
- Or the wrapper bash process is absorbing/ignoring the signal
- Or there's a race between spawn, setpgid, and tcsetpgrp

91 tests pass. Broken integration test removed. Shannon survives Ctrl+C but
child doesn't receive it — needs further investigation in experiment 3.

#### Conclusion

Partial progress. Shannon no longer dies from Ctrl+C (the original crash is
fixed). But the child process doesn't receive SIGINT, so Ctrl+C effectively does
nothing visible. Need to investigate why the child's process group isn't
receiving the terminal's SIGINT.

### Experiment 3: Simple SIG_IGN cycle with pre_exec fix

**Result:** Partial

Ctrl+C now correctly kills the child process AND shannon survives. The
`pre_exec` fix was critical — `SIG_IGN` is inherited across `fork()`, so the
child was also ignoring SIGINT. `pre_exec` restores `SIG_DFL` in the child after
fork but before exec.

Two remaining issues:

1. The prompt shows `!` (error indicator) — because the child exited with a
   signal (exit code 130 or similar). This may be correct behavior (Ctrl+C is an
   error exit), but the `!` is confusing for an intentional interrupt.
2. The cwd resets to `/` — the wrapper script's env capture didn't run because
   the child was killed by SIGINT before reaching the capture code. The wrapper
   looks like:
   ```
   sleep 10
   __shannon_ec=$?
   (export -p; ...) > temp_file
   ```
   When sleep is killed by SIGINT, bash exits immediately without running the
   capture lines. Shannon falls back to the default state which has cwd `/`.

#### Conclusion

Core signal handling works. Two bugs remain:

- `!` error indicator after Ctrl+C (cosmetic — may be acceptable)
- cwd resets to `/` because wrapper env capture is skipped on SIGINT

### Experiment 4: Preserve state on interrupted commands

#### Description

When Ctrl+C kills a command, the wrapper's env capture code never runs. The temp
file is empty or missing. `execute_command` falls back to a default state with
cwd `/`. It should preserve the previous state instead.

The `!` error indicator is actually correct — Ctrl+C is a nonzero exit. Bash
reports exit code 130 (128 + signal 2). This is standard behavior.

#### The fix

In `execute_command`, the fallback when env capture fails already exists:

```rust
.unwrap_or_else(|| ShellState {
    env: state.env.clone(),
    cwd: state.cwd.clone(),
    last_exit_code: exit_code,
})
```

This should preserve env and cwd from the previous state. The problem is that
`exit_code` comes from `status.code()` which may return `None` for signal-killed
processes (the process didn't exit with a code, it was terminated by a signal).
When `code()` returns `None`, we fall back to 1.

But the real issue: when the child is killed by SIGINT, bash may exit before
writing the temp file. Let me check — does `.status()` return an error, or a
success with a signal exit code?

On Unix, `ExitStatus::code()` returns `None` for signal-terminated processes.
`ExitStatus::signal()` returns `Some(2)` for SIGINT. Our current code:

```rust
let exit_code = match &status {
    Ok(s) => s.code().unwrap_or(1),
    Err(_) => 1,
};
```

This gives exit code 1 for signal-killed processes. The env/cwd fallback should
use the previous state. Let me verify the fallback code is actually working
correctly — the bug might be that the temp file exists but is empty/corrupt,
causing a parse that returns wrong data.

#### Changes

**`shannon/src/executor.rs`**:

1. Check if the process was killed by a signal. If so, use exit code 128 +
   signal number (standard convention: 130 for SIGINT).

2. Verify the fallback preserves previous state. Add a debug print temporarily
   to confirm the fallback path is taken.

Actually — reading the code more carefully, the fallback IS correct. The issue
might be that `parse_bash_env` succeeds on an empty temp file and returns an
empty env with cwd `/`. Let me check:

`parse_bash_env("")` returns `Some((empty_map, PathBuf::from("/")))`.

That's the bug. An empty temp file parses successfully and returns cwd `/`. The
fix: if the temp file is empty or missing, skip parsing and use the fallback.

```rust
let new_state = std::fs::read_to_string(&temp_path)
    .ok()
    .filter(|contents| !contents.is_empty())  // ← add this
    .and_then(|contents| parse_output(...))
    ...
```

#### Verification

1. `cargo test` passes.
2. `sleep 10` + Ctrl+C → sleep dies, shannon shows prompt with correct cwd (not
   `/`) and `!` indicator (exit code 130).
3. Normal commands still capture env and cwd correctly.
4. `cd /tmp` then `sleep 10` + Ctrl+C → cwd stays `/tmp`.

**Result:** Pass

All verification steps confirmed. 91 tests pass. Ctrl+C kills the child, shannon
survives, cwd is preserved, exit code shows 130 (128 + SIGINT). The empty temp
file filter prevents the fallback to cwd `/`.

#### Conclusion

The Ctrl+C fix is complete for the wrapper path (bash/fish/zsh). Three pieces
working together:

1. `SIG_IGN` in shannon before child spawn (shannon ignores SIGINT)
2. `pre_exec` restores `SIG_DFL` in child (child receives SIGINT)
3. Empty temp file filtered (previous state preserved on interrupt)
4. `SIG_DFL` restored in REPL loop before `read_line` (reedline handles Ctrl+C
   at prompt)

Remaining: verify nushell embedded path handles Ctrl+C correctly.

### Experiment 5: Don't show error indicator for SIGINT

#### Description

After Ctrl+C, the prompt shows `!` because the exit code is 130 (nonzero). But
the user intentionally interrupted — it's not an error. Bash and zsh show a
normal prompt after Ctrl+C, not an error indicator.

Fix: treat signal exits (exit code >= 128) as non-errors for the prompt
indicator. The exit code is still stored correctly in `last_exit_code` (scripts
can check `$?`), but the visual indicator shows `>` not `!`.

#### Changes

**`shannon/src/prompt.rs`** — update `get_indicator_color`:

```rust
fn get_indicator_color(&self) -> Color {
    if self.last_exit_code != 0 && self.last_exit_code < 128 {
        self.error_color
    } else {
        self.indicator_color
    }
}
```

And update `render_prompt_indicator` similarly:

```rust
if self.last_exit_code != 0 && self.last_exit_code < 128 {
    Cow::Owned(format!(" {depth_prefix}! "))
} else {
    Cow::Owned(format!(" {depth_prefix}> "))
}
```

Exit codes >= 128 mean the process was killed by a signal (128 + signal number).
These are intentional interrupts, not errors.

#### Verification

1. `cargo test` passes.
2. `sleep 10` + Ctrl+C → prompt shows `>` (not `!`).
3. `false` → prompt shows `!` (real error, exit code 1).
4. `exit 1` in bash → prompt shows `!`.
5. Normal command → prompt shows `>`.

**Result:** Pass

All verification steps confirmed. 91 tests pass. Signal exits (>= 128) show `>`
not `!`. Real errors still show `!`.

#### Conclusion

Ctrl+C no longer shows an error indicator. The prompt correctly distinguishes
intentional interrupts from actual errors.

### Experiment 6: Connect nushell's Signals to real SIGINT

#### Description

The nushell embedded path doesn't handle Ctrl+C correctly. Currently we set
`SIG_IGN` before nushell execution, which prevents shannon from dying but also
prevents the child process from receiving SIGINT (since `SIG_IGN` is inherited
across fork).

The real fix: nushell has its own signal system via `Signals` struct (an
`Arc<AtomicBool>`). When connected and triggered, nushell checks
`signals.interrupted()` internally and stops execution. Our `NushellEngine` uses
`Signals::empty()` (disconnected) — we need to connect it.

Nushell's own binary does this in `src/signals.rs`:

```rust
let interrupt = Arc::new(AtomicBool::new(false));
engine_state.set_signals(Signals::new(interrupt.clone()));
ctrlc::set_handler(move || {
    interrupt.store(true, Ordering::Relaxed);
});
```

We'll use `signal-hook` (more composable than `ctrlc` crate) to register a
handler that sets the AtomicBool on SIGINT. This replaces SIG_DFL with a safe
handler — shannon won't die, and nushell will see the interrupt.

#### Changes

**`shannon/Cargo.toml`**:

- Add `signal-hook = "0.3"` dependency

**`shannon/src/nushell_engine.rs`**:

- Create `Arc<AtomicBool>` in `NushellEngine::new()`
- Call `engine_state.set_signals(Signals::new(interrupt.clone()))`
- Register SIGINT handler via `signal_hook::flag::register(SIGINT, interrupt)`
- Store the `Arc<AtomicBool>` on the struct
- Add `reset_signals()` method that calls `engine_state.reset_signals()`
- Call `reset_signals()` at the start of `execute()`

**`shannon/src/repl.rs`**:

- Remove `SIG_IGN` from the nushell path in `run_command`
- Call `engine.reset_signals()` is handled internally by execute()

#### Verification

1. `cargo test` passes.
2. Switch to nushell, run `sleep 10sec`, Ctrl+C → sleep is interrupted, shannon
   shows prompt.
3. Switch to bash, run `sleep 10`, Ctrl+C → sleep is interrupted, shannon shows
   prompt (no regression).
4. `scripts/test-sigint.sh` still passes.

**Result:** Fail

Ctrl+C still kills the entire terminal pane. The `signal-hook` handler alone is
not enough — it registers a handler that sets the AtomicBool, but SIGINT's
default disposition still terminates the process. `signal_hook::flag::register`
adds a flag-setting action but does NOT prevent the default signal behavior from
also running. Shannon dies before nushell ever gets to check
`signals.interrupted()`.

The fix needs to also prevent SIGINT from killing shannon during nushell
execution. Options:

1. Use `SIG_IGN` on shannon's side AND signal-hook for nushell's signals — but
   SIG_IGN overrides signal-hook's handler.
2. Manually set a no-op signal handler via libc that just returns (not SIG_IGN,
   which is inherited across fork). Then separately set the AtomicBool.
3. Use `signal_hook::low_level::register` with a custom action that only sets
   the flag (this replaces the default handler entirely).

#### Conclusion

The REPL loop sets `libc::signal(SIGINT, SIG_DFL)` at the top of every
iteration, which overwrites signal-hook's handler. By the time nushell runs,
SIGINT is back to SIG_DFL and kills the process. Need to keep signal-hook's
handler active during nushell execution.

### Experiment 7: Keep signal-hook handler active during nushell execution

#### Description

The REPL loop calls `libc::signal(SIGINT, SIG_DFL)` at the top of every
iteration. This was needed so reedline could handle Ctrl+C at the prompt. But
reedline uses crossterm raw mode — Ctrl+C is a keypress event, not a signal.
Reedline doesn't need SIG_DFL.

The fix:

1. Remove `libc::signal(SIGINT, SIG_DFL)` from the REPL loop
2. Signal-hook's handler (from NushellEngine) stays active permanently
3. For the wrapper path, executor.rs already sets SIG_IGN before spawn — but it
   never restores afterwards (the REPL loop was doing that). Now executor.rs
   must restore signal-hook's handler after the child exits.

Problem: executor.rs can't easily restore signal-hook's handler because it
doesn't own the registration. Instead, we'll move the signal setup out of
NushellEngine and into the REPL. The REPL will own the AtomicBool and pass it to
both NushellEngine and executor.

Simpler approach: just re-register signal-hook after every wrapper execution.
`signal_hook::flag::register` can be called multiple times safely — it adds
another action (but same effect since same AtomicBool). Or use
`signal_hook::low_level::register` to install a persistent handler.

Actually simplest: the NushellEngine may not exist (if nushell isn't available).
So move signal-hook registration to REPL startup unconditionally. The handler
sets an AtomicBool that nushell uses if present. After wrapper execution,
re-register the signal-hook handler (since executor.rs set SIG_IGN which
overwrites it).

#### Changes

**`shannon/src/nushell_engine.rs`**:

- Accept `Arc<AtomicBool>` in `new()` instead of creating it internally
- Remove signal-hook registration from here

**`shannon/src/repl.rs`**:

- Create `Arc<AtomicBool>` at REPL startup
- Register signal-hook handler for SIGINT
- Pass the Arc to NushellEngine
- Remove `libc::signal(SIGINT, SIG_DFL)` from top of loop
- After `run_command` returns (for wrapper path only), re-register signal-hook
  to restore the handler that SIG_IGN overwrote

#### Verification

1. `cargo test` passes.
2. Nushell: `sleep 10sec` + Ctrl+C → interrupted, shannon survives.
3. Bash: `sleep 10` + Ctrl+C → interrupted, shannon survives.
4. `scripts/test-sigint.sh` passes.

**Result:** Pass

All verification steps confirmed. 91 tests pass. Nushell's `sleep 10sec` is
correctly interrupted by Ctrl+C and shannon returns to the prompt. The
"Operation interrupted" error message with source annotation is standard nushell
behavior (nushell's own REPL shows the same thing).

#### Conclusion

Nushell embedded path now handles Ctrl+C correctly. The fix has three parts:

1. `signal-hook` registers a SIGINT handler that sets an `Arc<AtomicBool>`
2. `NushellEngine` connects that Arc to nushell's `Signals` system via
   `set_signals(Signals::new(interrupt))`
3. The REPL no longer overwrites signal-hook with `SIG_DFL` — reedline uses
   crossterm raw mode and doesn't need OS-level SIGINT handling

After wrapper execution (bash/fish/zsh), signal-hook is re-registered since
executor.rs's `SIG_IGN` overwrites it.

Both execution paths (wrapper and embedded) now handle Ctrl+C correctly.

### Experiment 8: Revert error indicator special-casing for SIGINT

#### Description

Experiment 5 made exit code 130 (SIGINT) show `>` instead of `!` in the prompt.
This creates an inconsistency: Ctrl+C in bash/fish/zsh shows `>`, but Ctrl+C in
nushell shows `!` (because nushell returns exit code 1, not 130).

The user doesn't know or care about wrapper vs embedded paths. They see
inconsistent behavior across shells. The fix: revert experiment 5. All shells
show `!` after Ctrl+C. This is correct — the user interrupted something, the
command exited nonzero, and `!` accurately reflects that.

#### Changes

**`shannon/src/prompt.rs`**:

Revert the `!= 130` special case in both `render_prompt_indicator` and
`get_indicator_color`. Return to the original logic: any nonzero exit code shows
`!`.

```rust
// render_prompt_indicator
if self.last_exit_code != 0 {
    Cow::Owned(format!(" {depth_prefix}! "))
} else {
    Cow::Owned(format!(" {depth_prefix}> "))
}

// get_indicator_color
if self.last_exit_code != 0 {
    self.error_color
} else {
    self.indicator_color
}
```

#### Verification

1. `cargo test` passes.
2. Bash: `sleep 10` + Ctrl+C → prompt shows `!`.
3. Nushell: `sleep 10sec` + Ctrl+C → prompt shows `!`.
4. Both shells: `echo hello` → prompt shows `>`.
5. Both shells: `false` → prompt shows `!`.

**Result:** Pass

All verification steps confirmed. 91 tests pass. Ctrl+C now consistently shows
`!` in both bash and nushell.

#### Conclusion

Reverted the SIGINT special-casing from experiment 5. All shells now behave
consistently: any nonzero exit code (including Ctrl+C interrupts) shows `!`.

## Conclusion

Ctrl+C no longer kills shannon. Eight experiments, three key fixes:

1. **Wrapper path (bash/fish/zsh):** `SIG_IGN` before child spawn + `pre_exec`
   to restore `SIG_DFL` in the child + empty temp file filter to preserve state
   on interrupt.
2. **Nushell embedded path:** `signal-hook` registers a SIGINT handler that sets
   an `Arc<AtomicBool>` connected to nushell's `Signals` system. Nushell checks
   `signals.interrupted()` internally and stops execution.
3. **Prompt consistency:** All shells show `!` after Ctrl+C — no special-casing
   for signal exit codes.
