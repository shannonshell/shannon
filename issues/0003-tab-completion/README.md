+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-21"
+++

# Issue 3: Tab completion

## Goal

Add file and directory tab completion to shannon. Every shell has this — shannon
feels broken without it.

## Background

Reedline supports tab completion via the `Completer` trait and built-in menu
types (`ColumnarMenu`, `ListMenu`, `IdeMenu`). We need to implement a
`Completer` that provides file/directory completions and wire it up with a
completion menu.

For MVP, file/directory completion is sufficient. Command-aware and
argument-aware completion can come later (Issue 2 classified it as "nice to
have").

### Integration point

In `build_editor()` in `src/main.rs`, add `.with_completer()` and `.with_menu()`
to the reedline builder. Bind Tab to trigger the completion menu.

### Research findings

Reedline requires us to implement the `Completer` trait:

```rust
pub trait Completer: Send {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion>;
}
```

Each `Suggestion` includes a `value` (the replacement text), a `span` (what
range of the input to replace), and optional fields like `description` and
`style`. There is no built-in file completer — we write our own.

Nushell (which also uses reedline) binds Tab like this:

```rust
ReedlineEvent::UntilFound(vec![
    ReedlineEvent::Menu("completion_menu".to_string()),
    ReedlineEvent::MenuNext,
])
```

And uses `ColumnarMenu` with 4 columns for display.

All shells agree on core UX: single match inserts immediately, multiple matches
show a list, directories get a trailing `/`, hidden files are excluded unless the
user typed a leading `.`, and `~` expands to home.

## Experiments

### Experiment 1: File/directory completer with menu

#### Description

Implement a `FileCompleter` that provides file and directory completions, wire
it into reedline with a `ColumnarMenu`, and bind Tab to trigger it. This is the
full MVP — if this works, tab completion is done.

#### Changes

**`src/completer.rs`** (new file):

- `FileCompleter` struct implementing reedline's `Completer` trait.
- `complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion>`:
  1. Extract the word under the cursor by scanning backward from `pos` to the
     nearest whitespace (or start of line). This is the partial path the user
     is typing.
  2. Expand `~` to the home directory for matching purposes.
  3. Split into a directory part and a prefix part. For `src/ma`, the directory
     is `src/` and the prefix is `ma`. For `foo`, the directory is `.` and the
     prefix is `foo`.
  4. Read the directory entries with `std::fs::read_dir`.
  5. Filter entries to those whose filename starts with the prefix
     (case-sensitive).
  6. Skip hidden files (starting with `.`) unless the prefix starts with `.`.
  7. For each match, build a `Suggestion`:
     - `value`: the completed path. If the original input used `~`, keep `~`
       in the value (don't expand to absolute). Append `/` for directories.
     - `span`: covers the word being completed (start..pos).
     - `append_whitespace`: `true` for files, `false` for directories (so the
       user can keep tabbing deeper).
  8. Sort suggestions: directories first, then files, alphabetically within
     each group.

**`src/main.rs`** — update `build_editor`:

- Add `mod completer;`
- Create a `FileCompleter` and pass it via `.with_completer(Box::new(...))`.
- Create a `ColumnarMenu` named `"completion_menu"` with default settings
  (4 columns).
- Add it via `.with_menu(ReedlineMenu::EngineCompleter(Box::new(...)))`.
- Bind Tab in the keybindings:
  ```rust
  keybindings.add_binding(
      KeyModifiers::NONE,
      KeyCode::Tab,
      ReedlineEvent::UntilFound(vec![
          ReedlineEvent::Menu("completion_menu".to_string()),
          ReedlineEvent::MenuNext,
      ]),
  );
  ```

**`src/lib.rs`** — add `pub mod completer;` for test access.

**`src/completer.rs`** — add `#[cfg(test)] mod tests` with:

Unit tests use a temporary directory (`tempfile::TempDir`) populated with a
known file tree so tests are deterministic and don't depend on the repo's
working directory. Helper function `setup_test_dir()` creates:

```
tmp/
├── src/
│   ├── main.rs
│   └── lib.rs
├── Cargo.toml
├── Cargo.lock
├── .gitignore
├── .hidden_dir/
│   └── secret.txt
└── notes.txt
```

Tests call `complete()` with the completer's cwd set to the temp dir.

- `test_complete_partial_filename` — line `"cat Car"`, pos at end →
  suggestions include `Cargo.toml` and `Cargo.lock`.
- `test_complete_directory_contents` — line `"ls src/"`, pos at end →
  suggestions include `src/main.rs` and `src/lib.rs`.
- `test_complete_directory_trailing_slash` — completing `sr` → `src/` with
  `append_whitespace: false`.
- `test_complete_file_appends_space` — completing `notes` → `notes.txt` with
  `append_whitespace: true`.
- `test_hidden_files_excluded` — line `"ls "`, pos at end → suggestions do
  not include `.gitignore` or `.hidden_dir/`.
- `test_hidden_files_included_with_dot_prefix` — line `"ls ."`, pos at end →
  suggestions include `.gitignore` and `.hidden_dir/`.
- `test_no_matches` — line `"cat zzz"`, pos at end → empty suggestions.
- `test_sort_order` — directories sort before files, alphabetical within
  each group.
- `test_tilde_expansion` — line with `~/` completes home directory contents
  and keeps `~` in suggestion values.

**`Cargo.toml`** — add `tempfile` as a dev dependency.

#### Verification

1. `cargo build` succeeds.
2. `cargo run`, type `src/` then Tab — shows files in `src/`.
3. Type `Car` then Tab — completes to `Cargo.toml` (or shows `Cargo.toml` and
   `Cargo.lock`).
4. Type `.` then Tab — shows hidden files (`.git`, `.gitignore`, etc.).
5. Type `~` then Tab — shows home directory contents.
6. Type `nonexistent` then Tab — nothing happens (no matches).
7. Complete a directory — trailing `/` appears, no space appended.
8. Complete a file — space appended after the filename.
9. Multiple matches display in a columnar menu below the prompt.
10. `cargo test` passes — all new completer tests green, no regressions.

**Result:** Pass

All verification steps confirmed. 36 tests pass (9 new completer unit tests,
16 existing unit tests, 11 integration tests). Tab completion works for files,
directories, hidden files, tilde expansion, and nested paths.

#### Conclusion

File/directory tab completion is implemented in a single experiment. The
`FileCompleter` in `src/completer.rs` handles path completion with all expected
shell UX conventions. Wired into reedline via `ColumnarMenu` and Tab keybinding.

## Conclusion

Issue complete. Tab completion for files and directories is working. Key files:

- `src/completer.rs` — `FileCompleter` struct with 9 unit tests
- `src/main.rs` — Tab keybinding, completer and menu wired into `build_editor()`
