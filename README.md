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
- Environment variables synchronized across shell switches
- Working directory synchronized across shell switches
- Supports any shell installed on the host system
- Not a new language — zero new syntax to learn

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
