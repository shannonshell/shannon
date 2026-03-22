+++
status = "open"
opened = "2026-03-22"
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
