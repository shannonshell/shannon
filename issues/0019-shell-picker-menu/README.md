+++
status = "open"
opened = "2026-03-23"
+++

# Issue 19: Shell picker menu via Ctrl+Tab

## Goal

Add a Ctrl+Tab keybinding that opens a selection menu showing all available
shells. The user picks one and immediately switches to it, instead of cycling
through with Shift+Tab.

## Background

Shift+Tab cycles through shells in order: bash → nu → fish → zsh → bash. If
you're in bash and want fish, you press Shift+Tab twice. With four shells, this
is tedious.

Ctrl+Tab opens a menu (like the completion menu) showing all shells. You pick
the one you want directly. One keypress to open, one to select.

### How it works

1. User presses **Ctrl+Tab** at the prompt
2. A columnar menu appears below the prompt showing available shells:
   ```
   bash  nu  fish  zsh
   ```
3. User navigates with Tab/arrows and presses Enter to select
4. Shannon switches to the selected shell immediately
5. The menu disappears and the prompt updates

### Implementation via reedline

Reedline supports multiple menus, each with its own completer. We use
`ReedlineMenu::WithCompleter` to create a shell picker menu that's independent
of the command/file completion menu.

**Keybinding:**

```rust
keybindings.add_binding(
    KeyModifiers::CONTROL,
    KeyCode::Tab,  // or KeyCode::BackTab with different modifiers
    ReedlineEvent::Menu("shell_menu".to_string()),
);
```

Note: Ctrl+Tab may not be distinguishable from Tab in some terminals. Need to
test. Alternative: Ctrl+S, Ctrl+G, or another unused binding.

**Shell completer:**

A simple completer that returns the shell names as suggestions:

```rust
struct ShellSwitchCompleter {
    shells: Vec<String>,
}

impl Completer for ShellSwitchCompleter {
    fn complete(&mut self, _line: &str, _pos: usize) -> Vec<Suggestion> {
        self.shells.iter().map(|name| Suggestion {
            value: format!("__shannon_switch:{name}"),
            display_override: Some(name.clone()),
            description: None,
            ...
        }).collect()
    }
}
```

The `display_override` shows the clean shell name while the actual value
contains the switch command prefix. When the user selects and presses Enter, the
input becomes `__shannon_switch:fish`. The main loop parses this and switches.

**Menu setup:**

```rust
let shell_menu = ReedlineMenu::WithCompleter {
    menu: Box::new(ColumnarMenu::default().with_name("shell_menu")),
    completer: Box::new(ShellSwitchCompleter { shells }),
};

Reedline::create()
    .with_menu(ReedlineMenu::EngineCompleter(...))  // existing completion menu
    .with_menu(shell_menu)                           // new shell picker
```

**Main loop detection:**

```rust
if line.starts_with("__shannon_switch:") {
    let target = &line["__shannon_switch:".len()..];
    // Find the shell by name and switch to it
}
```

### Ctrl+Tab terminal support

Ctrl+Tab sends different escape sequences depending on the terminal:

- Some terminals send it as a distinct key event
- Some terminals can't distinguish Ctrl+Tab from Tab
- Ghostty, wezterm, and iTerm2 generally support it

If Ctrl+Tab doesn't work reliably, we can use an alternative binding. The
feature works regardless of which key triggers it.

## Experiments

### Experiment 1: Shell picker menu

#### Description

Add a shell picker menu triggered by Ctrl+Tab (or fallback keybinding).
Uses reedline's `WithCompleter` menu pattern — a dedicated completer that
returns shell names, independent of the command/file completion menu.

#### Changes

**`shannon/src/repl.rs`** — add shell picker:

1. Create `ShellSwitchCompleter` struct:
   ```rust
   struct ShellSwitchCompleter {
       shells: Vec<String>,
   }
   impl Completer for ShellSwitchCompleter { ... }
   ```
   Returns one `Suggestion` per shell. Uses `display_override` to show the
   clean name while the `value` is `__shannon_switch:{name}`.

2. In `build_editor`, add the shell menu:
   ```rust
   let shell_menu = ReedlineMenu::WithCompleter {
       menu: Box::new(ColumnarMenu::default().with_name("shell_menu")),
       completer: Box::new(ShellSwitchCompleter { shells }),
   };
   ```
   Add `.with_menu(shell_menu)` to the reedline builder.

3. Add Ctrl+Tab keybinding to both insert and normal modes:
   ```rust
   kb.add_binding(
       KeyModifiers::CONTROL,
       KeyCode::Tab,
       ReedlineEvent::Menu("shell_menu".to_string()),
   );
   ```

4. Update `build_editor` signature to accept shell names:
   ```rust
   fn build_editor(
       shell_config: &ShellConfig,
       session_id: Option<HistorySessionId>,
       ai_mode: bool,
       theme: &Theme,
       shell_names: &[String],
   ) -> Reedline
   ```

5. Update the main loop to detect `__shannon_switch:{name}`:
   ```rust
   if let Some(target) = line.strip_prefix("__shannon_switch:") {
       // Find shell by name, switch to it
   } else if line == SWITCH_COMMAND {
       // Existing Shift+Tab cycle behavior
   }
   ```

**Pass shell names through:** The `build_editor` calls need the list of
shell names. Extract them from the `shells` vec and pass through.

#### Testing Ctrl+Tab

Ctrl+Tab may not work in all terminals. If it doesn't, try these
alternatives in order:
- `KeyModifiers::CONTROL, KeyCode::BackTab` (Ctrl+Shift+Tab)
- `KeyModifiers::ALT, KeyCode::Tab` (Alt+Tab — may conflict with OS)
- `KeyModifiers::CONTROL, KeyCode::Char('g')` (Ctrl+G — unused)

Test in ghostty first. If Ctrl+Tab works, keep it. If not, fall back to
Ctrl+G or similar.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Press Ctrl+Tab — shell menu appears with all available shells.
4. Navigate with Tab/arrows, press Enter — switches to selected shell.
5. Prompt updates to show the new shell.
6. Shift+Tab still works for cycling.
7. State (env, cwd) carries over on switch (same as Shift+Tab).
8. Menu doesn't appear if only one shell is available.
