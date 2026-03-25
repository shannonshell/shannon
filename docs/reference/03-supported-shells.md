# Supported Shells

Shannon ships with three built-in shell engines. Future shells can be added
by implementing the `ShellEngine` trait.

## Built-in Shells

### Nushell (nu)

- **Highlighting:** tree-sitter-nu grammar
- **Execution:** native via `eval_source()` — no subprocess
- **Always available** — embedded, no system binary required

Nushell is embedded via the nushell crate API. Commands are evaluated directly
by nushell's engine. Builtins auto-print, interactive programs work, and
variables and functions persist across commands.

### Brush (brush)

- **Highlighting:** tree-sitter-bash grammar
- **Execution:** native via `Shell::builder()` and `run_string()` — no subprocess
- **Always available** — embedded, no system binary required

Brush is a Rust reimplementation of bash, embedded as a library. Bash
variables, functions, and aliases persist across commands. Compatible with
bash scripts and documentation.

### AI Chat (ai)

- **Highlighting:** none
- **Execution:** sends messages to an LLM, prints responses
- **Always available** — requires an API key in `env.sh`

The AI shell is a conversational interface powered by an LLM (Anthropic by
default). Type a question, get a response. Conversation history persists
within the session. The LLM knows your cwd and OS for context.

Configure the provider and model in `config.toml`:

```toml
[ai]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
```

## Default Rotation

Without a `toggle` list, all built-in shells are available in default order:
nu, brush, ai. With a `toggle` list, only the listed shells appear, in the
specified order.

```toml
# Just nushell and AI
toggle = ["nu", "ai"]
```

See [Configuration](02-configuration.md) for all config options.
