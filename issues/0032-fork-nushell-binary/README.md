+++
status = "open"
opened = "2026-03-26"
+++

# Issue 32: Rearchitect shannon as a fork of the nushell binary

## Goal

Research whether shannon should be restructured from "a shell that wraps
nushell" to "nushell, enhanced with brush and AI." Fork nushell's binary (not
just its library), and add mode switching to brush and AI from within nushell's
native REPL.

## Background

### Current architecture

Shannon has its own REPL (`src/repl.rs`) that uses reedline directly. It embeds
nushell and brush via their library APIs (`eval_source` and `run_string`). This
gives us command evaluation but loses everything the nushell binary provides:
terminal ownership, process groups, job control, signal handling, native
multiline editing, plugins, and more.

We've already forked nushell (`shannonshell/shannon_nushell`), brush
(`shannonshell/shannon_brush`), and reedline (`shannonshell/shannon_reedline`)
as submodules with renamed crates on crates.io.

### The problem

Embedding nushell via `eval_source` loses critical functionality:

- **Job control** — Ctrl+Z doesn't work. Nushell's job control requires terminal
  ownership and process group management that only the nushell binary sets up
  (via `terminal.rs` and `ForegroundChild`).
- **Signal handling** — We've worked around SIGINT with signal-hook, but the
  solution is fragile (double-registration workaround).
- **Multiline editing** — Nushell's REPL has proper multiline support with
  validation. Our REPL doesn't.
- **Completions** — Nushell has context-aware completions for its own commands.
  We use fish completions which don't know nushell syntax.
- **Plugins** — Nushell's plugin system doesn't work through `eval_source`.

### Proposed architecture

Shannon IS nushell — fork the nushell binary as shannon's entry point. Brush and
AI become modes within nushell's native REPL:

```
shannon (= modified nushell binary)
├── [nu] mode   — nushell's native REPL (default)
├── [brush] mode — commands routed to BrushEngine
└── [ai] mode    — messages routed to AiEngine
```

Shift+Tab switches modes. The prompt changes. In nushell mode, everything works
exactly as standalone nushell. In brush mode, the command goes through
`BrushEngine`. In AI mode, the message goes through `AiEngine`.

### What we gain

- Full job control (Ctrl+Z, fg, bg, jobs)
- Proper process groups and terminal management
- Nushell's native multiline editing and validation
- Nushell's context-aware completions
- Nushell's plugin system
- Nushell's configuration system (`config.nu`, `env.nu`)
- All signal handling done correctly by nushell
- Less code to maintain — nushell's REPL replaces ours

### What we lose / change

- Shannon's custom REPL (`src/repl.rs`) — replaced by nushell's
- Shannon's custom completer — replaced by nushell's (better)
- Shannon's custom highlighter — replaced by nushell's (better)
- Shannon's `config.toml` — may need to integrate with nushell's config
- Independence from nushell — deeper coupling to nushell's internals
- Shannon's current reedline keybinding setup — needs to be done via nushell's
  keybinding system instead

### Research questions

1. **Where is nushell's REPL loop?** Can it be modified to dispatch commands to
   different engines based on the active mode?
2. **How does nushell's keybinding system work?** Can we add Shift+Tab to switch
   modes without modifying reedline?
3. **How does nushell handle `eval_source` vs its REPL?** What's the difference
   between the two paths? What does the REPL do that `eval_source` doesn't?
4. **Can we keep shannon as a separate binary** that depends on nushell's
   crates, or do we need to literally fork nushell's `main.rs`?
5. **How does nushell's config system work?** Can we extend it with
   shannon-specific settings (brush, AI, toggle)?
6. **What's the migration path?** Can we do this incrementally, or is it a full
   rewrite?
7. **How do we integrate brush?** When in brush mode, where does the command go?
   Does it bypass nushell's parser entirely and route to `BrushEngine`? How does
   brush receive the raw command string before nushell tries to parse it?
