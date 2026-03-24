+++
status = "open"
opened = "2026-03-24"
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
