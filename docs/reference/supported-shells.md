# Supported Shells

Shannon currently supports two shells. More can be added.

## Bash

- **Binary:** `bash`
- **Detection:** `bash --version` at startup
- **History file:** `~/.config/shannon/bash_history`
- **Highlighting:** tree-sitter-bash grammar

Bash is available on virtually every Unix system. Shannon wraps commands with
`bash -c` and captures state via `export -p`.

## Nushell

- **Binary:** `nu`
- **Detection:** `nu --version` at startup
- **History file:** `~/.config/shannon/nu_history`
- **Highlighting:** tree-sitter-nu grammar

Nushell must be installed separately. Shannon wraps commands with `nu -c` and
captures state as JSON via `$env | to json`.

### Nushell Quirks

- **PATH is a list** in nushell. Shannon joins it with `:` (`;` on Windows)
  when capturing state, so it works correctly in bash.
- **Non-string env vars** are dropped. Nushell allows structured values in
  `$env`, but only strings cross the shell boundary.
- **Output rendering** — nushell's `echo` returns a value rather than
  printing. Shannon's wrapper uses `print` to render output to the terminal.

## Adding a New Shell

Adding a shell requires changes in four files:

1. **`src/shell.rs`** — add a variant to `ShellKind`, implement `display_name`,
   `binary`, and `history_file`.
2. **`src/executor.rs`** — add a wrapper script builder (like
   `build_bash_wrapper`) that runs the command and captures env vars, cwd, and
   exit code to a temp file. Add a parser for the captured output.
3. **`src/highlighter.rs`** — add a color mapping function and a tree-sitter
   grammar dependency (if one exists).
4. **`src/main.rs`** — add the shell to the detection list.
