# Keybindings

Shannon uses vi-style keybindings for line editing. You start in insert mode
(ready to type). Press **Esc** to enter normal mode, **i** to return to
insert mode.

## Shannon-Specific

These work in both insert and normal mode:

| Key       | Action                              |
| --------- | ----------------------------------- |
| Shift+Tab | Cycle to next shell                 |
| Ctrl+S    | Shell picker menu                   |
| Tab       | Command/file completion             |
| Right     | Accept autosuggestion (insert mode) |
| Enter     | Clear error state (on empty line)    |
| Ctrl+D    | Exit shannon                        |

## Vi Normal Mode

| Key | Action                             |
| --- | ---------------------------------- |
| Esc | Enter normal mode                  |
| h   | Move cursor left                   |
| l   | Move cursor right                  |
| w   | Move forward one word              |
| b   | Move back one word                 |
| 0   | Move to start of line              |
| $   | Move to end of line                |
| x   | Delete character under cursor      |
| dd  | Delete entire line                 |
| dw  | Delete word                        |
| p   | Paste                              |
| u   | Undo                               |
| i   | Enter insert mode at cursor        |
| a   | Enter insert mode after cursor     |
| A   | Enter insert mode at end of line   |
| I   | Enter insert mode at start of line |

## Vi Insert Mode

| Key    | Action                    |
| ------ | ------------------------- |
| Esc    | Exit to normal mode       |
| Ctrl+W | Delete word before cursor |
| Ctrl+U | Delete to start of line   |

## History

| Key    | Action                         |
| ------ | ------------------------------ |
| Up     | Previous command in history    |
| Down   | Next command in history        |
| Ctrl+R | Reverse search through history |
| k      | Previous command (normal mode) |
| j      | Next command (normal mode)     |

## Terminal Control

| Key    | Action                           |
| ------ | -------------------------------- |
| Ctrl+L | Clear screen                     |
| Ctrl+C | Cancel current input / interrupt |
| Ctrl+D | Exit shannon (at empty prompt)   |
