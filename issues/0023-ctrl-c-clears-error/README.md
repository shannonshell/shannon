+++
status = "closed"
opened = "2026-03-24"
closed = "2026-03-24"
+++

# Issue 23: Ctrl+C on empty prompt should clear error state

## Goal

Pressing Ctrl+C at an empty prompt should reset `last_exit_code` to 0, clearing
the `!` error indicator. The next prompt should show `>`.

## Background

After a command fails (or is interrupted with Ctrl+C), the prompt shows `!` to
indicate a nonzero exit code. This is correct. But the error indicator persists
until the user runs a successful command.

In bash and zsh, pressing Ctrl+C at the prompt clears the line and resets the
error state. The prompt goes back to normal. This is a natural "dismiss" gesture
— the user acknowledges the error and moves on.

Currently in shannon, Ctrl+C at the prompt is handled by reedline as
`Signal::CtrlC`. Shannon checks if AI mode is active (and exits it), then
continues the loop. The `last_exit_code` is not reset, so the `!` persists.

### Current behavior

1. Run `false` → prompt shows `!`
2. Press Ctrl+C → prompt still shows `!`

### Expected behavior

1. Run `false` → prompt shows `!`
2. Press Ctrl+C → prompt shows `>`

### The fix

In `repl.rs`, in the `Signal::CtrlC` handler, reset `state.last_exit_code` to 0:

```rust
Ok(Signal::CtrlC) => {
    if ai_mode {
        ai_mode = false;
        ai_session = None;
    }
    state.last_exit_code = 0;
    continue;
}
```

## Experiments

### Experiment 1: Reset exit code on Ctrl+C

#### Description

Add `state.last_exit_code = 0` to the `Signal::CtrlC` handler in `repl.rs`.

#### Changes

**`shannon/src/repl.rs`** — in the `Signal::CtrlC` match arm, add
`state.last_exit_code = 0;` before `continue`.

#### Verification

1. `cargo test` passes.
2. Run `false` → prompt shows `!`. Press Ctrl+C → prompt shows `>`.
3. Run `sleep 10` + Ctrl+C → prompt shows `!`. Press Ctrl+C → prompt shows `>`.
4. In nushell: `sleep 10sec` + Ctrl+C → prompt shows `!`. Press Ctrl+C → prompt
   shows `>`.

**Result:** Pass

All verification steps confirmed. 91 tests pass.

#### Conclusion

One-line fix. Ctrl+C at the prompt now clears the error state.

## Conclusion

Ctrl+C at an empty prompt resets `last_exit_code` to 0, clearing the `!`
indicator. Works consistently across all shells.
