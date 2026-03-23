# Syntax Highlighting

Shannon highlights your input as you type using
[tree-sitter](https://tree-sitter.github.io/) grammars. Each shell has its own
grammar, so you get accurate highlighting for bash, nushell, fish, and zsh
syntax.

## Color Themes

By default, shannon uses standard ANSI colors that inherit from your
terminal's theme. If your terminal uses Dracula, shannon looks like Dracula.
If it uses Solarized Light, shannon adapts automatically.

You can also pick a named theme or override individual colors. See
[Configuration](../reference/02-configuration.md) for the `[theme]` section.

### Color Categories

| Category   | What it colors                | Default (ANSI) |
| ---------- | ----------------------------- | -------------- |
| Keywords   | `if`, `for`, `let`, `export`  | Magenta bold   |
| Commands   | `ls`, `grep`, `echo`, `cd`    | Blue           |
| Strings    | `"hello"`, `'world'`          | Green          |
| Numbers    | `42`, `3.14`                  | Yellow         |
| Variables  | `$HOME`, `$env.PATH`          | Cyan           |
| Operators  | `|`, `>`, `&&`, `||`          | Cyan           |
| Comments   | `# this is a comment`         | Dark gray      |
| Types      | `int`, `string` (nushell)     | Yellow         |
| Errors     | Syntax errors                 | Red bold       |

## Bash Highlighting

Keywords like `if`, `then`, `else`, `for`, `while`, `export`, and `function`
are highlighted. Command names are colored. Variables (`$FOO`, `${BAR}`) and
pipes/redirections are distinct.

## Nushell Highlighting

Nushell has additional categories: type annotations (`int`, `string`) and
booleans (`true`, `false`). Nushell keywords include `let`, `mut`, `def`,
`match`, `try`, `catch`, and `use`.

## Fish Highlighting

Fish keywords like `if`, `function`, `set`, `for`, `while` are highlighted.
Command names (the first word of a command) are colored.

## Zsh Highlighting

Zsh uses the bash grammar for highlighting (the syntax is similar enough).

## Incomplete Input

Tree-sitter handles incomplete input gracefully. If you're mid-line and the
syntax is incomplete (like an unterminated string), the parser still highlights
the valid portions. Unrecoverable parse errors are shown in the error color.

## Automatic Switching

When you switch shells with Shift+Tab, the highlighter switches too. The next
keystroke uses the new shell's grammar — no configuration needed.
