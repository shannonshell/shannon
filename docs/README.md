# Shannon Documentation

Shannon is an AI-first poly-shell — use bash, nushell, fish, zsh, and
AI-powered natural language, all in one session. Press Shift+Tab to switch
between shells. Your environment variables, working directory, and exit code
carry over automatically.

## Getting Started

- [Getting Started](01-getting-started.md) — install, first run, basic usage

## Features

- [Shell Switching](features/01-shell-switching.md) — Shift+Tab between bash, nushell, fish, zsh
- [State Synchronization](features/02-state-sync.md) — env vars, cwd, and exit code across shells
- [Syntax Highlighting](features/03-syntax-highlighting.md) — tree-sitter with configurable themes
- [Command History](features/04-history.md) — shared SQLite history across shells and instances
- [Tab Completion](features/05-tab-completion.md) — command-aware completion for 983 commands
- [Autosuggestions](features/06-autosuggestions.md) — ghost text from history as you type

## Reference

- [Keybindings](reference/01-keybindings.md) — complete list of keyboard shortcuts
- [Configuration](reference/02-configuration.md) — config directory, theming, shell rotation
- [Supported Shells](reference/03-supported-shells.md) — bash, nushell (embedded), fish, zsh, custom

## Architecture

- [How Shannon Works](02-architecture.md) — embedded nushell, subprocess wrappers, state capture
