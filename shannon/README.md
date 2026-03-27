# Shannon

An AI-first shell built on nushell, with seamless bash compatibility and
AI chat — all in one session.

Named after [Claude Shannon](https://en.wikipedia.org/wiki/Claude_Shannon),
the father of information theory.

## The Idea

Nobody remembers every shell command. Shannon lets you type in plain English
and have an LLM translate your intent into the right command. When you need
precise control, press **Shift+Tab** to drop into bash — then Shift+Tab back.

```
[nu] ~/project > ls | where size > 1mb
...
[nu] ~/project > <Shift+Tab>

[brush] ~/project > grep -r "TODO" src/
...
[brush] ~/project > <Shift+Tab>

[ai] ~/project > how do I find rust files modified today?
You can use `fd` or `find`:
  fd --extension rs --changed-within 1d
```

## Features

### AI Chat

- Shift+Tab into `[ai]` mode — ask questions in plain English
- Configurable provider (Anthropic by default)
- Context-aware — the LLM knows your cwd and OS
- Conversational — follow-up questions remember context

### Nushell at the Core

Shannon IS nushell — you get all nushell features out of the box:

- Structured data (tables, records, lists)
- Powerful pipelines (`ls | where size > 1mb | sort-by modified`)
- Job control (Ctrl+Z, `job unfreeze`)
- Native completions, multiline editing, plugins
- Hooks, keybindings, themes — all configurable via `config.nu`

### Bash Compatibility

- **Shift+Tab** to switch to `[brush]` mode for bash commands
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

### Shannon-specific settings

Add to `env.nu`:

```nushell
$env.SHANNON_CONFIG = {
    TOGGLE: ["nu", "brush", "ai"]
    AI_PROVIDER: "anthropic"
    AI_MODEL: "claude-sonnet-4-20250514"
    AI_API_KEY_ENV: "ANTHROPIC_API_KEY"
}
```

## Installation

```sh
cargo install shannonshell
```

Or build from source:

```sh
git clone --recursive https://github.com/shannonshell/shannon.git
cd shannon/shannon
cargo build --release
```

Note: `--recursive` is needed to fetch the nushell, brush, and reedline
submodules.

## License

MIT
