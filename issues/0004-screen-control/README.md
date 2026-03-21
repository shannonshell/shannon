+++
status = "open"
opened = "2026-03-21"
+++

# Issue 4: Screen control (Ctrl+L)

## Goal

Verify and ensure Ctrl+L clears the screen in olshell.

## Background

Every shell supports Ctrl+L to clear the terminal. Reedline's default emacs
keybindings likely already bind Ctrl+L to `ClearScreen`. This issue may just
need verification, not implementation.

### Verification steps

1. Run `cargo run` and press Ctrl+L.
2. If the screen clears, this issue can be closed immediately.
3. If not, add the keybinding in `build_editor()`.
