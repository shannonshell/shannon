+++
status = "open"
opened = "2026-03-22"
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