8. **How do we integrate AI?** Same question — AI mode receives plain English,
   not nushell syntax. How do we intercept the input before nushell's parser?
9. **How do we handle Shift+Tab?** Nushell uses reedline keybindings. Can we add
   a custom keybinding that triggers a mode switch without modifying reedline?
   Does nushell's `ExecuteHostCommand` mechanism work for this?
10. **Syntax highlighting per mode** — Nushell highlights nushell syntax. When
    in brush mode, we need bash highlighting. When in AI mode, no highlighting.
    Can we swap the highlighter dynamically? Does nushell rebuild the editor on
    mode switch?
11. **Completions per mode** — Nushell has nushell-aware completions. Brush mode
    needs bash/file completions. AI mode needs no completions (or different
    ones). Can we swap the completer dynamically?
12. **Prompt per mode** — The prompt needs to show `[nu]`, `[brush]`, or `[ai]`.
    Can we change nushell's prompt dynamically from within the REPL loop?
13. **How do we support env.sh?** Shannon currently runs a bash script
    (`env.sh`) at startup to load PATH, API keys, and other env vars. This is
    critical — tutorials and AI always give instructions as "add this to your
    .bashrc." Shannon's `env.sh` lets users follow those instructions directly.
    Nushell uses `env.nu` (nushell syntax) instead. How do we preserve
    bash-based env loading in a nushell-based architecture? Options: run
    `env.sh` via brush at startup and inject the result into nushell's env, or
    source `.bashrc` via brush and propagate.

## Experiments

### Experiment 1: Research nushell internals

#### Description

Read the vendored nushell source code to answer all 13 research questions.

#### Findings

**1. Where is nushell's REPL loop?**

`nu-cli/src/repl.rs` — `evaluate_repl()` (line 71) sets up state, then calls
`loop_iteration()` in a loop (lines 188-247). Each iteration:

- Merges env from previous iteration
- Resets signals
- Evaluates hooks (pre_prompt, env_change, pre_execution)
- Sets up keybindings via `setup_keybindings()`
- Updates prompt via `update_prompt()`
- Calls `line_editor.read_line()` for input
- Parses with `parse_operation()` (auto-cd detection, etc.)
- Evaluates with `eval_source()` via `do_run_cmd()`
- Evaluates post-execution hooks

Before the loop, `run_repl()` in `src/run.rs` calls `setup_config()` to load
env.nu and config.nu. The binary's `main.rs` handles terminal acquisition,
process groups, and signal handlers.

**2. How does nushell's keybinding system work?**

Keybindings are defined in `$env.config.keybindings` (nushell config). Each
entry has `modifier`, `keycode`, `event`, `mode`, `name`. They're re-parsed
every REPL iteration via `create_keybindings()` in `reedline_config.rs`.

Custom keybindings can use `ExecuteHostCommand` to run arbitrary nushell code.
Shift+Tab can be added via config without modifying reedline:

```nushell
{ name: "switch_mode", modifier: "shift", keycode: "backtab",
  event: { send: "ExecuteHostCommand", cmd: "shannon-switch" },
  mode: ["emacs", "vi_insert", "vi_normal"] }
```

**3. eval_source vs REPL — what's different?**

`eval_source` is pure evaluation: parse → merge delta → eval_block → print. The
REPL adds: terminal ownership, process groups, signal handlers, env merging
between iterations, hooks (pre_prompt, pre_execution, post_execution), shell
integration (OSC sequences), prompt management, keybinding setup, history, and
reedline configuration. All of this is lost when embedding via `eval_source`
alone.

**4. Can shannon be a separate binary?**

Yes. Nushell's `main.rs` does: parse CLI args, init EngineState, set up signals
and terminal, load standard library, and call `run_repl()`. Shannon could
replicate this with modifications — add brush/AI engines, custom commands, and
mode switching. The heavy lifting is in `nu-cli`, which is a library crate.

**5. How does nushell's config system work?**

