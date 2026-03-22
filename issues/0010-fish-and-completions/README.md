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

### Experiment 2: Adopt fish completions for command-aware tab completion

#### Description

Copy fish's 1,055 completion files into the repo, parse them at build time
into a static lookup table, and integrate that table into shannon's completer
so that Tab shows subcommands and flags for known commands in any shell mode.

This has three pieces:

1. **Copy script** — reproducible way to pull completions from vendored fish
2. **Parser** — Rust build script that reads `.fish` files and generates a
   compiled-in data structure
3. **Completer integration** — replace `FileCompleter` with a
   `ShannonCompleter` that checks command completions first, falls back to
   file completion

#### Data model

The parsed completion data for each command:

```rust
struct CommandSpec {
    /// Subcommands available when no subcommand has been typed yet
    subcommands: Vec<CompletionEntry>,
    /// Global flags (available regardless of subcommand)
    global_flags: Vec<FlagEntry>,
    /// Flags specific to a subcommand
    subcommand_flags: HashMap<String, Vec<FlagEntry>>,
    /// Whether to suppress file completion for this command
    no_file: bool,
}

struct CompletionEntry {
    name: String,
    description: String,
}

struct FlagEntry {
    short: Option<char>,      // -m
    long: Option<String>,     // --message
    description: String,
    takes_arg: bool,          // from -r (requires parameter)
}
```

The full table is `HashMap<String, CommandSpec>` — keyed by command name.

#### Parsing strategy

Parse each `complete` statement extracting these flags:

| Fish flag | Meaning | How we use it |
|-----------|---------|---------------|
| `-c CMD` | Command name | Key in the HashMap |
| `-a ARGS` | Argument/subcommand values | `subcommands` if no `-n` or `-n __fish_use_subcommand` |
| `-s C` | Short option | `FlagEntry.short` |
| `-l NAME` | Long option | `FlagEntry.long` |
| `-d DESC` | Description | `description` field |
| `-f` | No file completion | `no_file = true` |
| `-r` | Requires parameter | `FlagEntry.takes_arg = true` |
| `-x` | Exclusive (= `-r -f`) | Both `takes_arg` and `no_file` |
| `-n COND` | Condition | Determines context (see below) |

Condition handling:

- No `-n` → global (applies always)
- `-n __fish_use_subcommand` or `-n '__fish_use_subcommand'` → this is a
  subcommand definition, goes in `subcommands`
- `-n '__fish_seen_subcommand_from X Y Z'` → these flags are specific to
  subcommands X, Y, Z, go in `subcommand_flags`
- Any other `-n` → skip (tool-specific conditions we can't evaluate)

This gives us 74% coverage per the earlier research.

#### Changes

**`scripts/update-completions.sh`** (new):

Copies `vendor/fish/share/completions/*.fish` to `completions/` in the repo
root. Simple `cp` with a file count summary. The `completions/` directory is
checked into git so builds work without the vendor directory.

**`completions/`** (new directory):

1,055 `.fish` files, checked into git. Updated by running the script.

**`build.rs`** (new):

Rust build script that:

1. Reads all `.fish` files from `completions/`
2. Parses each `complete` statement using a simple line-by-line parser
3. Builds the `HashMap<String, CommandSpec>` in memory
4. Serializes to JSON and writes to `OUT_DIR/completions.json`
5. Emits `cargo:rerun-if-changed=completions/`

The parser is a function that tokenizes a `complete` line into flags. Fish's
`complete` command uses standard POSIX-style options, so parsing is
straightforward: split on whitespace, handle quoted strings, extract flag
values.

**`src/completions.rs`** (new module):

- `CompletionTable` struct wrapping the deserialized data
- `CompletionTable::load()` — deserializes from the embedded JSON
  (`include_str!(concat!(env!("OUT_DIR"), "/completions.json"))`)
- `CompletionTable::complete_command(cmd, args) -> Vec<Suggestion>` — given a
  command name and the arguments typed so far, returns matching completions
- Implements the "needs subcommand" / "seen subcommand from" logic:
  - If no non-flag arg after the command → return subcommands
  - If a subcommand is identified → return that subcommand's flags +
    global flags
  - Filter by prefix of the word being completed

**`src/completer.rs`** — replace `FileCompleter` with `ShannonCompleter`:

- Holds a `CompletionTable` and delegates to it for command-aware completions
- Logic in `complete()`:
  1. Parse the line to identify: command name, arguments so far, current word
  2. If on the first word (command position) → file completion only
  3. If the command is in the completion table → delegate to
     `CompletionTable::complete_command`
  4. If the table returns results → return those
  5. Otherwise → fall back to file/directory completion
- File completion logic stays unchanged (moved into a helper method)

**`src/lib.rs`** — add `pub mod completions;`

**`src/main.rs`** — no changes needed (completer is already wired in)

#### Tests

**`src/completions.rs`** tests:

- `test_load_table` — table loads, contains `git`
- `test_git_subcommands` — `git` has subcommands `commit`, `push`, `pull`,
  etc.
- `test_git_commit_flags` — `git commit` has `--message`/`-m`,
  `--amend`, etc.
- `test_unknown_command` — returns empty for unknown commands
- `test_prefix_filter` — typing `com` filters to `commit`

**`src/completer.rs`** tests (update existing):

- `test_command_completion_git` — line `"git "` → subcommands
- `test_command_completion_git_commit_flags` — line `"git commit --"` → flags
- `test_file_completion_fallback` — line `"cat "` → file completions (cat has
  no interesting subcommands)

#### Verification

1. `scripts/update-completions.sh` copies files successfully.
2. `cargo build` succeeds — build.rs parses all 1,055 files.
3. `cargo test` passes — completion table loads, git completions work.
4. `cargo run`, type `git ` then Tab → subcommands appear (commit, push, pull,
   etc.).
5. Type `git commit --` then Tab → flags appear (--message, --amend, etc.).
6. Type `cat ` then Tab → file completion (fallback).
7. Works in all shell modes (bash, nushell, fish).
