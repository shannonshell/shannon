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
[nu] ~/project >                    ← press Enter on empty line
[nu:ai] ~/project > find all rust files modified today
  → fd --extension rs --changed-within 1d
  [Enter] run  [Esc] cancel

[nu] ~/project > <Shift+Tab>

[bash] ~/project > grep -r "TODO" src/
...
[bash] ~/project > <Shift+Tab>

[fish] ~/project > ls | head -5
...
```

## Features

### AI Mode

- Press **Enter on empty line** to toggle AI mode
- Type in plain English — an LLM generates the shell command
- Configurable provider (Anthropic by default)
- Review and confirm before execution
- Context-aware — the LLM knows your shell, cwd, and OS
- Conversational within a session — follow-up questions remember context

### Poly-Shell

- **Shift+Tab** to cycle between shells (bash, nushell, fish, zsh)
- **Nushell embedded** — nushell runs natively via library, not as a subprocess
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

### Nushell as a First-Class Citizen

Nushell is embedded via its crate API (`eval_source`), not wrapped in a
subprocess. This means:

- `pwd`, `ls`, and other builtins auto-print their results
- Interactive programs like `vim` and `htop` work correctly
- Nushell variables and functions persist across commands
- No system `nu` binary required — nushell is always available

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

## Building

Requires [Rust](https://www.rust-lang.org/tools/install).

```sh
cargo build --release
./scripts/install.sh
```

## License

MIT