Config files load in order: default env → env.nu → config.nu → login.nu. The
`Config` struct in `nu-protocol` has fields for colors, completions,
keybindings, hooks, menus, etc. Unknown config fields are rejected. Plugin
configs use `$env.config.plugins.<name>` — this is the extensibility point.
Shannon-specific settings could live there or in a separate config file.

**6. Migration path**

Incremental. Shannon can start as a separate binary that calls nushell's
`evaluate_repl()` with a modified EngineState. The binary adds: brush engine, AI
engine, env.sh loading, mode switching commands. The nushell fork is already a
submodule. The key modification is to `loop_iteration()` — add a mode check
before evaluation to dispatch to brush or AI when active.

**7. How do we integrate brush?**

When in brush mode, intercept the command AFTER reedline returns it but BEFORE
nushell parses it. In `loop_iteration()`, after `read_line()` returns
`Signal::Success(line)`, check the active mode. If brush, send the line to
`BrushEngine` instead of `parse_operation()` / `do_run_cmd()`. The raw string
bypasses nushell's parser entirely.

**8. How do we integrate AI?**

Same mechanism as brush. When in AI mode, the raw input string goes to
`AiEngine` instead of nushell's parser. The response is printed directly. No
parsing, no evaluation by nushell.

**9. How does ExecuteHostCommand work?**

Keybindings can specify `{ send: "ExecuteHostCommand", cmd: "..." }`. The
command string is executed as nushell code when the key is pressed. This is how
nushell implements features like fzf history search. Shannon can use it for mode
switching — the command sets `$env.SHANNON_MODE` and the REPL loop checks this
variable each iteration.

**10. Syntax highlighting per mode**

The highlighter is recreated EVERY REPL iteration — `NuHighlighter` is
constructed fresh in `loop_iteration()` (line 400). It's a
`Box<dyn
Highlighter>`. In brush mode, substitute a bash highlighter. In AI mode,
substitute a no-op highlighter. The swap happens naturally because the
highlighter is rebuilt each iteration.

**11. Completions per mode**

Same as highlighting — `NuCompleter` is recreated every iteration (line 408).
It's a `Box<dyn ReedlineCompleter>`. In brush mode, substitute a file/command
completer. In AI mode, substitute a no-op or history completer. The swap is
trivial because completers are rebuilt each iteration.

**12. Prompt per mode**

The prompt is rebuilt every iteration via `update_prompt()`. It evaluates
`$env.PROMPT_COMMAND` closures. Shannon can set the prompt closure to check
`$env.SHANNON_MODE` and display `[nu]`, `[brush]`, or `[ai]` accordingly.
Changes take effect immediately on the next iteration.

**13. How do we support env.sh?**

Nushell loads config in order: default env → env.nu → config.nu. Shannon can
inject a step between default env and env.nu: run env.sh via brush, capture the
resulting env vars, and inject them into the stack via `add_env_var()`. This
happens in `setup_config()` in `config_files.rs`. The injected vars are then
available to env.nu and config.nu.

#### Result

Research complete. All 13 questions answered.

**Key finding: This architecture is viable and can be done incrementally.**

The critical insight: nushell rebuilds the highlighter, completer, and prompt
every single REPL iteration. This means mode switching is naturally supported —
check the mode, return the appropriate highlighter/completer/prompt, done. No
need to rebuild the entire editor or restart reedline.

The mode switch mechanism: a keybinding triggers `ExecuteHostCommand` which sets
`$env.SHANNON_MODE`. The next `loop_iteration()` checks this variable and
dispatches accordingly.

#### Conclusion

Rearchitecting shannon as a nushell fork is feasible. The main modification is
to `loop_iteration()` — add a mode check after `read_line()` to dispatch to
brush or AI instead of nushell's parser. Everything else (highlighter,
completer, prompt, keybindings) is already designed to be dynamic and
per-iteration. The env.sh feature can be preserved by injecting bash env vars
during config loading.
