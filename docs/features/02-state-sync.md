# State Synchronization

Shannon keeps your environment variables and working directory in sync across
mode switches.

## Environment Variables

```
[brush] ~/project > export API_KEY="sk-1234"
[brush] ~/project > <Shift+Tab>
[nu] ~/project > $env.API_KEY
sk-1234
```

It works both ways:

```
[nu] ~/project > $env.FOO = "hello"
[nu] ~/project > <Shift+Tab>
[brush] ~/project > echo $FOO
hello
```

## Typed Value Conversion

Nushell stores some env vars as typed values (PATH is a list, not a string).
Shannon handles this automatically:

- **Nu to Brush:** `env_to_strings()` converts typed values to strings using
  `ENV_CONVERSIONS` `to_string` closures
- **Brush to Nu:** String env vars are written back. Nushell automatically
  applies `from_string` conversions on the next REPL iteration

You don't need to do anything — PATH, LS_COLORS, and other typed vars just
work.

## Working Directory

```
[brush] ~/project > cd /tmp
[brush] /tmp > <Shift+Tab>
[nu] /tmp >
```

## The Strings-Only Boundary

Only exported environment variables and the cwd cross between modes. Internal
shell state does not transfer:

- Nushell variables (`let x = 5`) stay in nushell
- Bash local variables and aliases stay in brush
- Use `export` / `$env.X = ...` for values that need to cross
