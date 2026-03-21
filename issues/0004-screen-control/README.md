+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-21"
+++

# Issue 4: Screen control (Ctrl+L)

## Goal

Verify and ensure Ctrl+L clears the screen in shannon.

## Background

Every shell supports Ctrl+L to clear the terminal. Reedline's default emacs
keybindings likely already bind Ctrl+L to `ClearScreen`. This issue may just
need verification, not implementation.

### Verification steps

1. Run `cargo run` and press Ctrl+L.
2. If the screen clears, this issue can be closed immediately.
3. If not, add the keybinding in `build_editor()`.

## Experiments

### Experiment 1: Verify Ctrl+L binding in reedline source

#### Description

Check whether `default_emacs_keybindings()` already includes a Ctrl+L binding
by reading the reedline source code.

#### Verification

Confirmed in vendored reedline source
(`vendor/reedline/src/edit_mode/keybindings.rs` line 102):

```rust
kb.add_binding(KM::CONTROL, KC::Char('l'), ReedlineEvent::ClearScreen);
```

This is added by `add_common_control_bindings()`, which is called by
`default_emacs_keybindings()`. Shannon uses `default_emacs_keybindings()` in
`build_editor()`, so Ctrl+L already works.

**Result:** Pass — already works, no changes needed.

## Conclusion

Ctrl+L clear screen works out of the box via reedline's default emacs
keybindings. No code changes required.
