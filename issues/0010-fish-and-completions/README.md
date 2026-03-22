+++
status = "open"
opened = "2026-03-22"
+++

# Issue 10: Add fish shell support and adopt fish completions

## Goal

Two related goals:

1. Add fish as a supported shell in shannon's Shift+Tab rotation, alongside bash
   and nushell.
2. Adopt fish's 1,055 community-maintained completion definitions for use in
   shannon — providing command-aware tab completion (subcommands, flags,
   descriptions) for all supported shells, not just fish.

## Background

### Fish as a supported shell

Shannon currently supports bash and nushell. Adding fish follows the same
pattern: a wrapper script in `executor.rs` that runs the command, captures env
vars, cwd, and exit code, then parses the output. Fish uses a different env dump
format than bash or nushell, so we need a new wrapper and parser.

Fish is widely installed (available via `brew install fish`) and is the third
most popular interactive shell after bash and zsh. Adding it to shannon's
rotation makes shannon more useful.

What's needed:

- Add `Fish` variant to `ShellKind` enum in `shell.rs`
- Add a fish wrapper script builder in `executor.rs`
- Add a fish env parser in `executor.rs`
- Add fish to the shell detection list in `main.rs`
- Add tree-sitter-fish grammar for syntax highlighting (if one exists)

### Fish completions for all shells

Fish ships 1,055 completion definition files covering most common commands (git,
docker, cargo, npm, ssh, curl, etc.). These files use a simple declarative
format:

```fish
complete -c git -n __fish_git_needs_command -a commit -d 'Record changes'
complete -c git -n '__fish_git_using_command commit' -l message -s m -d 'Commit message'
```

Research from our earlier investigation found that 74% of these completions
(24,077 of 32,401 statements) can be parsed into static lookup tables at build
time. This covers subcommands, flags, and descriptions for 1,000+ commands.

The completions would be used by shannon's `Completer` implementation for all
shells — not just fish. When a user types `git` and presses Tab in bash or
nushell mode, they'd see git's subcommands.

### Completion file management

Fish's completions are actively maintained (last update: 2026-03-14). We need a
reproducible way to:

1. Copy the completion files from fish's repo into our project
2. Update them when we want newer completions
3. Parse them at build time into a Rust data structure
4. Bake the parsed data into the shannon binary

Proposed approach:

- A `scripts/update-completions.sh` script that copies
  `vendor/fish/share/completions/*.fish` into a `completions/` directory in our
  repo (checked into git, so builds don't need fish vendored)
- A build-time step (build.rs or a separate script) that parses the `.fish`
  files into a static lookup table
- The parsed table is compiled into the binary — zero runtime cost

### Completion parsing scope

What we parse (74% of statements):

- Static subcommands and flags with descriptions
- Completions gated by `__fish_use_subcommand` (no subcommand typed yet)
- Completions gated by `__fish_seen_subcommand_from` (specific subcommand
  active)

What we skip for now (26% of statements):

- Tool-specific condition functions (`__fish_git_using_command`, etc.)
- Dynamic completions that require running commands at tab-time
- These can be added incrementally later

### How completions integrate with reedline

Shannon's `FileCompleter` currently handles file/directory completion. We need
to extend or replace it with a completer that:

1. Checks if the current word is a command argument (not the first word)
2. Looks up the command in the static completion table
3. Returns subcommands or flags based on context (is a subcommand needed? which
   subcommand is active?)
4. Falls back to file completion when no command-specific completions match

This requires implementing the two generic fish condition functions in Rust:

- "needs subcommand" — no non-flag argument after the command name
- "seen subcommand from" — a specific subcommand has been typed

## Experiments

### Experiment 1: Add fish as a supported shell

#### Description

Add fish to shannon's Shift+Tab rotation following the same pattern as bash and
nushell. This is the simpler of the two tracks and gives us fish support before
tackling completions.

Fish's env can be captured via the `env` command (standard `KEY=VALUE` per
line), which is simpler than bash's `declare -x` or nushell's JSON. The `env`
command is a POSIX utility available everywhere, and fish can call it directly.

A `tree-sitter-fish` grammar exists on crates.io (v3.6.0) for syntax
highlighting.

#### Changes

**`Cargo.toml`** — add `tree-sitter-fish = "3.6"`.

**`src/shell.rs`** — add `Fish` variant to `ShellKind`:

- `display_name()` → `"fish"`
- `binary()` → `"fish"`

**`src/executor.rs`** — add fish wrapper and parser:

`build_fish_wrapper(command, temp_path)`:

```fish
{command}
set __shannon_ec $status
env > '{temp_path}'
echo "__SHANNON_CWD="(pwd) >> '{temp_path}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{temp_path}'
exit $__shannon_ec
```

`parse_fish_env(contents)`:

- Parse `KEY=VALUE` lines (one per line, from `env` output)
- Extract `__SHANNON_CWD` and `__SHANNON_EXIT`
- Handle multiline values if present (env vars with newlines are rare but
  possible)
- Return `(HashMap<String, String>, PathBuf)` like the other parsers

Add `ShellKind::Fish` match arms to `execute_command`.

**`src/highlighter.rs`** — add fish color mapping:

- Import `tree_sitter_fish::LANGUAGE`
- Add `fish_color()` method matching fish's node types to Tokyo Night colors
- Fish keywords: `if`, `else`, `for`, `while`, `switch`, `case`, `function`,
  `end`, `begin`, `set`, `return`, `and`, `or`, `not`
- Fish variables use `$` prefix like bash

**`src/main.rs`** — add `ShellKind::Fish` to the detection list:

```rust
let shells: Vec<ShellKind> = [ShellKind::Bash, ShellKind::Nushell, ShellKind::Fish]
    .into_iter()
    .filter(|s| shell_available(*s))
    .collect();
```

**`src/executor.rs` tests** — add:

- `test_parse_fish_env_basic` — standard `KEY=VALUE` output with cwd and exit
- `test_parse_fish_env_empty` — empty input
- `test_build_fish_wrapper` — verify wrapper contains command and temp path

**`tests/integration.rs`** — add fish integration tests (skip if fish not
installed):

- `test_fish_echo` — run `echo hello`, verify exit code 0
- `test_fish_env_capture` — run `set -gx FOO test_val`, verify in returned env
- `test_fish_cwd_capture` — run `cd /tmp`, verify cwd
- `test_env_bash_to_fish` — set env in bash, verify in fish

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes — all new and existing tests green.
3. `cargo run`, Shift+Tab cycles through bash → nushell → fish.
4. Fish prompt shows `[fish]` with syntax highlighting.
5. Commands typed in fish execute correctly.
6. Env vars set in bash carry to fish and vice versa.
7. If fish is not installed, it's silently skipped.

**Result:** Pass

All verification steps confirmed. Fish is now a fully supported shell in
shannon's Shift+Tab rotation. 49 tests pass (33 unit + 16 integration),
including 3 new fish parser tests and 5 new fish integration tests.

#### Conclusion

Fish shell support is complete. The wrapper uses the standard `env` command
for environment capture (simpler than bash or nushell). Tree-sitter-fish
provides syntax highlighting. Fish joins the rotation as bash → nushell →
fish. Ready to tackle fish completions in Experiment 2.
