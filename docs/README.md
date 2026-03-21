# Shannon Documentation

Shannon is a poly-shell that lets you use bash, nushell, and (soon) AI-powered
natural language — all in one session. Press Shift+Tab to switch between shells.
Your environment variables, working directory, and exit code carry over
automatically.

## Getting Started

- [Getting Started](getting-started.md) — install, first run, basic usage

## Features

- [Shell Switching](features/shell-switching.md) — Shift+Tab between bash and nushell
- [State Synchronization](features/state-sync.md) — env vars, cwd, and exit code across shells
- [Syntax Highlighting](features/syntax-highlighting.md) — tree-sitter with Tokyo Night colors
- [Command History](features/history.md) — per-shell history and Ctrl+R search
- [Tab Completion](features/tab-completion.md) — file and directory completion

## Reference

- [Keybindings](reference/keybindings.md) — complete list of keyboard shortcuts
- [Configuration](reference/configuration.md) — config directory and files
- [Supported Shells](reference/supported-shells.md) — bash, nushell, and adding more

## Architecture

- [How Shannon Works](architecture.md) — subprocess model, wrapper scripts, state capture
