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
`signal-hook::register()` returns a registration ID that can be used
with `signal-hook::unregister()`. Before calling
`dispatcher.execute()`, temporarily unregister nushell's SIGINT handler so
brush's tokio signal handlers work uncontested. Re-register after brush returns.

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
- `signal-hook` registration IDs need to be threaded through to the
  REPL dispatch code. May need to store on `EngineState` or pass via
  `LoopContext`.
- Need to ensure the handler is always re-registered, even on panic (use a
  guard/drop pattern).

## Experiments

### Experiment 1: Replace ctrlc with signal-hook, unregister during brush

#### Description

Replace the `ctrlc` crate with `signal-hook` for SIGINT handling. Store
the registration `SigId` so it can be unregistered before brush execution and
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
signal-hook handler did not fix the issue. The `ctrlc` crate's handler is
STILL registered — we added a signal-hook handler but never removed the ctrlc
one. The `ctrlc` crate registers its own handler at startup (internally, via
nushell's upstream code or our own call), and that handler persists regardless
of what we do with signal-hook. Two handlers are now registered: the original
ctrlc one and our new signal-hook one.

The root cause may not be what we assumed. Possibilities:
1. The `ctrlc` crate handler is still active and consuming SIGINT
2. The issue isn't about handler competition but about how brush's tokio
   runtime handles signals inside `block_on()`
3. The child process isn't in the right process group to receive SIGINT

#### Conclusion

Simply adding signal-hook alongside ctrlc doesn't fix the issue. We need to
either fully remove ctrlc (but nushell's internal code depends on it) or
investigate whether the root cause is something else entirely.
