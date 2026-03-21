# olshell

A poly-shell that wraps multiple shell interpreters and lets you switch between
them mid-session.

## The Problem

Modern shells like [nushell](https://www.nushell.sh/) have far superior
languages to bash. But bash is everywhere — tutorials, AI-generated snippets,
Stack Overflow answers, deployment scripts. You're constantly forced to choose:
stay in your modern shell and translate syntax, or drop back to bash and lose
your environment.

## The Solution

olshell is not a new shell language. It's a meta-shell that runs _real_ shell
interpreters under the hood and lets you hot-swap between them with
**Shift+Tab**. Press Shift+Tab to switch from nushell to bash, paste the
command, run it, and Shift+Tab back. Your environment variables, working
directory, and I/O stay consistent across switches.

This works because all shells already agree on a common interop layer:

- **Environment variables** — always strings
- **Current working directory**
- **stdin / stdout / stderr**

olshell keeps these synchronized whenever you switch shells. Internal data
structures differ between shells, but the shared OS-level state does not.

## Why This Matters

Every new shell faces the same adoption barrier: "but I need bash for X."
olshell removes that barrier entirely. Any shell — no matter how experimental —
gets near-automatic bash compatibility. Just switch to bash when you need it and
switch back.

## Features

- **Shift+Tab** to cycle between shell languages at the prompt
- **Syntax highlighting** for each shell language
- **Per-shell command history** — up arrow and Ctrl+R search within the active
  shell's history
- Environment variables synchronized across shell switches
- Working directory synchronized across shell switches
- Exit code propagated across shell switches
- Visual indicator showing which shell is currently active
- Supports any shell installed on the host system
- Graceful degradation — skips shells that aren't installed
- Not a new language — zero new syntax to learn
- Architecture designed for forward-compatibility with IDE features like command
  completion

## How State Synchronization Works

When a command finishes, olshell wraps the invocation in a system-level script
that captures the sub-shell's resulting state — environment variables, working
directory, and exit code — and sends it back to the host process. This means
olshell always knows the current state regardless of what the user's command did,
and can inject that state into the next shell when switching.

Only string-typed data crosses the shell boundary. Shell-internal data structures
(nushell tables, bash arrays, etc.) do not transfer — this is by design.

## Configuration

olshell uses its own configuration directory (e.g.
`~/.config/olshell/`). This includes:

- **Shell list and order** — which shells to include in the Shift+Tab cycle
- **Per-shell rc files** — e.g. `config.nu`, `.bashrc`, `.zshrc` scoped to
  olshell. These are separate from the user's normal shell configs because the
  experience inside olshell is different from running each shell standalone.

## Signal Handling

- **Ctrl+C** — interrupts the running command in the active sub-shell
- **Ctrl+Z** — suspends the running command in the active sub-shell
- **Ctrl+D** — sends EOF to the active sub-shell (does not quit olshell unless
  all shells have exited)

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
