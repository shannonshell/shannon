# Command History

Shannon keeps separate command history for each shell. Your bash history and
nushell history don't mix.

## Storage

History files are stored in your config directory:

- `~/.config/shannon/bash_history`
- `~/.config/shannon/nu_history`

Each file holds up to 10,000 entries. The directory is created automatically on
first run.

## Navigating History

- **Up/Down arrows** — step through previous commands in the active shell
- **Ctrl+R** — reverse search through history

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

## Per-Shell Isolation

When you switch shells with Shift+Tab, the history switches too. Pressing up
arrow in nushell shows nushell commands, not bash commands. This keeps each
shell's history relevant and uncluttered.

History files persist across shannon sessions. When you restart shannon, your
previous history is still there.
