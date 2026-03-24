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

## Experiments

### Experiment 1: Verify nushell already preserves internal state

#### Description

Test whether nushell's embedded engine already preserves internal state (shell
variables, custom commands) across shell switches. Since `NushellEngine` persists
for the session and `inject_state` only sets env vars and cwd, internal nushell
state should survive switching away and back.

#### Verification

1. Switch to nushell, set a variable: `let myvar = 5`
2. Verify it persists: `echo $myvar` → `5`
3. Switch to bash (Shift+Tab), then to zsh, then to fish, then back to nushell
4. Verify the variable survived: `echo $myvar` → `5`

**Result:** Pass

Nushell already preserves internal state across shell switches. The
`NushellEngine` struct (EngineState + Stack) lives for the entire session.
`inject_state` only updates env vars and cwd — it doesn't touch nushell's
internal variables, custom commands, or other Stack state.

No code changes needed for nushell.

#### Conclusion

Nushell's embedded architecture gives us per-shell state for free. Research
question 1 is answered: yes, it works already. Remaining question: can we
achieve the same for bash/fish/zsh?

### Experiment 2: Research bash persistent subprocess feasibility

#### Description

Read the vendored bash source code to understand whether shannon can keep a bash
process alive between commands, send commands one at a time, and capture state
after each command. The key question: can we replace the "spawn, run, capture,
exit" wrapper model with a persistent interactive bash subprocess?

#### Findings

**How bash reads commands:**

Bash's main loop (`eval.c:reader_loop()`) calls `read_command()` →
`parse_command()` → `yyparse()`. The parser reads character by character via
`shell_getc()` in `y.tab.c`. Multi-line commands (if/then/fi, heredocs) are
handled by the parser — it shows `PS2` and reads continuation lines until the
command is syntactically complete. Once complete, the command is immediately
executed.

**Interactive vs non-interactive:**

The critical difference: prompts and `PROMPT_COMMAND` only run in interactive
mode. The check is `SHOULD_PROMPT()` in `y.tab.c`:

```c
#define SHOULD_PROMPT() \
  (interactive && (bash_input.type == st_stdin || bash_input.type == st_stream))
```

Our current wrapper uses `bash -c "..."` which is non-interactive. A persistent
subprocess would need `bash -i` to get access to the hooks we need.

**Available hooks for command boundaries:**

- `PS0` (bash 4.4+): Expanded and printed to stderr *after* parsing but *before*
  execution. Good for "command received" signal.
- `PROMPT_COMMAND`: Runs *after* command execution and *before* the next prompt.
  This is the key hook for state capture — we could emit env vars here.
- `trap DEBUG`: Fires before each command. Less useful than PROMPT_COMMAND for
  our purpose.

The combination of `PS0` (pre-execution marker) and `PROMPT_COMMAND`
(post-execution state capture) gives us command boundary detection.

**The fundamental problem: PTY requirement.**

For bash to be truly interactive (enabling `PROMPT_COMMAND`, `PS0`, signal
handling), it needs a TTY. The parent process would need to:

1. Create a pseudo-terminal (PTY) pair
2. Spawn bash with the slave end as its controlling terminal
3. Write commands to the master end
4. Read output from the master end
5. Parse output to separate user-visible output from state-capture markers
6. Handle terminal window size, raw mode, and signal forwarding

This is essentially what `expect`, `screen`, and `tmux` do. It's a significant
amount of complexity.

**The synchronization problem:**

Even with a PTY, the parent needs to know when a command is done:

1. Parent writes command to master PTY
2. Bash receives it, `PS0` fires (marker on stderr: "command starting")
3. Command runs, output goes to terminal
4. Command finishes, `PROMPT_COMMAND` fires (captures env to temp file or emits
   state on stderr)
5. Bash prints `PS1` prompt
6. Parent detects the prompt and reads captured state

This requires parsing the output stream to find markers mixed in with command
output. Fragile — a command could print text that matches the markers.

**Alternative: the wrapper model is actually well-suited.**

The current wrapper model sidesteps all of these problems:

- No PTY needed (child inherits shannon's terminal directly)
- No output parsing (command output goes straight to the user)
- Clear command boundaries (subprocess starts, runs, exits)
- State capture via temp file (clean separation from output)

The trade-off is: internal bash state (shell variables, aliases, functions) is
lost between commands.

**What internal state matters in practice?**

- **Shell variables** (`FOO=bar` without `export`): Rare in interactive use.
  Most users export variables they care about.
- **Aliases**: Typically set in `.bashrc`, which we already source via init
  files.
- **Functions**: Same as aliases — usually in dotfiles.
- **Shell options** (`set -o`, `shopt`): Rarely changed interactively.
- **Directory stack** (`pushd`/`popd`): Would be lost. Uncommon in practice.

The most impactful loss is aliases and functions defined during the session, but
this is an edge case.

#### Result

Research complete. Findings documented above.

**Persistent bash subprocess:** Feasible but requires PTY management, output
parsing, and marker-based synchronization. High complexity, fragile.

**Current wrapper model:** Loses internal state but is simple, robust, and
handles all the hard problems (stdio, signals, env capture) cleanly.

#### Conclusion

A persistent bash subprocess is technically possible using PTY + `PROMPT_COMMAND`
for state capture, but the complexity is high and the benefits are marginal. The
internal state that would be preserved (shell variables, aliases, functions set
during the session) is rarely needed in practice — most of it comes from
dotfiles which are already sourced.

The current wrapper model is a good design for bash/fish/zsh. The nushell
embedded model is where per-shell state really shines because it's free.

Next step: decide whether the PTY approach is worth pursuing, or if there's a
lighter-weight alternative (e.g., persisting specific state like aliases between
wrapper invocations by capturing and re-sourcing them).

### Experiment 3: Research embedding brush (Rust-based bash) like nushell

#### Description

[Brush](https://github.com/reubeno/brush) is a Rust reimplementation of bash.
Since it's written in Rust, it might be embeddable as a library — the same way
we embed nushell via `eval_source()`. This could give us persistent bash state
without PTY complexity.

Research questions:

1. **Crate structure:** Does brush expose a library crate? What's the public
   API? Can we create a shell instance, send it commands, and read state back?
2. **State model:** How does brush represent env vars, shell variables, cwd, and
   exit code? Can we read/write these between commands?
3. **Stdio:** When brush runs a command, does output go to the real terminal?
   Can interactive programs (vim, less, ssh) work?
4. **External commands:** Does brush spawn real subprocesses for external
   commands (ls, git, npm)? Or does it try to interpret everything internally?
5. **Compatibility:** How complete is brush's bash compatibility? Can it run
   real-world commands reliably?
6. **Signal handling:** Does brush handle SIGINT for subprocesses? How does it
   interact with our signal-hook setup?
7. **Integration pattern:** What would a `BrushEngine` (analogous to
   `NushellEngine`) look like? What crates would we depend on?

#### Method

Read the vendored brush source at `vendor/brush/`. Examine:

- `Cargo.toml` files for crate structure and public API surface
- `brush-core/` or similar for the shell engine
- How commands are evaluated (eval loop, AST execution)
- How env/variables/cwd are stored and accessible
- How external commands are spawned
- Any existing examples of embedding brush as a library
