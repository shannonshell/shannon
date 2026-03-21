+++
status = "open"
opened = "2026-03-21"
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

In `build_editor()` in `src/main.rs`, add `.with_completer()` and
`.with_menu()` to the reedline builder. Bind Tab to trigger the completion menu.
