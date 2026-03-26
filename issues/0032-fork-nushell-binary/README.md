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
