# Configuration

Shannon stores its files in `~/.config/shannon/`.

## Current Files

| File            | Purpose                             |
| --------------- | ----------------------------------- |
| `bash_history`  | Bash command history (up to 10,000) |
| `nu_history`    | Nushell command history (up to 10,000) |

The config directory is created automatically on first run.

## Platform Paths

The config directory follows platform conventions via the `dirs` crate:

| Platform | Path                                    |
| -------- | --------------------------------------- |
| macOS    | `~/Library/Application Support/shannon` |
| Linux    | `~/.config/shannon`                     |
| Windows  | `C:\Users\<user>\AppData\Roaming\shannon` |

Note: On macOS, the actual path is `~/Library/Application Support/shannon`,
not `~/.config/shannon`. The `~/.config` notation is used throughout these docs
for readability.
