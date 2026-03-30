# Shannon

An AI-first shell built on nushell, with seamless bash compatibility via brush.
Shift+Tab cycles between nu and bash modes.
Named after Claude Shannon.

## Build

```sh
cd shannon
cargo build
cargo run
```

The Rust crate lives in the `shannon/` subdirectory. All cargo commands
run from there.

## Architecture

Shannon IS nushell — it copies the nushell binary source code and adds mode
dispatch for bash (via brush crate). Nushell's REPL handles terminal ownership,
process groups, job control, signal handling, multiline editing, completions,
and all interactive features. Shannon adds a `ModeDispatcher` trait (defined
in nu-cli) that intercepts commands when the mode is not "nu".

### Repo structure

```
shannon/              (main repo)
├── shannon/          (shannonshell crate — binary + library)
├── nushell/          (submodule → shannonshell/shannon_nushell)
├── brush/            (submodule → shannonshell/shannon_brush)
├── reedline/         (submodule → shannonshell/shannon_reedline)
├── vendor/           (reference source code, gitignored)
├── issues/           (issue tracking with experiments)
└── scripts/          (build, install, release)
```

### Submodules (forked dependencies)

- **nushell** — fork of `nushell/nushell`, renamed crates to `shannon-nu-*`.
  Changes: `ModeDispatcher` trait in nu-cli, `BashHighlighter` in nu-cli,
  Shift+Tab keybinding, config dir `~/.config/shannon/`, relaxed libc pin.
- **brush** — fork of `reubeno/brush`, renamed crates to `shannon-brush-*`.
  Changes: crate renames only (API already has what we need on main).
- **reedline** — fork of `nushell/reedline`, renamed to `shannon-reedline`.
  Changes: crate rename only.

### Source files (under `shannon/src/`)

**Copied from nushell binary (the shell):**
- `main.rs` — entry point, startup sequence (from nu binary, modified)
- `run.rs` — `run_repl()` wrapper, creates ModeDispatcher, shows banner
- `command.rs` — CLI argument parsing
- `command_context.rs` — registers all nushell commands
- `config_files.rs` — loads env.nu, config.nu, login.nu
- `signals.rs` — Ctrl+C handler via ctrlc crate
- `terminal.rs` — Unix terminal/process group acquisition
- `logger.rs` — logging setup
- `ide.rs` — IDE/LSP features
- `experimental_options.rs` — nushell experimental feature flags
- `test_bins.rs` — nushell test utilities

**Shannon-specific (engines and dispatch):**
- `lib.rs` — library exports for the shannonshell crate
- `dispatcher.rs` — `ShannonDispatcher` implementing `ModeDispatcher`
- `brush_engine.rs` — `BrushEngine` (embedded bash via brush crate)
- `shell_engine.rs` — `ShellEngine` trait
- `shell.rs` — `ShellState` (env, cwd, exit code), config directory helpers
- `executor.rs` — startup script (`env.sh`) execution via bash

### How command execution works

**Nu mode (default):**
1. User types a command
2. Nushell's `loop_iteration()` reads input via reedline
3. `$env.SHANNON_MODE` is "nu" — falls through to nushell's parser/evaluator
4. Nushell handles everything: parsing, execution, output, env updates

**Bash mode (bash):**
1. User types a command
2. Nushell's `loop_iteration()` reads input via reedline
3. `$env.SHANNON_MODE` is "bash" — calls `ModeDispatcher::execute()`
4. Env vars converted to strings via `env_to_strings()`
5. `BrushEngine::execute()` runs the command via brush's `run_string()`
6. Result (env vars, cwd, exit code) written back to nushell's Stack
7. Nushell's REPL continues with updated state

### Environment propagation

When switching modes, all exported environment variables and cwd are preserved:
- **Nu → Brush:** `env_to_strings()` converts nushell typed values to strings
  (using `ENV_CONVERSIONS` `to_string` closures for PATH etc.)
- **Brush → Nu:** String env vars written back to Stack via `add_env_var()`.
  Nushell's REPL automatically applies `from_string` conversions on the next
  iteration.

### Testing

Every new feature must include tests. No feature ships without test coverage.

- **Unit tests** go in each module as `#[cfg(test)] mod tests { ... }`.
- **Integration tests** go in `tests/`.
- Use `tempfile::TempDir` for tests that need filesystem fixtures.
- `cargo test` must pass before a feature is considered done.

### Key design decisions

- **Shannon IS nushell** — the binary copies nushell's startup code (~4,600
  lines) and adds mode dispatch. Shannon gets all nushell features for free:
  job control, plugins, multiline editing, completions, hooks, etc.
- **Trait injection** — `ModeDispatcher` trait defined in nu-cli, implemented
  by `ShannonDispatcher` in shannon. Nushell's fork has a ~30-line hook in
  `loop_iteration()` that dispatches to bash when `$env.SHANNON_MODE` is
  "bash". Shannon stays the primary binary; nushell stays a dependency.
- **Strings at the boundary** — env vars cross between shells as strings.
  `env_to_strings()` and `from_string` conversions handle typed values
  (PATH as list, etc.).
- **Forked dependencies** — nushell, brush, reedline are forked as submodules
  with renamed crates (`shannon-nu-*`, `shannon-brush-*`, `shannon-reedline`)
  published to crates.io. Upstream sync via `git rebase upstream/main` on
  the `shannon` branch.
- **Vendor directory is for reference only** — vendored repos are for reading
  source code, not for building against.

## Modes

Shannon has two modes, cycled via Shift+Tab:

- **nu** — nushell (native, default)
- **bash** — bash-compatible via brush crate

Mode is stored in `$env.SHANNON_MODE`. Shift+Tab sends `__shannon_switch`
via reedline's `ExecuteHostCommand`, which cycles the mode.

Each mode gets appropriate syntax highlighting:
- Nu mode: `NuHighlighter` (nushell's native highlighter)
- Bash mode: `BashHighlighter` (tree-sitter-bash, Tokyo Night colors)

## Config

Shannon uses `~/.config/shannon/` (respects `XDG_CONFIG_HOME`):

- `env.sh` — bash script for PATH, env vars, API keys (runs first via brush)
- `env.nu` — nushell env setup (runs after env.sh)
- `config.nu` — nushell config (keybindings, colors, hooks, etc.)
- `login.nu` — login shell config
- `history.sqlite3` — SQLite command history

Config is nushell-native. Shannon adds no custom configuration beyond what
nushell provides. All shell settings (keybindings, colors, hooks, completions)
are configured via nushell's `config.nu`.

The `env.sh` feature is critical — it lets users follow bash-style setup
instructions ("add this to your .bashrc") directly. Shannon sources it via
brush at startup and injects the resulting env vars into nushell's Stack.

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
