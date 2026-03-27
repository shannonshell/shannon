# Shannon Documentation

Shannon is an AI-first shell built on nushell, with bash compatibility via
brush and AI chat via Anthropic. Press Shift+Tab to switch between modes.
Environment variables and working directory carry over automatically.

## Getting Started

- [Getting Started](01-getting-started.md) — install, first run, basic usage

## Features

- [Shell Switching](features/01-shell-switching.md) — Shift+Tab between nu, brush, ai
- [State Synchronization](features/02-state-sync.md) — env vars and cwd across modes
- [Syntax Highlighting](features/03-syntax-highlighting.md) — per-mode highlighting

## Reference

- [Configuration](reference/01-configuration.md) — config files and settings

## Architecture

- [How Shannon Works](02-architecture.md) — mode dispatch, forked deps, env sync

## Nushell Documentation

Shannon IS nushell — all nushell features work natively. For nushell-specific
topics, see the [Nushell documentation](https://nushell.sh/book/):

- [Keybindings](https://nushell.sh/book/line_editor.html#keybindings)
- [Completions](https://nushell.sh/book/line_editor.html#tab-completions)
- [History](https://nushell.sh/book/line_editor.html#history)
- [Hooks](https://nushell.sh/book/hooks.html)
- [Themes and Colors](https://nushell.sh/book/coloring_and_theming.html)
- [Plugins](https://nushell.sh/book/plugins.html)
