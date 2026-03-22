# Syntax Highlighting

Shannon highlights your input as you type using
[tree-sitter](https://tree-sitter.github.io/) grammars. Each shell has its own
grammar, so you get accurate highlighting for bash and nushell syntax.

## Tokyo Night Theme

Shannon uses the [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme)
color palette:

| Element   | Color                           | Examples                            |
| --------- | ------------------------------- | ----------------------------------- |
| Keywords  | Purple `#bb9af7`                | `if`, `for`, `let`, `export`, `def` |
| Commands  | Blue `#7aa2f7`                  | `ls`, `grep`, `echo`, `cd`          |
| Strings   | Green `#9ece6a`                 | `"hello"`, `'world'`                |
| Numbers   | Orange `#ff9e64`                | `42`, `3.14`                        |
| Variables | Cyan `#7dcfff`                  | `$HOME`, `$env.PATH`                |
| Operators | Bright cyan `#89ddff`           | `\|`, `>`, `&&`, `\|\|`             |
| Comments  | Gray `#565f89`                  | `# this is a comment`               |
| Types     | Yellow `#e0af68` (nushell only) | `int`, `string`, `list`             |
| Errors    | Red `#f7768e`                   | Syntax errors, unmatched quotes     |
| Default   | Foreground `#a9b1d6`            | Everything else                     |

## Bash Highlighting

Keywords like `if`, `then`, `else`, `for`, `while`, `export`, and `function`
are highlighted in purple. Command names are blue. Variables (`$FOO`,
`${BAR}`) are cyan. Pipes and redirections are bright cyan.

```
export FOO="hello" | grep -r "pattern" src/
^^^^^^              ^      ^  ^^^^^^^^  ^^^^
purple   green      cyan   blue green   default
```

## Nushell Highlighting

Nushell has additional categories: type annotations (`int`, `string`) are
yellow, and booleans (`true`, `false`) are orange. Nushell keywords include
`let`, `mut`, `def`, `match`, `try`, `catch`, and `use`.

```
let name: string = "world"
^^^ ^^^^  ^^^^^^   ^^^^^^^
purple cyan yellow  green
```

## Incomplete Input

Tree-sitter handles incomplete input gracefully. If you're mid-line and the
syntax is incomplete (like an unterminated string), the parser still highlights
the valid portions. Unrecoverable parse errors are shown in red.

## Automatic Switching

When you switch shells with Shift+Tab, the highlighter switches too. The next
keystroke uses the new shell's grammar — no configuration needed.
