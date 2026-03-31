# Shannon

A poly-shell built on nushell, with seamless bash compatibility.
Shift+Tab to switch between nushell and bash.

Named after [Claude Shannon](https://en.wikipedia.org/wiki/Claude_Shannon),
the father of information theory.

## The Idea

Nushell is powerful but the world runs on bash. Shannon gives you both —
press **Shift+Tab** to switch between nushell and bash. Environment variables
and working directory sync automatically.

```
[nu] ~/project > ls | where size > 1mb
...
[nu] ~/project > <Shift+Tab>

[bash] ~/project > grep -r "TODO" src/ && echo "done"
...
[bash] ~/project > <Shift+Tab>

[nu] ~/project >
```

## Features

### Nushell at the Core

Shannon IS nushell — you get all nushell features out of the box:

- Structured data (tables, records, lists)
- Powerful pipelines (`ls | where size > 1mb | sort-by modified`)
- Job control (Ctrl+Z, `job unfreeze`)
- Native completions, multiline editing, plugins
- Hooks, keybindings, themes — all configurable via `config.nu`

### Bash Compatibility

- **Shift+Tab** to switch to `[bash]` mode for bash commands
- Bash syntax highlighting (tree-sitter-bash, Tokyo Night colors)
- Environment variables sync between nushell and bash automatically
- `env.sh` for bash-style setup (PATH, API keys) — follow any tutorial that
  says "add this to your .bashrc"

### Environment Sync

- Environment variables, cwd, and exit code synchronized across mode switches
- Set `export FOO=bar` in bash, switch to nushell — `$env.FOO` works
- Set `$env.BAZ = "qux"` in nushell, switch to bash — `echo $BAZ` works
- PATH and other typed env vars converted automatically via `ENV_CONVERSIONS`

## Configuration

Shannon uses `~/.config/shannon/` (respects `XDG_CONFIG_HOME`):

| File | Purpose |
|------|---------|
| `env.sh` | Bash environment setup — PATH, env vars, API keys (runs first) |
| `env.nu` | Nushell env setup (runs after env.sh) |
| `config.nu` | Nushell config — keybindings, colors, hooks, completions |
| `history.sqlite3` | SQLite command history |

No config files are required — shannon works out of the box.

## Installation

```sh
cargo install --git https://github.com/shannonshell/shannon
```

Or build from source:

```sh
git clone https://github.com/shannonshell/shannon.git
cd shannon
cargo build --release
```

## License

MIT
