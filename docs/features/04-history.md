# Command History

Shannon stores all command history in a shared SQLite database. Commands typed
in bash and nushell share the same history — switch shells and your history
comes with you.

## Storage

History is stored in a single database file:

```
~/.config/shannon/history.db
```

The database stores rich metadata for each command: the command text, a
timestamp, the session ID, and the working directory. The database is created
automatically on first run.

## Shared Across Shells

Unlike traditional shells that keep separate history files, shannon uses one
history for all shells. A command typed in bash appears when you search history
in nushell, and vice versa. This makes sense for a poly-shell — you're one
user, not two.

## Cross-Instance Sharing

Multiple shannon instances (e.g. multiple terminal windows) share the same
database. History from one instance is visible in another via Ctrl+R and
autosuggestions.

Shannon uses session-aware queries to prioritize sensibly:

- **Current session commands** are always visible.
- **Other sessions' commands** are visible if they were saved before this
  session started.

This means your current session's history is preferred, but you can still find
commands from other windows.

## Navigating History

- **Up/Down arrows** — step through previous commands
- **Ctrl+R** — reverse search through all history (current + other sessions)

### Reverse Search

Press Ctrl+R and start typing to search your history:

```
(reverse-search: grep) grep -r "TODO" src/
```

The prompt updates as you type, showing the best match. If no match is found:

```
(failing reverse-search: xyzzy)
```

Press Enter to execute the match, or Esc to cancel.

## Autosuggestions

As you type, shannon shows a faded "ghost text" suggestion based on your
history. If you've previously typed `git status`, typing `gi` shows the rest
of the command in muted text:

```
[bash] ~/project > gi|t status
                     ^^^^^^^^ ghost text (muted)
```

- **Right arrow** — accept the full suggestion
- Keep typing to narrow the suggestion or ignore it

See [Autosuggestions](06-autosuggestions.md) for more details.
