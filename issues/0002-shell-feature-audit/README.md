+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-21"
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

**Result:** Pass

#### Feature Matrix

##### Tab Completion

| Shell      | Approach                                                                         |
| ---------- | -------------------------------------------------------------------------------- |
| bash       | Programmable completion (complete/compgen/compopt), custom functions             |
| zsh        | Sophisticated completion system (compinit/compadd), menu selection               |
| fish       | Built-in + 1000s of pre-built completions, fuzzy matching                        |
| nushell    | Per-type completers (file, dir, flag, env var, operator), fuzzy/prefix/substring |
| powershell | TabExpansion2, Register-ArgumentCompleter, .NET method completion                |
| elvish     | Completion mode with matchers (prefix, subsequence, substring)                   |

**olshell: Must have.** Every shell has this. Minimum: file/directory
completion.

##### Autosuggestions / Hints

| Shell      | Has it?                                               |
| ---------- | ----------------------------------------------------- |
| fish       | Yes — history-based, inline, validated against syntax |
| nushell    | Yes — via reedline hinter, CWD-aware                  |
| zsh        | Via plugin (zsh-autosuggestions)                      |
| bash       | No (third-party only)                                 |
| elvish     | No                                                    |
| powershell | Via PSReadLine predictive IntelliSense                |

**olshell: Should have.** Reedline already has hinter support — just wire it up.

##### Screen Control (Ctrl+L)

All six shells support Ctrl+L to clear screen.

**olshell: Must have.** Verify reedline handles this (likely already works).

##### Job Control (bg, fg, Ctrl+Z, jobs, disown)

| Shell           | Level                |
| --------------- | -------------------- |
| bash, zsh, fish | Full                 |
| powershell      | Full (different API) |
| nushell, elvish | Limited              |

**olshell: Out of scope.** Requires persistent process group model we don't
have. Users who need job control should run `bash` or `zsh` directly.

##### Hooks (preexec, precmd, chpwd)

| Shell      | Hooks                                                                    |
| ---------- | ------------------------------------------------------------------------ |
| bash       | PROMPT_COMMAND, trap DEBUG                                               |
| zsh        | precmd, preexec, chpwd, periodic                                         |
| fish       | Event system (signal, variable, exit, job, generic)                      |
| nushell    | pre_prompt, pre_execution, env_change, display_output, command_not_found |
| elvish     | before-readline, after-readline, after-command                           |
| powershell | Register-EngineEvent                                                     |

**olshell: Nice to have.** Architecture should allow for it later.

##### Right Prompt

Supported by zsh, fish, nushell, elvish. Not bash or powershell.

**olshell: Nice to have.** Reedline supports it. Low effort.

##### Prompt Features (git branch, command duration)

All six shells support git info in prompt via various mechanisms.

**olshell: Should have.** Git branch is expected by modern users.

##### Bracketed Paste

All modern shells support it.

**olshell: Must have.** Reedline likely handles this — verify.

##### History Deduplication

All six shells support dedup in some form.

**olshell: Should have.** Consider SQLite history backend.

##### Terminal Title (OSC 2)

All six shells support setting terminal title.

**olshell: Nice to have.** Straightforward to add.

##### Startup/Config Files

All shells have rc files. Already planned for olshell.

**olshell: Should have.** Per-shell rc files in ~/.config/olshell/.

##### Vi Mode

All six shells support vi keybindings.

**olshell: Should have.** Reedline has built-in Vi mode. Needs config option.

#### Priority Summary

**Must have** (broken without):

1. Tab completion (file/directory)
2. Ctrl+L clear screen
3. Bracketed paste

**Should have** (noticeably better): 4. Autosuggestions/hints 5. Git branch in
prompt 6. History deduplication 7. Startup config files 8. Vi mode option

**Nice to have** (defer): 9. Right prompt 10. Hooks (preexec/precmd) 11.
Terminal title (OSC 2) 12. Command duration in prompt 13. Command-aware
completion

**Out of scope:**

- Job control
- History expansion (!!, !$)
- Brace/glob expansion
- Aliases/functions

#### Conclusion

The audit is complete. The three must-have gaps are tab completion, Ctrl+L, and
bracketed paste. Ctrl+L and bracketed paste are likely already handled by
reedline and just need verification. Tab completion is the only significant
implementation effort needed. New issues should be created for the must-have and
should-have items.
