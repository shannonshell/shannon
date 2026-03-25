+++
status = "open"
opened = "2026-03-25"
+++

# Issue 26: Replace Enter-toggle with /ai command for AI mode

## Goal

Replace the "press Enter on empty line to toggle AI mode" behavior with a `/ai`
meta-command. Pressing Enter on an empty line should do nothing (or just redraw
the prompt), not change modes.

## Background

AI mode is currently toggled by pressing Enter on an empty prompt. This is too
easy to do by accident — any stray Enter press changes the shell's behavior
unexpectedly. The user has to notice the prompt changed and press Enter again to
get back.

Meta-commands (`/switch`, `/help`) are already the established pattern for shell
control. AI mode should follow the same pattern.

### New commands

- `/ai` — toggle AI mode (on → off, off → on)
- `/ai on` — turn AI mode on
- `/ai off` — turn AI mode off
- `/ai toggle` — same as `/ai` with no argument

### Changes needed

**`shannon/src/repl.rs`:**

1. Remove the "empty line toggles AI mode" logic (the `if line.is_empty()` block
   that sets `ai_mode` and rebuilds the editor)
2. Add `/ai` to `handle_meta_command` with on/off/toggle subcommands
3. Update `/help` output to show `/ai` instead of "Enter (empty)"

Empty Enter should reset `last_exit_code` to 0 (clearing the `!` indicator if
present) and continue the loop. This preserves the behavior from issue 23 where
Ctrl+C clears error state — empty Enter does the same.

## Experiments

### Experiment 1: Replace Enter toggle with /ai command

#### Description

Remove the empty-Enter AI toggle. Add `/ai` to `handle_meta_command`. Update
`/help`.

#### Changes

**`shannon/src/repl.rs`** — `handle_meta_command`:

Add `/ai` case:

```rust
"/ai" => {
    match arg {
        "on" => {
            ai_mode = true;
            ai_session = Some(Session::new());
        }
        "off" => {
            ai_mode = false;
            ai_session = None;
        }
        "" | "toggle" => {
            if ai_mode {
                ai_mode = false;
                ai_session = None;
            } else {
                ai_mode = true;
                ai_session = Some(Session::new());
            }
        }
        _ => {
            eprintln!("Usage: /ai [on|off|toggle]");
        }
    }
    // Rebuild editor to toggle highlighting
    *editor = build_editor(...);
    true
}
```

This requires `ai_mode` and `ai_session` to be passed to `handle_meta_command`.

**`shannon/src/repl.rs`** — empty line handler:

Replace the AI toggle block with:

```rust
if line.is_empty() {
    state.last_exit_code = 0;
    continue;
}
```

**`shannon/src/repl.rs`** — `/help` output:

Replace `"  Enter (empty)    — toggle AI mode"` with
`"  /ai [on|off]     — toggle AI mode"`.

#### Verification

1. `cargo test` passes.
2. `/ai` toggles AI mode on and off.
3. `/ai on` and `/ai off` work explicitly.
4. Empty Enter clears error state, does not toggle AI mode.
5. `/help` shows the new command.
