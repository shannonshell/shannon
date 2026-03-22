+++
status = "closed"
opened = "2026-03-22"
closed = "2026-03-22"
+++

# Issue 16: Terminal integration (OSC sequences)

## Goal

Emit standard OSC escape sequences so terminal emulators can track shannon's
state — current working directory, window title, and prompt markers. This
enables features like opening new panes in the same directory and showing the
running command in the tab title.

## Background

Terminal emulators (ghostty, wezterm, iTerm2, etc.) rely on shells sending OSC
(Operating System Command) escape sequences to communicate metadata. Shannon
currently sends none of these, which means:

1. **New panes open in the home directory** instead of the current cwd.
2. **Tab/window title is blank or generic** — ghostty shows a ghost icon.
3. **No prompt markers** — the terminal can't distinguish prompts from output
   for features like scroll-to-prompt.

### OSC sequences to implement

| OSC     | Purpose                          | When to emit                                                                       |
| ------- | -------------------------------- | ---------------------------------------------------------------------------------- |
| OSC 7   | Report current working directory | After every command (cwd may have changed)                                         |
| OSC 2   | Set window/tab title             | Before prompt (idle title) and before execution (command title)                    |
| OSC 133 | Prompt markers                   | Before prompt start, after prompt end, before command output, after command output |

### OSC 7 — Current Working Directory

```
\x1b]7;file://hostname/path/to/dir\x1b\\
```

Emitted after every command completes (and at startup). The terminal uses this
to open new panes/tabs in the same directory. The path must be percent-encoded
and include the hostname.

### OSC 2 — Window/Tab Title

```
\x1b]2;title text\x07
```

Two states:

- **Idle** (at prompt): `\x1b]2;[nu] ~/project\x07`
- **Running command**: `\x1b]2;[nu] ~/project> git status\x07`

The idle title shows the shell name and cwd. The running title adds the command
name. This gives users visual feedback in the tab bar about what's happening.

### OSC 133 — Prompt Markers (Semantic Prompts)

```
\x1b]133;A\x07  — prompt start
\x1b]133;B\x07  — prompt end (command start)
\x1b]133;C\x07  — command output start
\x1b]133;D;exit_code\x07  — command output end
```

These enable terminal features like:

- Scroll to previous/next prompt
- Select command output
- Dim previous commands

Reedline may handle some of these already — it has `Osc133ClickEventsMarkers`
support. Need to check if reedline emits these or if we need to do it ourselves.

### Implementation approach

These are simple `eprint!` calls at specific points in the REPL loop:

1. **Before showing prompt**: emit OSC 2 (idle title) + OSC 7 (cwd)
2. **After user presses Enter**: emit OSC 2 (command title)
3. **After command completes**: emit OSC 7 (updated cwd)

For nushell mode, `eval_source` may already emit some of these internally. Need
to check and avoid duplicates.

### Hostname

OSC 7 requires a hostname. Use `gethostname()` from the `hostname` crate, or
`std::env::var("HOSTNAME")`, or fall back to "localhost".

### Research from nushell

Nushell's REPL implements all of these via `run_shell_integration_osc2`,
`run_shell_integration_osc7`, and `run_shell_integration_osc633` functions in
`crates/nu-cli/src/repl.rs`. Key patterns:

- OSC 2 title: `\x1b]2;{path}> {command}\x07` when running, `\x1b]2;{path}\x07`
  when idle
- OSC 7 cwd: `\x1b]7;file://{hostname}{path}\x1b\\` with percent-encoding
- Path is tilde-contracted for OSC 2 (display) but absolute for OSC 7
  (machine-readable)
- All controlled by `config.shell_integration.osc2` etc. toggles

## Experiments

### Experiment 1: Emit OSC 2 and OSC 7

#### Description

Add OSC 2 (title) and OSC 7 (cwd) to the REPL loop. These are the two most
impactful sequences — they fix the broken new-pane-cwd and missing title. Defer
OSC 133 (prompt markers) to a later experiment since reedline may already handle
parts of it.

#### Changes

**`src/repl.rs`** — add helper functions and emit calls:

`emit_osc7(cwd: &Path)`:

- Get hostname from `$HOSTNAME` env var, fall back to "localhost"
- Percent-encode the path (just encode control chars and spaces)
- Print `\x1b]7;file://{hostname}{path}\x1b\\` to stderr

`emit_osc2_idle(shell_name: &str, cwd: &Path)`:

- Tilde-contract the cwd
- Print `\x1b]2;[{shell_name}] {contracted_path}\x07` to stderr

`emit_osc2_command(shell_name: &str, cwd: &Path, command: &str)`:

- Tilde-contract the cwd
- Extract first word of command (the binary name)
- Print `\x1b]2;[{shell_name}] {contracted_path}> {binary}\x07` to stderr

Emit points in the REPL loop:

1. **Before each prompt** (top of loop): `emit_osc2_idle` + `emit_osc7`
2. **Before executing a command** (after user presses Enter, before
   `run_command`): `emit_osc2_command`
3. **After command completes** (after `run_command` returns): `emit_osc7` (cwd
   may have changed)

For AI mode: emit `emit_osc2_command` before the AI-suggested command runs, not
when the user types the question.

**Tilde contraction:** Extract the existing tilde contraction logic from
`ShannonPrompt` into a standalone function in `prompt.rs` so it can be reused by
both the prompt and OSC emission.

#### What about nushell mode?

When running nushell commands via `eval_source`, nushell might emit its own OSC
sequences if its shell_integration config is enabled. For now, we emit ours
regardless — double-emitting OSC 7 is harmless (terminal just gets the cwd
twice). If it becomes a problem, we can disable nushell's internal shell
integration.

#### Tests

No automated tests — OSC sequences are terminal-level and can't be verified
without a terminal emulator. Manual verification only.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes (no regressions).
3. Run shannon in ghostty:
   - Tab title shows `[nu] ~/project` (not blank/ghost icon).
   - Run `git status` — title changes to `[nu] ~/project> git`.
   - After command finishes — title returns to `[nu] ~/project`.
4. Run shannon in ghostty or wezterm:
   - `cd /tmp` in shannon.
   - Open a new pane/split — it opens in `/tmp` (not home dir).
5. Switch shells — title updates to `[bash] ~/project` etc.
6. AI mode — title shows command only when executing, not the question.

**Result:** Pass

All verification steps confirmed. Tab title shows shell name and cwd,
updates during command execution, and reverts to idle after. New panes
open in the correct directory. 76 tests pass, no regressions.

#### Conclusion

OSC 2 and OSC 7 are working. Terminal emulators now track shannon's cwd
and display meaningful titles.

## Conclusion

Issue complete. Shannon now emits OSC 2 (title) and OSC 7 (cwd) escape
sequences at the right points in the REPL loop. Terminal emulators can
track the working directory and show the active command in the tab title.

Key files:
- `src/repl.rs` — `emit_osc7`, `emit_osc2_idle`, `emit_osc2_command`
- `src/prompt.rs` — `tilde_contract` extracted as standalone function
