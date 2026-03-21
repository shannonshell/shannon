# Shannon

An AI-first shell with seamless access to bash, nushell, and any other shell —
all in one session.

Named after [Claude Shannon](https://en.wikipedia.org/wiki/Claude_Shannon),
the father of information theory.

## The Idea

Nobody remembers every shell command. Shannon's default mode lets you type in
plain English (or any language) and have an LLM translate your intent into the
right command. When you need precise control, press **Shift+Tab** to drop into
bash, nushell, or any other shell — then Shift+Tab back.

```
[shannon] ~/project > find all rust files modified today
  → fd --extension rs --changed-within 1d

[shannon] ~/project > <Shift+Tab>

[bash] ~/project > grep -r "TODO" src/
...
[bash] ~/project > <Shift+Tab>

[nu] ~/project > ls | where size > 1mb
...
```

## Two Problems, One Shell

**Problem 1: Shell commands are hard to remember.** Users constantly search for
the right flags, the right syntax, the right tool. An AI assistant that
understands your intent and generates the command removes this friction.

**Problem 2: No single shell does everything.** Modern shells like nushell have
better languages, but bash is everywhere — tutorials, AI-generated snippets,
deployment scripts. You're forced to choose one and lose the other.

Shannon solves both. The AI mode handles "what command do I need?" and the
poly-shell handles "which shell should run it?"

## How It Works

Shannon is a meta-shell that runs _real_ shell interpreters under the hood. It
manages three things across shell switches:

- **Environment variables** — always strings
- **Current working directory**
- **stdin / stdout / stderr**

These are synchronized whenever you switch shells. Shell-internal data structures
(nushell tables, bash arrays, etc.) do not transfer — only strings cross the
boundary. This is by design.

The AI mode is another "shell" in the Shift+Tab rotation. It sends your input to
a configurable LLM provider, which generates a shell command. You confirm and
execute it in the appropriate shell.

## Why This Matters

Every new shell faces the same adoption barrier: "but I need bash for X."
Shannon removes that barrier entirely. Any shell — no matter how experimental —
gets near-automatic bash compatibility. And the AI mode means you don't need to
memorize any shell's syntax to be productive.

## Features

### AI Mode (planned)

- Type in plain English (or any language) — an LLM generates the shell command
- Configurable LLM provider (Anthropic, OpenAI, local models, etc.)
- Review and confirm before execution
- Context-aware — the LLM sees your cwd, recent commands, and environment

### Poly-Shell (working)

- **Shift+Tab** to cycle between shell languages at the prompt
- **Syntax highlighting** for each shell language (Tokyo Night, tree-sitter)
- **Per-shell command history** — up arrow and Ctrl+R search within the active
  shell's history
- Environment variables synchronized across shell switches
- Working directory synchronized across shell switches
- Exit code propagated across shell switches
- Visual indicator showing which shell is currently active
- Supports any shell installed on the host system
- Graceful degradation — skips shells that aren't installed
- Not a new language — zero new syntax to learn

## How State Synchronization Works

When a command finishes, shannon wraps the invocation in a system-level script
that captures the sub-shell's resulting state — environment variables, working
directory, and exit code — and sends it back to the host process. This means
shannon always knows the current state regardless of what the user's command did,
and can inject that state into the next shell when switching.

Only string-typed data crosses the shell boundary. Shell-internal data structures
(nushell tables, bash arrays, etc.) do not transfer — this is by design.

## Configuration

shannon uses its own configuration directory at `~/.config/shannon/`. Currently
this stores per-shell history files. Planned additions:

- **LLM provider** — which provider to use for AI mode (API key, model, endpoint)
- **Shell list and order** — which shells to include in the Shift+Tab cycle
- **Per-shell rc files** — e.g. `config.nu`, `.bashrc`, `.zshrc` scoped to
  shannon. These will be separate from the user's normal shell configs because
  the experience inside shannon is different from running each shell standalone.

## Signal Handling

- **Ctrl+C** — interrupts the running command in the active sub-shell
- **Ctrl+Z** — suspends the running command in the active sub-shell
- **Ctrl+D** — exits shannon

## Supported Platforms

- macOS
- Linux
- Windows

## Building

Requires [Rust](https://www.rust-lang.org/tools/install).

```sh
cargo build --release
```

## License

MIT
