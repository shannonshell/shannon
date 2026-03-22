# Supported Shells

Shannon ships with built-in support for four shells. Any additional shell can
be added via [config.toml](02-configuration.md).

## Built-in Shells

### Bash

- **Binary:** `bash`
- **Highlighting:** tree-sitter-bash grammar
- **Parser:** `bash` (reads `declare -x` output)

Bash is available on virtually every Unix system.

### Nushell

- **Binary:** `nu`
- **Highlighting:** tree-sitter-nu grammar
- **Parser:** `nushell` (reads JSON from `$env | to json`)

Nushell must be installed separately.

#### Nushell Quirks

- **PATH is a list** in nushell. Shannon joins it with `:` (`;` on Windows)
  when capturing state, so it works correctly in bash.
- **Non-string env vars** are dropped. Nushell allows structured values in
  `$env`, but only strings cross the shell boundary.
- **Output rendering** — nushell's `echo` returns a value rather than
  printing. Shannon's wrapper uses `print` to render output to the terminal.

### Fish

- **Binary:** `fish`
- **Highlighting:** tree-sitter-fish grammar
- **Parser:** `env` (reads `KEY=VALUE` output)

Fish must be installed separately (`brew install fish` on macOS).

Fish is also the source of shannon's command-aware completions — see
[Tab Completion](../features/05-tab-completion.md).

### Zsh

- **Binary:** `zsh`
- **Highlighting:** bash grammar (zsh syntax is close enough)
- **Parser:** `env` (reads `KEY=VALUE` output)

Zsh is the default shell on macOS. It uses the same generic `env` wrapper as
fish, and the bash grammar for syntax highlighting.

## Adding a Custom Shell

Any shell that supports `-c` for command execution can be added via
`config.toml`. No code changes or recompilation needed.

Example — adding elvish:

```toml
toggle = ["nu", "bash", "elvish"]

[shells.elvish]
binary = "elvish"
parser = "env"
wrapper = """
{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
"""
```

Custom shells must be included in the `toggle` list to appear in the rotation.
The `env` parser reads standard `KEY=VALUE` output, which works for most POSIX
shells.

See [Configuration](02-configuration.md) for all config options.

## Shell Detection

Shannon checks each shell's binary at startup using `<binary> --version`. If
the binary isn't found in PATH, the shell is silently skipped. If no shells
are available, shannon exits with an error.

Without a `toggle` list, all installed built-in shells are available in
default order: bash, nu, fish, zsh. With a `toggle` list, only the listed
shells appear, in the specified order.
