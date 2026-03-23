+++
status = "closed"
opened = "2026-03-23"
closed = "2026-03-23"
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

**Result:** Partial

Ctrl+S menu works — shows shells, selection switches correctly. Two issues:

1. Ctrl+Tab was taken by the terminal — changed to Ctrl+S (S for switch)
2. The menu fills `__shannon_switch:zsh` into the prompt, which looks ugly
   and requires an extra Enter press

#### Conclusion

The picker menu works but the internal `__shannon_switch:` prefix is
user-visible. This motivates a meta-command system with `/` prefix that
looks intentional. The menu should fill in `/switch zsh` instead.

### Experiment 2: Meta-commands with `/` prefix

#### Description

Replace `__shannon_switch:{name}` with `/switch {name}`. Establish a
meta-command convention: lines starting with `/` are checked against known
shannon commands before being sent to the shell. If `/command` doesn't
match a known meta-command AND a file `/command` exists on the filesystem,
it's sent to the shell. Otherwise it's handled as a meta-command.

This also means users can type `/switch zsh` directly without the picker
menu.

#### Changes

**`shannon/src/repl.rs`**:

1. Update `ShellSwitchCompleter` — change suggestion values from
   `__shannon_switch:{name}` to `/switch {name}`:

   ```rust
   value: format!("/switch {name}"),
   display_override: Some(name.clone()),
   ```

2. Add meta-command detection in the main loop. After trimming the line,
   before the existing `__shannon_switch` check:

   ```rust
   // Meta-commands: /switch, /help, etc.
   if line.starts_with('/') {
       if handle_meta_command(line, &shells, &mut active_idx, &mut editor,
           session_id, ai_mode, &theme, &shell_names) {
           continue;
       }
       // Not a known meta-command — fall through to shell execution
   }
   ```

3. Add `handle_meta_command` function:

   ```rust
   fn handle_meta_command(
       line: &str,
       shells: &[(String, ShellConfig)],
       active_idx: &mut usize,
       editor: &mut Reedline,
       session_id: Option<HistorySessionId>,
       ai_mode: bool,
       theme: &Theme,
       shell_names: &[String],
   ) -> bool {
       let parts: Vec<&str> = line.splitn(2, ' ').collect();
       let cmd = parts[0];
       let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

       // Check if a file with this name exists on the filesystem
       if std::path::Path::new(cmd).exists() {
           return false; // let the shell handle it
       }

       match cmd {
           "/switch" => {
               if let Some(idx) = shells.iter().position(|(n, _)| n == arg) {
                   *active_idx = idx;
                   *editor = build_editor(...);
               } else if !arg.is_empty() {
                   eprintln!("shannon: unknown shell: {arg}");
               } else {
                   // No arg — list available shells
                   let names: Vec<&str> = shells.iter().map(|(n, _)| n.as_str()).collect();
                   eprintln!("Available shells: {}", names.join(", "));
               }
               true
           }
           "/help" => {
               eprintln!("Shannon commands:");
               eprintln!("  /switch <shell>  — switch to a shell");
               eprintln!("  /help            — show this help");
               eprintln!("  Shift+Tab        — cycle to next shell");
               eprintln!("  Ctrl+S           — shell picker menu");
               eprintln!("  Enter (empty)    — toggle AI mode");
               true
           }
           _ => false, // unknown /command, let shell handle it
       }
   }
   ```

4. Remove the `__shannon_switch:` detection (replaced by `/switch`).

5. Update highlighter to skip tree-sitter for meta-commands. In
   `TreeSitterHighlighter::highlight()`, at the top:

   ```rust
   if line.starts_with('/') {
       let first_word = line.split_whitespace().next().unwrap_or("");
       if matches!(first_word, "/switch" | "/help") {
           styled.push((Style::new().fg(self.foreground), line.to_string()));
           return styled;
       }
   }
   ```

   This renders meta-commands in plain foreground text — no confusing
   syntax highlighting artifacts.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Ctrl+S menu shows shells, selecting fills `/switch zsh`, Enter switches.
4. Typing `/switch bash` directly switches to bash.
5. `/switch` with no arg lists available shells.
6. `/help` shows help text.
7. `/usr/bin/env` still works (filesystem check finds the file).
8. `/nonexistent` with no matching file and no matching command goes to shell.
9. Shift+Tab still cycles.
10. Meta-commands don't get tree-sitter highlighted (plain text).

**Result:** Pass

All verification steps confirmed. 88 tests pass. `/switch` and `/help`
meta-commands work. Ctrl+S opens a vertical ListMenu (arrow key navigation).
Filesystem check prevents conflicts with real files. Highlighter skips
tree-sitter for meta-commands.

Additional fix during implementation: changed ColumnarMenu (horizontal,
Tab-only) to ListMenu (vertical, arrow keys) for the shell picker.

#### Conclusion

Meta-command system is in place with `/switch` and `/help`. The shell
picker menu (Ctrl+S) uses the same `/switch` command. The foundation
supports future meta-commands (`/model`, `/theme`, etc.).

## Conclusion

Issue complete. Two ways to switch shells:

- **Shift+Tab** — instant cycle (unchanged)
- **Ctrl+S** — vertical picker menu with arrow key navigation
- **`/switch {name}`** — type directly

Plus `/help` for discoverability. Meta-commands use `/` prefix with
filesystem existence check to avoid conflicts.

Key files:
- `shannon/src/repl.rs` — ShellSwitchCompleter, handle_meta_command,
  ListMenu for shell picker
- `shannon/src/highlighter.rs` — skips tree-sitter for meta-commands
