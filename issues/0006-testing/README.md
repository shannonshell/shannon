+++
status = "open"
opened = "2026-03-21"
+++

# Issue 6: Testing framework and coverage for existing features

## Goal

Establish a testing strategy for olshell and write tests for all existing
features. Every new feature should have tests before or alongside its
implementation — but first we need to cover what we already have.

## Background

olshell currently has zero tests. Every feature has been verified manually. This
is unsustainable as the project grows — we need automated tests before adding
more features (tab completion, hints, config files, etc.).

### What needs testing

The existing features that need test coverage:

1. **Shell wrapper scripts** — bash and nushell wrapper generation. Given a user
   command and temp path, does the wrapper produce correct output?
2. **Env capture parsing** — bash `declare -x` parser and nushell JSON parser.
   Given captured output, does the parser extract the correct env vars, cwd, and
   exit code?
3. **Nushell array-to-string conversion** — PATH and similar list-valued env
   vars are joined with `:` on unix.
4. **Shell switching** — Shift+Tab cycles through available shells correctly.
5. **Prompt rendering** — correct shell indicator, tilde contraction, error
   indicator.
6. **Syntax highlighting** — tree-sitter highlighter produces correct styled
   output for sample inputs.
7. **End-to-end command execution** — run a command in bash, capture env, switch
   to nushell, verify env carried over.

### Testing challenges

olshell is an interactive terminal application. This creates challenges:

- **Reedline interaction** is hard to test directly — it owns the terminal.
  Testing shell switching or keybindings requires either mocking reedline or
  driving a real terminal via a PTY.
- **Subprocess execution** requires real shells to be installed. Tests that call
  bash or nushell are integration tests, not unit tests.
- **Prompt rendering** depends on reedline's `Prompt` trait, which we can test
  by calling the trait methods directly.

### Testing strategy

Split tests into layers:

1. **Unit tests** (fast, no external deps) — test parsing, wrapper generation,
   prompt rendering, and highlighting in isolation. These go in each module as
   `#[cfg(test)] mod tests { ... }`.
2. **Integration tests** (require bash/nu installed) — test actual command
   execution and env capture round-trips. These go in `tests/`.
3. **PTY-based tests** (future, optional) — drive olshell via a pseudo-terminal
   to test interactive features like Shift+Tab and Ctrl+R. Complex to set up,
   defer to later.

### Categories of tests to write

**Unit tests (src/executor.rs):**

- `parse_bash_env` with normal input, empty input, multiline values, special
  characters
- `parse_nushell_env` with normal JSON, arrays (PATH), non-string values dropped
- `build_bash_wrapper` produces expected script
- `build_nushell_wrapper` produces expected script

**Unit tests (src/prompt.rs):**

- `render_prompt_left` shows correct shell name and tilde-contracted path
- `render_prompt_indicator` shows `>` on success, `!` on error
- Color differs per shell

**Unit tests (src/highlighter.rs):**

- Highlighting a bash command produces correct styled segments
- Highlighting a nushell command produces correct styled segments
- Empty input returns empty StyledText
- Incomplete input (error nodes) produces red-styled segments

**Unit tests (src/shell.rs):**

- `ShellKind::display_name`, `binary`, `history_file` return expected values
- `ShellState::from_current_env` captures real env

**Integration tests (tests/):**

- Execute a bash command, verify env capture
- Execute a nushell command, verify env capture
- Set env in bash, verify it carries to nushell
- `cd` in one shell, verify cwd carries over
- Exit code propagation

## Experiments

### Experiment 1: Unit tests for parsers and data model

#### Description

Add unit tests for the pure functions in `executor.rs` and `shell.rs`. These are
the most critical code paths (env parsing determines whether state syncs
correctly) and the easiest to test (no external deps, no terminal, no
subprocesses).

#### Changes

**`src/executor.rs`** — add `#[cfg(test)] mod tests` with:

- `test_parse_bash_env_basic` — parse a typical `export -p` output with a few
  variables, `__OLSHELL_CWD`, and `__OLSHELL_EXIT`. Verify env map, cwd, and
  that olshell markers are excluded from env.
- `test_parse_bash_env_quoted_values` — values containing spaces, quotes, and
  special characters (`declare -x FOO="hello \"world\""`).
- `test_parse_bash_env_empty` — empty string returns None.
- `test_parse_bash_env_no_value` — `declare -x VAR` (exported but unset) is
  skipped.
- `test_parse_nushell_env_basic` — parse a JSON object with string values,
  `__OLSHELL_CWD`, and `__OLSHELL_EXIT`. Verify env map and cwd.
- `test_parse_nushell_env_arrays` — PATH as a JSON array of strings is joined
  with `:`.
- `test_parse_nushell_env_non_string_dropped` — non-string values (objects,
  numbers, booleans) are silently dropped.
- `test_parse_nushell_env_invalid_json` — garbage input returns None.
- `test_unescape_bash_value` — test `\"`, `\\`, `\$`, `\`` escapes.
- `test_build_bash_wrapper` — verify the wrapper contains the user command and
  temp path.
- `test_build_nushell_wrapper` — verify the wrapper contains the user command
  and temp path.

**`src/shell.rs`** — add `#[cfg(test)] mod tests` with:

- `test_shell_kind_display_name` — Bash -> "bash", Nushell -> "nu".
- `test_shell_kind_binary` — Bash -> "bash", Nushell -> "nu".
- `test_shell_kind_history_file` — returns path ending in `bash_history` or
  `nu_history`.
- `test_shell_state_from_current_env` — captures at least PATH and a cwd.

#### Verification

1. `cargo test` passes with all tests green.
2. No tests require bash or nushell to be installed.
3. Parser edge cases (empty input, special chars, arrays) are covered.
