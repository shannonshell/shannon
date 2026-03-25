# Shannon

An AI-first shell with seamless access to bash, nushell, and any other shell.
The default mode accepts plain English (via a configurable LLM), and Shift+Tab
switches to traditional shells. Named after Claude Shannon.

## Build

```sh
cd shannon
cargo build
cargo run
```

The Rust crate lives in the `shannon/` subdirectory. All cargo commands
run from there.

## Architecture

Shannon uses reedline as its line editor. Nushell is embedded as a library via
`eval_source()` from the `nu-cli` crate. Bash is embedded via the brush crate's
`Shell` API. Fish and zsh run as subprocesses via wrapper scripts.

### Source files (under `shannon/`)

- `src/main.rs` — entry point, startup sequence
- `src/repl.rs` — main REPL loop, shell switching, AI mode, OSC integration
- `src/lib.rs` — re-exports modules for integration tests
- `src/config.rs` — TOML config loading, built-in shell definitions, AI config
- `src/shell.rs` — `ShellState` (env, cwd, exit code), config directory helpers
- `src/executor.rs` — subprocess spawning, wrapper templates, env capture parsing
- `src/nushell_engine.rs` — embedded nushell via `EngineState` + `eval_source()`
- `src/brush_engine.rs` — embedded bash via brush `Shell` + tokio async runtime
- `src/prompt.rs` — custom reedline `Prompt` impl, tilde contraction
- `src/highlighter.rs` — tree-sitter syntax highlighting with Tokyo Night colors
- `src/completer.rs` — `ShannonCompleter` combining command + file completion
- `src/completions.rs` — fish completion table (loaded from build-time JSON)
- `src/ai/` — AI mode: provider (rig-core), prompt builder, sessions, translation

### How command execution works

**Bash/fish/zsh (wrapper model):**
1. User types a command
2. Shannon wraps it in a shell-specific template that captures env + cwd
3. Subprocess runs with inherited stdio
4. After exit, shannon reads captured state from a temp file
5. State is injected into the next command's subprocess

**Brush (embedded bash via brush crate):**
1. User types a command
2. Shannon calls `shell.run_string()` via tokio `block_on`
3. Output goes directly to the terminal
4. Shannon reads env vars from `shell.env().iter_exported()` and cwd from
   `shell.working_dir()` after evaluation

**Nushell (embedded model):**
1. User types a command
2. Shannon calls `eval_source()` directly via the nushell crate API
3. Output goes directly to the terminal (auto-print, vim, etc. all work)
4. Shannon reads env vars and cwd from the nushell `Stack` after evaluation

### Testing

Every new feature must include tests. No feature ships without test coverage.

- **Unit tests** go in each module as `#[cfg(test)] mod tests { ... }`.
- **Integration tests** go in `tests/`.
- Use `tempfile::TempDir` for tests that need filesystem fixtures.
- `cargo test` must pass before a feature is considered done.

### Key design decisions

- **Strings only** — only env vars (strings), cwd, and exit code cross the
  shell boundary. No shell-internal data structures.
- **Nushell and brush embedded, others wrapped** — nushell and brush (bash)
  run natively via crate APIs. Bash, fish, and zsh also available as subprocess
  wrappers.
- **Config-driven shells** — shells are defined in `config.toml` with wrapper
  templates. Adding a new shell requires no code changes.
- **Fish completions baked in** — 983 commands parsed from fish completion
  files at build time, available in all shell modes.
- **Vendor directory is for reference only** — vendored repos are for reading
  source code, not for building against. Use crates.io dependencies.

## Modes

Shannon has two modes, toggled via the `/ai` command:

- **Normal mode** — commands go to the active shell (bash, brush, nushell,
  fish, zsh)
- **AI mode** — input goes to an LLM which generates a shell command. User
  confirms before execution. Use `/ai on`, `/ai off`, or `/ai toggle`.

Shell switching (Shift+Tab) works in both modes.

## Config

Shannon uses `~/.config/shannon/` (respects `XDG_CONFIG_HOME`):

- `config.toml` — shell rotation (`toggle`), custom shells, AI provider/model
- `env.sh` — bash script for PATH, env vars, API keys (runs once at startup)
- `history.db` — SQLite command history (shared across all shells and instances)

## Issues and Experiments

Every significant piece of work gets an issue in `issues/`. Issues describe the
problem, provide background, and propose solutions. Experiments are the
incremental steps that solve the problem.

### Issue Structure

Each issue is a **folder** containing a `README.md` with TOML frontmatter:

```
issues/0001-tree-sitter-highlighting/
├── README.md          ← main issue document with frontmatter
├── 01-bash-grammar.md ← optional: additional files for long issues
└── 02-nushell-grammar.md
```

The folder name is `{number}-{slug}`. The number is globally sequential. The
slug is lowercase, hyphenated, and describes the topic.

#### Frontmatter

Every `README.md` starts with TOML frontmatter:

```
+++
status = "open"
opened = "2026-03-21"
+++
```

Or for closed issues:

```
+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-22"
+++
```

#### README.md structure

After the frontmatter, a new issue has these sections:

1. **Title** (H1) — `# Issue {N}: {descriptive title}`
2. **Goal** — One or two sentences describing the desired outcome.
3. **Background** — Context, prior work, constraints.
4. **Architecture** / **Analysis** / **Proposed Solutions** — Technical details.

A new issue does **not** have an Experiments section yet.

#### Additional files

For long issues, split experiments or sub-topics into numbered files:
`01-name.md`, `02-name.md`, etc. Link them from the README.md. Keep each file
under ~1000 lines to fit in an AI agent's context window.

### Multiple Open Issues

Multiple issues can be open at the same time. This allows interleaving work —
a large issue can stay open while smaller issues are opened and closed alongside
it.

### Experiments

#### When to create an experiment

Only after the issue's requirements are clear. Each experiment is designed,
implemented, and concluded before the next one is designed.

**Never list experiments upfront.** The outcome of each experiment informs what
comes next.

#### Experiment structure

Each experiment has:

1. **Title** (H3) — `### Experiment {N}: {descriptive title}`
2. **Description** — What and why.
3. **Changes** — Specific code changes, listed by file.
4. **Verification** — How to test. Concrete steps and pass/fail criteria.

#### One at a time

Design and implement one experiment at a time. The result of Experiment 1
directly informs what Experiment 2 should be.

#### Recording results

After testing, add a result below the verification section:

```markdown
**Result:** Pass / Partial / Fail

{description}

#### Conclusion

{what we learned, what to do next}
```

All three outcomes are valuable. Failed experiments eliminate dead ends.

### Closing an Issue

Add a `## Conclusion` section after the last experiment. Update the frontmatter
to `status = "closed"` with a `closed` date.

### Immutability

Closed issues are historical records. They are **immutable** and must NEVER be
modified. History stays as it was written.

### Process Summary

1. **Create the issue** — `issues/{number}-{slug}/README.md` with frontmatter,
   goal, background. No experiments yet.
2. **Design Experiment 1** — Add `## Experiments` and `### Experiment 1`.
3. **Implement Experiment 1** — Write the code.
4. **Record the result** — Pass, partial, or fail with a conclusion.
5. **Repeat** — Design the next experiment. Continue until the goal is met.
6. **Close the issue** — Write the `## Conclusion`, update frontmatter.
