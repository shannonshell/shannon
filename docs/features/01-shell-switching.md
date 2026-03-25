# Shell Switching

Shannon lets you use multiple shells in one session. Press **Shift+Tab** to
cycle between them.

## How It Works

Shannon has three built-in shell engines — all always available:

```
nu → brush → ai → nu → ...
```

If a shell isn't installed, it's skipped. If only one shell is available,
Shift+Tab has no effect.

You can customize which shells appear and their order with the `toggle` option
in `config.toml`. See [Configuration](../reference/02-configuration.md).

## Switching in Action

```
[bash] ~/project > echo "I'm in bash"
I'm in bash
[bash] ~/project > <Shift+Tab>
[nu] ~/project > echo "Now I'm in nushell"
Now I'm in nushell
[nu] ~/project > <Shift+Tab>
[bash] ~/project >
```

The prompt updates immediately to show the active shell.

## What Carries Over

When you switch shells, three things are preserved:

1. **Environment variables** — `export FOO=bar` in bash is visible as
   `$env.FOO` in nushell
2. **Working directory** — `cd /tmp` in bash means you're in `/tmp` when you
   switch to nushell
3. **Exit code** — the prompt indicator (`>` or `!`) reflects the last
   command's exit code regardless of which shell ran it

## What Doesn't Carry Over

Shell-internal data structures do not transfer between shells. This includes:

- Bash arrays and associative arrays
- Nushell tables, records, and lists
- Shell functions and aliases
- Shell-local variables (unexported)

Only string-valued environment variables cross the shell boundary. This is by
design — see [Architecture](../02-architecture.md) for why.

## Per-Shell Features

Each shell keeps its own:

- **Syntax highlighting** — colors match the active shell's grammar
- **Tab completion** — command and file completion works the same in all shells
- **Command history** — shared across all shells via SQLite

See [History](04-history.md) and [Syntax Highlighting](03-syntax-highlighting.md)
for details.
