+++
status = "open"
opened = "2026-03-24"
+++

# Issue 24: Per-shell internal state with env propagation on switch

## Goal

Research whether shannon can maintain internal state per shell (nushell keeps
its Stack, bash/fish/zsh keep their own state) while propagating environment
variables when switching between shells. Determine feasibility for both the
embedded path (nushell) and the wrapper path (bash/fish/zsh).

## Background

### Current architecture

Shannon maintains a single `ShellState` (env vars, cwd, exit code) that is
shared across all shells. When the user runs a command, shannon injects this
state into the active shell and captures the updated state after execution. This
is the "strings only" boundary — only env vars, cwd, and exit code cross between
shells.

This works but has a limitation: each shell loses its internal state between
commands. Nushell's Stack is rebuilt from scratch each time. Bash doesn't
remember shell variables (non-exported), aliases set during the session, or
shell options.

### Desired architecture

Each shell maintains its own persistent internal state across commands:

- **Nushell:** The `EngineState` + `Stack` already persist across commands (the
  `NushellEngine` struct lives for the session). Nushell variables, custom
  commands, and internal state survive between commands.
- **Bash/fish/zsh:** Currently each command spawns a new subprocess. Internal
  state (shell variables, aliases, functions, options) is lost between commands.

When the user switches shells (Shift+Tab or `/switch`), environment variables
from the previous shell are propagated to the next shell. Internal state stays
with each shell.

### What "internal state" means per shell

**Nushell:** Variables (`$foo`), custom commands (`def`), modules, overlays.
These live in the Stack/EngineState and are already persistent.

**Bash:** Shell variables (non-exported), aliases, functions, shell options
(`set -o`, `shopt`), directory stack (`pushd`/`popd`).

**Fish:** Universal variables, abbreviations, functions defined in session.

**Zsh:** Shell variables, aliases, functions, options (`setopt`), named
directories.

### The two research questions

1. **Nushell (embedded):** Already has persistent state. When switching away
   from nushell, can we extract just the env vars (not internal nushell state)
   to propagate? When switching back, can we inject env vars without disturbing
   nushell's internal state? This likely already works — `inject_state` sets env
   vars and cwd, and nushell's Stack preserves everything else.

2. **Bash/fish/zsh (wrapper):** Currently each command is a new subprocess.
   Internal state is lost. To preserve it, we'd need a persistent subprocess (a
   long-running shell process that we send commands to). This is a fundamental
   change from the current "spawn, run, capture, exit" model. Is this feasible?
   What are the trade-offs? How would env capture work? How would stdio work
   (the user needs to see output and interact with programs)?
