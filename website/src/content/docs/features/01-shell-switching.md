# Mode Switching

Shannon has two modes — press **Shift+Tab** to toggle between them:

```
nu ↔ bash
```

## Switching in Action

```
[nu] ~/project > ls | where size > 1mb
...
[nu] ~/project > <Shift+Tab>
[bash] ~/project > echo hello && echo world
hello
world
[bash] ~/project > <Shift+Tab>
[nu] ~/project >
```

The prompt updates immediately to show the active mode.

## What Carries Over

When you switch modes, these are preserved:

1. **Environment variables** — `export FOO=bar` in bash is visible as
   `$env.FOO` in nushell
2. **Working directory** — `cd /tmp` in bash means you're in `/tmp` when you
   switch to nushell

Environment variables are converted automatically between nushell's typed
values and bash strings using `ENV_CONVERSIONS`.

## What Doesn't Carry Over

Shell-internal data stays within its mode:

- Nushell variables (`let x = 5`) don't exist in bash
- Bash local variables and aliases don't exist in nushell
- Only exported environment variables cross the boundary

