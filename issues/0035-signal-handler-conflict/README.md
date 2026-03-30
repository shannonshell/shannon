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

Replace `ctrlc` with `signal-hook-registry` directly in Shannon's signal setup.
`signal-hook-registry::register()` returns a registration ID that can be used
with `signal-hook-registry::unregister()`. Before calling
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

- Add `signal-hook-registry` dependency
- Remove `ctrlc` dependency (if no longer needed)

### Complications

- The `ctrlc` crate is also used by nushell's upstream code. Removing it may
  require changes to how nushell's signal handlers are registered.
- `signal-hook-registry` registration IDs need to be threaded through to the
  REPL dispatch code. May need to store on `EngineState` or pass via
  `LoopContext`.
- Need to ensure the handler is always re-registered, even on panic (use a
  guard/drop pattern).
