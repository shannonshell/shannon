# Shannon

An AI-first shell with seamless access to bash, nushell, and any other shell —
all in one session.

Named after [Claude Shannon](https://en.wikipedia.org/wiki/Claude_Shannon),
the father of information theory.

## The Idea

Nobody remembers every shell command. Shannon lets you type in plain English
and have an LLM translate your intent into the right command. When you need
precise control, press **Shift+Tab** to drop into bash, nushell, or any other
shell — then Shift+Tab back.

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

- Shift+Tab into the `[ai]` shell — ask questions in plain English
- Configurable provider (Anthropic by default)
- Context-aware — the LLM knows your cwd and OS
- Conversational — follow-up questions remember context

### Poly-Shell

- **Shift+Tab** to cycle between shells (bash, brush, nushell, fish, zsh)
- **Nushell and brush embedded** — nushell and brush run natively via library, not as subprocesses
- **Syntax highlighting** for each shell (Tokyo Night theme, tree-sitter)
- **Command-aware tab completion** — 983 commands with subcommands and flags
- **Autosuggestions** — ghost text from history as you type
- **SQLite history** — shared across shells and instances, cross-session
- **Vi keybindings** by default
- Environment variables, cwd, and exit code synchronized across shell switches
- **Configurable** — add any shell via `config.toml`, no recompilation
- **Terminal integration** — OSC 2 (title) and OSC 7 (cwd) for tab titles
  and new-pane-in-same-directory
- Not a new language — zero new syntax to learn

### Nushell and Brush as First-Class Citizens

Nushell is embedded via its crate API (`eval_source`), not wrapped in a
subprocess. Brush (a bash-compatible shell written in Rust) is also embedded
via its crate API (`Shell::builder()` + `run_string()`). This means:

- `pwd`, `ls`, and other builtins auto-print their results
- Interactive programs like `vim` and `htop` work correctly
- Nushell variables and functions persist across commands
- Bash variables, functions, and aliases persist across commands (via brush)
- No system `nu` binary required — nushell and brush are always available
- Bash is also available as a traditional subprocess wrapper if installed

## Configuration

Shannon uses `~/.config/shannon/` (respects `XDG_CONFIG_HOME`):

| File | Purpose |
|------|---------|
| `config.toml` | Shannon settings — shell rotation, custom shells, AI config |
| `env.sh` | Environment setup — PATH, env vars, API keys (bash script) |
| `history.db` | SQLite command history (shared across shells and instances) |

No config files are required — shannon works out of the box.

### Example config.toml

```toml
toggle = ["nu", "bash"]

[ai]
model = "claude-sonnet-4-20250514"
```

See the [documentation](docs/) for full config options.

## Installation

```sh
cargo install shannonshell
```

Or build from source:

```sh
git clone https://github.com/shannonshell/shannon.git
cd shannon/shannon
cargo build --release
```

## License

MIT
