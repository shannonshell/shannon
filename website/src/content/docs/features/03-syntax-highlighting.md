# Syntax Highlighting

Each mode has its own syntax highlighter that activates automatically when you
switch modes.

## Nu Mode

Uses nushell's native `NuHighlighter`. Full nushell syntax awareness —
keywords, commands, strings, variables, pipes, types, and errors are all
colored according to your `$env.config.color_config`.

See [Nushell theming docs](https://nushell.sh/book/coloring_and_theming.html)
for customization.

## Brush Mode

Uses `BashHighlighter` with tree-sitter-bash. Tokyo Night color scheme:

| Category | What it colors | Color |
|----------|---------------|-------|
| Keywords | `if`, `for`, `export`, `while` | Purple |
| Commands | `ls`, `grep`, `echo`, `cd` | Blue |
| Strings | `"hello"`, `'world'` | Green |
| Numbers | `42`, `3.14` | Orange |
| Variables | `$HOME`, `${BAR}` | Yellow |
| Operators | `\|`, `>`, `&&`, `\|\|` | Cyan |
| Comments | `# this is a comment` | Gray |

## Automatic Switching

When you press Shift+Tab, the highlighter switches immediately. The next
keystroke uses the new mode's highlighter.
