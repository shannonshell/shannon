# olshell

A poly-shell that wraps multiple shell interpreters and lets you switch between
them mid-session using Shift+Tab.

## Build

```sh
cargo build
cargo run
```

## Architecture

olshell uses reedline (from crates.io) as its line editor. Each command spawns a
fresh subprocess — there are no persistent shell sessions.

### Source files

- `src/main.rs` — entry point, reedline loop, Shift+Tab shell switching
- `src/shell.rs` — `ShellKind` enum (Bash/Nushell), `ShellState` (env, cwd, exit code)
- `src/executor.rs` — subprocess spawning, wrapper scripts, env capture parsing
- `src/prompt.rs` — custom reedline `Prompt` impl showing active shell + cwd

### How command execution works

1. User types a command
2. olshell wraps it in a shell-specific script that captures env vars + cwd after execution
3. Subprocess runs with inherited stdio (output streams directly to terminal)
4. After exit, olshell reads captured state from a temp file
5. State (env vars, cwd, exit code) is injected into the next command's subprocess

### Key design decisions

- **Strings only** — only env vars (strings), cwd, and exit code cross the shell boundary. No shell-internal data structures.
- **One subprocess per command** — no persistent shell sessions. Type `bash` or `nu` for a full interactive session.
- **Vendor directory is for reference only** — vendored repos are for reading source code, not for building against. Use crates.io dependencies in Cargo.toml.
- **Nushell output rendering** — nushell's `echo` returns a Value rather than printing. The wrapper uses try/catch + explicit `print` to render output.

## Shells supported

Currently: bash, nushell. The architecture supports any shell — adding one means adding a wrapper script builder and an env parser in `executor.rs`.

## Config

History files are stored in `~/.config/olshell/` (per-shell).

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
