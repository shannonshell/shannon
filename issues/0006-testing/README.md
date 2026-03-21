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

**Result:** Pass. 16 tests, all green.

#### Conclusion

Parser and data model unit tests are in place. Ready to test the real execution
round-trip in Experiment 2.

### Experiment 2: Integration tests for command execution

#### Description

Test the full round-trip: `execute_command()` with real bash and nushell
subprocesses. These tests verify that the wrapper scripts actually work in real
shells and produce parseable output — the critical path that unit tests can't
cover.

Tests that require nushell use `#[ignore]` so `cargo test` passes on machines
without nushell installed. Run ignored tests with `cargo test -- --ignored`.

#### Changes

**`tests/integration.rs`** (new file):

Bash tests:

- `test_bash_echo` — run `echo hello`, verify command succeeds (exit code 0).
- `test_bash_env_capture` — run `export FOO=test_value_123`, verify `FOO`
  appears in returned state env.
- `test_bash_cwd_capture` — run `cd /tmp`, verify cwd is `/tmp` (or
  `/private/tmp` on macOS).
- `test_bash_exit_code` — run `false`, verify exit code is nonzero.
- `test_bash_env_persistence` — run `export A=1`, then run `echo $A` with the
  returned state, verify it works (exit code 0).

Nushell tests (all `#[ignore]`):

- `test_nushell_echo` — run `print hello`, verify exit code 0.
- `test_nushell_env_capture` — run `$env.FOO = "test_value_456"`, verify `FOO`
  in returned env.
- `test_nushell_cwd_capture` — run `cd /tmp`, verify cwd.
- `test_nushell_exit_code` — run `exit 1`, verify nonzero exit code.

Cross-shell tests (`#[ignore]`):

- `test_env_bash_to_nushell` — set `CROSS=hello` in bash, then execute in
  nushell with the returned state, verify `CROSS` is present.
- `test_cwd_bash_to_nushell` — `cd /tmp` in bash, then execute in nushell with
  returned state, verify cwd.

Helper: a `has_shell(ShellKind) -> bool` function that checks if a shell is
installed, used to skip tests gracefully.

#### Verification

1. `cargo test` passes (nushell tests are `#[ignore]`).
2. `cargo test -- --ignored` passes on machines with nushell installed.
3. All execution round-trips produce correct env, cwd, and exit codes.
