# Autosuggestions

Shannon shows inline suggestions as you type, based on your command history.
The suggestion appears as faded "ghost text" after your cursor — just like fish
and nushell.

## How It Works

As you type, shannon searches your history for the most recent command that
starts with what you've typed so far. The remainder of that command appears as
muted ghost text:

```
[bash] ~/project > git sta|tus
                          ^^^ ghost text
```

The suggestion updates with every keystroke. If no history entry matches, no
ghost text appears.

## Accepting Suggestions

- **Right arrow** — accept the entire suggestion
- Keep typing — the suggestion narrows or disappears as you type

After accepting, the suggestion becomes part of your input and you can edit it
before pressing Enter.

## Prioritization

Suggestions prefer commands from your current session. If no match is found in
the current session, shannon looks at commands from other sessions (other
terminal windows) that were saved before this session started.

This means if you typed `git push --force` in another window, you'll see it
suggested — but commands from your current window are always preferred.

## Style

Ghost text uses the Tokyo Night muted color (`#565f89`), the same as comments
in syntax highlighting. This keeps it visible but clearly distinct from your
actual input.
