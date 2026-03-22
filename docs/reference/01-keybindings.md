# Keybindings

Shannon uses Emacs-style keybindings for line editing, with a few custom
additions.

## Shannon-Specific

| Key       | Action                           |
| --------- | -------------------------------- |
| Shift+Tab | Switch to next shell             |
| Tab       | File/directory completion        |
| Right     | Accept autosuggestion (at end of line) |
| Ctrl+D    | Exit shannon                     |

## History

| Key    | Action                         |
| ------ | ------------------------------ |
| Up     | Previous command in history    |
| Down   | Next command in history        |
| Ctrl+R | Reverse search through history |

## Line Editing

| Key    | Action                              |
| ------ | ----------------------------------- |
| Ctrl+A | Move cursor to start of line        |
| Ctrl+E | Move cursor to end of line          |
| Ctrl+B | Move cursor back one character      |
| Ctrl+F | Move cursor forward one character   |
| Alt+B  | Move cursor back one word           |
| Alt+F  | Move cursor forward one word        |

## Text Manipulation

| Key    | Action                              |
| ------ | ----------------------------------- |
| Ctrl+U | Delete from cursor to start of line |
| Ctrl+K | Delete from cursor to end of line   |
| Ctrl+W | Delete word before cursor           |
| Ctrl+Y | Yank (paste) last deleted text      |
| Ctrl+T | Transpose characters                |

## Terminal Control

| Key    | Action                              |
| ------ | ----------------------------------- |
| Ctrl+L | Clear screen                        |
| Ctrl+C | Cancel current input / interrupt    |
| Ctrl+D | Exit shannon (at empty prompt)      |
