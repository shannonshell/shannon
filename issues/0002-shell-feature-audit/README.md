+++
status = "open"
opened = "2026-03-21"
+++

# Issue 2: Shell feature audit

## Goal

Identify all the features users expect from an interactive shell, assess which
ones olshell already has, and create issues for the ones we still need. The
output is a prioritized checklist that becomes our roadmap.

## Background

olshell is not a new shell language — it delegates to real shells. But it still
owns the interactive experience: the prompt, input editing, keybindings,
history, tab completion, job control, and everything else that happens between
the user pressing a key and a subprocess being spawned.

Users coming from bash, zsh, fish, or nushell will expect certain baseline
features to just work. If olshell is missing something fundamental (like tab
completion or Ctrl+L to clear screen), it will feel broken regardless of how
well the shell-switching works.

### What we already have

| Feature                       | Status | Notes                        |
| ----------------------------- | ------ | ---------------------------- |
| Command input via reedline    | Done   | Emacs keybindings            |
| Per-shell command history     | Done   | FileBackedHistory, up arrow  |
| Ctrl+R reverse history search | Done   | Built into reedline          |
| Shift+Tab shell switching     | Done   | Core feature                 |
| Syntax highlighting           | Done   | Tree-sitter, Tokyo Night     |
| Environment variable sync     | Done   | Captured via wrapper scripts |
| Working directory sync        | Done   | Captured via wrapper scripts |
| Exit code propagation         | Done   | Shown in prompt indicator    |
| Visual shell indicator        | Done   | `[bash]` / `[nu]` in prompt  |
| Ctrl+C interrupt              | Done   | During input and subprocess  |
| Ctrl+D exit                   | Done   | Exits olshell                |

### What we need to audit

The experiment for this issue is a research task: survey what features
interactive shells provide, categorize them, and determine which ones olshell
needs. For each feature, decide:

1. **Must have** — users will consider olshell broken without it.
2. **Should have** — noticeably better experience, worth implementing soon.
3. **Nice to have** — can defer, but should be on the roadmap.
4. **Out of scope** — belongs to the sub-shell, not olshell.

### Categories to investigate

- **Input editing** — tab completion, multi-line editing, brace expansion, glob
  expansion, word movement (Alt+B/F), kill ring (Ctrl+K/Y)
- **History** — persistent history, history expansion (!!, !$), shared history
  across sessions, history deduplication
- **Screen control** — Ctrl+L clear screen, scrollback behavior
- **Job control** — background jobs (&), fg/bg, Ctrl+Z suspend, job list
- **Navigation** — cd shortcuts (cd -, pushd/popd, autojump-style), directory
  stack
- **Prompt** — git branch display, command duration, timestamp
- **Aliases and functions** — per-shell config files (already planned in README)
- **Startup/shutdown** — rc files, login vs non-login, MOTD
- **Terminal integration** — title bar updates, OSC sequences, clipboard
- **Globbing and expansion** — does olshell need to expand anything, or does the
  sub-shell handle it all?

## Experiments

### Experiment 1: Audit interactive features across vendored shells

#### Description

Research the interactive features provided by all six vendored shells (bash,
zsh, fish, nushell, powershell, elvish). Focus exclusively on the **interactive
layer** — features that belong to the shell's line editor, session management,
and terminal integration. Ignore language features (syntax, control flow, data
types) since those are handled by the sub-shell.

For each shell, examine:

1. **Line editor capabilities** — what keybindings, completion, and editing
   features does their input handler provide?
2. **Tab completion** — how does it work? File completion, command completion,
   argument-aware completion? What triggers it?
3. **History features** — beyond basic up/down, what history features exist?
   Substring search, deduplication, per-directory history, timestamps?
4. **Screen/terminal control** — clear screen, scrollback, terminal title, OSC
   sequences, bracketed paste?
5. **Job control** — background jobs, fg/bg, suspend, job list, disown?
6. **Hooks and events** — preexec, precmd, chpwd, command-not-found handlers?
7. **Prompt features** — right prompt, transient prompt, async prompt, git
   integration?
8. **Startup** — rc files, profile files, env files, login vs interactive
   distinction?

#### Method

Use the vendored source repos to identify features. For each shell, look at:

- README / documentation files for feature lists
- Line editor / input handling source code
- Completion system source code
- Key binding definitions

Record findings in a comparison matrix.

#### Changes

No code changes. Output is a feature matrix documented in this issue as the
experiment result.

#### Verification

1. Feature matrix covers all six shells across all categories.
2. Each feature is classified as: must have / should have / nice to have / out
   of scope for olshell.
3. Must-have features that olshell is missing are identified clearly.
4. New issues are created for each missing must-have and should-have feature.
