+++
status = "open"
opened = "2026-03-24"
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

The integration test (sending SIGINT directly to the process) passed, but
the real scenario still fails. The integration test doesn't reproduce the
actual problem: in a real terminal, SIGINT goes to the entire foreground
process group, not just to our process. The `SIG_IGN` fix in
`execute_command` is not sufficient.

An external shell script test (`scripts/test-sigint.sh`) correctly
reproduces the failure. The fix needs to address process group behavior,
not just per-process signal handling.

#### Conclusion

The integration test approach was wrong — it tested signal delivery to a
single process, not process group behavior. The external script test
confirms the bug still exists. Need a different fix approach in
experiment 2.

### Experiment 2: External script test + process group fix

#### Description

Remove the broken integration test. Use `scripts/test-sigint.sh` as the
test. Fix the actual problem: shannon needs to either put child processes
in their own process group, or become a session leader so that terminal
SIGINT doesn't kill it.

#### The real problem

When you press Ctrl+C, the terminal sends SIGINT to the **foreground
process group**. Shannon and its child subprocess are in the same group.
Both receive SIGINT. `SIG_IGN` on shannon's side isn't enough because
reedline or the Rust runtime may have their own signal handlers that
override it.

#### The fix: pre_exec to set child process group

Use Rust's `Command::pre_exec` (Unix-only) to put the child in its own
process group via `setpgid(0, 0)`. Then give the child's group foreground
control of the terminal via `tcsetpgrp`. When Ctrl+C is pressed, SIGINT
goes to the child's process group only. Shannon doesn't receive it.

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

This is exactly what bash does — child gets its own process group, gets
terminal foreground, receives SIGINT on Ctrl+C. Shell stays in background,
never gets SIGINT.

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
