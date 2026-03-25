+++
status = "open"
opened = "2026-03-25"
+++

# Issue 29: Remove Ctrl+S shell picker menu

## Goal

Remove the Ctrl+S keybinding and the shell picker menu. Shell switching is
handled by Shift+Tab (cycle) and `/switch <shell>` (explicit). The Ctrl+S menu
adds complexity without value — it requires two keypresses (select + Enter) and
the menu UI is clunky.

## Changes needed

**`shannon/src/repl.rs`:**

1. Remove the Ctrl+S keybinding from `build_editor()`
2. Remove the `ShellSwitchCompleter` struct and its `Completer` impl
3. Remove the `shell_menu` (IdeMenu + WithCompleter) from the editor builder
4. Remove `shell_names` parameter from `build_editor()` (only used for menu)
5. Update `/help` to remove the Ctrl+S line

**Docs:**

6. Remove Ctrl+S from `docs/reference/01-keybindings.md`
7. Remove Ctrl+S mention from `/help` output

## Experiments

### Experiment 1: Remove Ctrl+S and shell picker menu

#### Description

Remove all Ctrl+S / shell menu code. Pure deletion.

#### Changes

**`shannon/src/repl.rs`:**
- Remove Ctrl+S keybinding from `build_editor()`
- Remove `ShellSwitchCompleter` struct and `Completer` impl
- Remove `shell_menu` variable and `.with_menu(shell_menu)` from builder
- Remove `shell_names` parameter from `build_editor()` and all call sites
- Remove `Ctrl+S` line from `/help` output

**`docs/reference/01-keybindings.md`:**
- Remove Ctrl+S row from the keybindings table

#### Verification

1. `cargo test` passes.
2. Ctrl+S does nothing (no menu appears).
3. Shift+Tab still cycles shells.
4. `/switch brush` still works.
5. `/help` does not mention Ctrl+S.
