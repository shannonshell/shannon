+++
status = "closed"
opened = "2026-03-22"
closed = "2026-03-22"
+++

# Issue 9: SQLite history and autosuggestions

## Goal

Replace per-shell file-backed history with a shared SQLite database and add
autosuggestions (ghost text) powered by that history. Multiple shannon instances
should share history, with the current instance's commands preferred.

## Background

Shannon currently uses `FileBackedHistory` — per-shell plain text files
(`bash_history`, `nu_history`) with 10k entry limits. This has several problems:

1. **No autosuggestions** — users coming from fish or nushell expect ghost text
   that completes commands as they type.
2. **No cross-instance sharing** — two shannon windows don't see each other's
   history.
3. **Per-shell isolation** — bash and nushell have separate history files. A
   command typed in bash doesn't appear when searching in nushell. For a
   poly-shell, shared history across shells makes more sense.
4. **No metadata** — no timestamps, no cwd, no exit status. Can't search by
   "commands I ran in this directory" or "commands that succeeded."

### What reedline provides

Reedline already has everything needed:

**`SqliteBackedHistory`** — a drop-in replacement for `FileBackedHistory`:

- Same `.with_history()` builder method.
- Rich schema: command, timestamp, session_id, cwd, exit status, duration.
- Session isolation with cross-session sharing. Each instance gets a
  `session_id` and `session_timestamp`. Searches return commands from the
  current session OR commands from before this session started.
- Concurrent access via SQLite WAL mode — multiple instances safely share one
  database file.
- Requires the `sqlite` cargo feature on reedline.

**`DefaultHinter`** — ghost text autosuggestions:

- Implements the `Hinter` trait.
- Searches history for the most recent command matching the current prefix.
- Renders the suffix as styled ghost text after the cursor.
- Wired in via `.with_hinter()` on the reedline builder.
- `HistoryHintComplete` and `HistoryHintWordComplete` events for accepting.

### Session prioritization

`SqliteBackedHistory` implements exactly the prioritization we want:

- **Current instance commands** — always visible, regardless of when they were
  typed.
- **Other instances' commands** — visible only if they were typed before this
  session started. This prevents partially-typed commands from another window
  from leaking into autosuggestions.

This means: as you type `gi...`, if you typed `git status` in this session, you
see that first. If not, you see `git status` from any other session. Ctrl+R
works the same way — searches all sessions.

### Design decisions

- **One shared database** — `~/.config/shannon/history.db` instead of per-shell
  text files. All shells share the same history. A command typed in bash appears
  in nushell's history and vice versa.
- **Session ID** — generated at startup (e.g. process ID or timestamp).
- **Ghost text style** — Tokyo Night muted color (`#565f89`), matching the
  comment color from syntax highlighting.
- **Accept keybinding** — Right arrow at end of line accepts the full hint.
  Alt+Right accepts the next word.
- **Entry cap** — periodic cleanup to keep the database under 100k entries.
  Delete oldest entries first.

### Migration

The old `bash_history` and `nu_history` files can be left in place — they won't
conflict. Users who want to import old history can do so manually, but it's not
required for the initial implementation.

## Experiments

### Experiment 1: Replace FileBackedHistory with SqliteBackedHistory and add DefaultHinter

#### Description

Swap the history backend and add autosuggestions in one experiment. Both
features are provided by reedline and require only wiring changes — no new
algorithms or data structures. The history backend change is a prerequisite
for the hinter to work well (it needs session-aware search).

#### Changes

**`Cargo.toml`** — enable reedline's `sqlite` feature:

```toml
reedline = { version = "0.46.0", features = ["sqlite"] }
```

Also add `chrono` as a dependency (needed for `Utc::now()` in session
timestamp).

**`src/shell.rs`** — replace `history_file()` with `history_db()`:

- Remove the per-shell history file method.
- Add `pub fn history_db() -> PathBuf` that returns
  `config_dir().join("history.db")`. One shared database for all shells.

**`src/main.rs`** — rewrite `build_editor()`:

1. Remove `FileBackedHistory` creation.
2. Create a `SqliteBackedHistory`:
   ```rust
   let session_id = Reedline::create_history_session_id();
   let history = SqliteBackedHistory::with_file(
       ShellKind::history_db(),  // or shell::history_db()
       session_id,
       Some(Utc::now()),
   ).expect("failed to create history database");
   ```
3. The session ID and history must be created **once** in `main()` and shared
   across editor rebuilds (shell switches). Currently `build_editor()` is
   called on every Shift+Tab, so the history and session need to be passed
   in rather than created fresh each time.
4. Change `build_editor` signature to accept the history:
   ```rust
   fn build_editor(shell: ShellKind, history: Box<dyn History>, session_id: Option<HistorySessionId>) -> Reedline
   ```
   Wait — reedline takes ownership of the history via `Box<dyn History>`.
   We can't reuse it across rebuilds. Instead, create a new
   `SqliteBackedHistory` pointing to the same file with the same session ID
   each time. SQLite handles concurrent access.
5. Add `DefaultHinter` with Tokyo Night muted style:
   ```rust
   .with_hinter(Box::new(
       DefaultHinter::default()
           .with_style(Style::new().fg(Color::Rgb(86, 95, 137)))  // #565f89
   ))
   ```
6. Add hint accept keybinding — Right arrow at end of line:
   ```rust
   keybindings.add_binding(
       KeyModifiers::NONE,
       KeyCode::Right,
       ReedlineEvent::HistoryHintComplete,
   );
   ```
   Note: this may conflict with the default Right arrow (cursor move).
   Reedline's `UntilFound` can handle this — try hint complete first, fall
   back to cursor move:
   ```rust
   keybindings.add_binding(
       KeyModifiers::NONE,
       KeyCode::Right,
       ReedlineEvent::UntilFound(vec![
           ReedlineEvent::HistoryHintComplete,
           ReedlineEvent::Edit(vec![EditCommand::MoveRight]),
       ]),
   );
   ```
7. Pass `session_id` into `build_editor` so each rebuild uses the same
   session. Store it in `main()`.

**`src/main.rs`** — update imports:

Add `SqliteBackedHistory`, `DefaultHinter`, `HistorySessionId`, `EditCommand`
from reedline. Add `chrono::Utc`. Remove `FileBackedHistory`.

#### Verification

1. `cargo build` succeeds (sqlite feature compiles).
2. `cargo run`, type commands — they're saved. Exit, restart — history is
   there via up arrow and Ctrl+R.
3. Type `ech` — ghost text shows `o "hello"` (or whatever the last `echo`
   command was) in muted gray.
4. Press Right arrow — ghost text is accepted into the input.
5. Open two shannon instances. Type `unique_cmd_1` in instance A. In instance
   B, type `uni` — ghost text shows `que_cmd_1` (cross-instance sharing).
6. Shift+Tab to switch shells — history is shared. A command typed in bash
   appears in nushell's Ctrl+R.
7. `~/.config/shannon/history.db` exists and is a valid SQLite database.
8. `cargo test` passes — no regressions. (History tests in integration.rs
   may need adjustment if they relied on file-backed history.)

**Result:** Pass

All verification steps confirmed. One additional fix was needed: reedline's
`submit_buffer` saves history entries with `start_timestamp: None`. The
cross-session filter uses `start_timestamp < :session_timestamp`, which fails
on NULL. Fixed by calling `editor.update_last_command_context()` after each
command to set `start_timestamp` and `cwd` — the same approach nushell uses.

41 tests pass, no regressions.

#### Conclusion

SQLite history and autosuggestions are working. Ghost text appears as you type,
powered by history with cross-session sharing. Right arrow accepts the hint.
Ctrl+R searches across all sessions. History persists across restarts in a
single shared `history.db`.

## Conclusion

Issue complete. Shannon now has SQLite-backed history with autosuggestions.
Key files:

- `Cargo.toml` — reedline `sqlite` feature enabled, `chrono` added
- `src/shell.rs` — `history_db()` returns shared database path
- `src/main.rs` — `SqliteBackedHistory`, `DefaultHinter`, session ID,
  `update_last_command_context` for timestamps
