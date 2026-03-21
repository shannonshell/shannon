# State Synchronization

Shannon keeps your environment variables, working directory, and exit code in
sync across shell switches. Set a variable in bash, switch to nushell, and it's
there.

## Environment Variables

```
[bash] ~/project > export API_KEY="sk-1234"
[bash] ~/project > <Shift+Tab>
[nu] ~/project > $env.API_KEY
sk-1234
```

Only string-valued variables are synchronized. Nushell stores some env vars as
structured data (for example, `PATH` is a list). When capturing state from
nushell, shannon joins list values with `:` (or `;` on Windows) so they work in
bash. Non-string values (numbers, booleans, records) are silently dropped.

## Working Directory

```
[bash] ~/project > cd /tmp
[bash] /tmp > <Shift+Tab>
[nu] /tmp >
```

The working directory is captured after every command and injected into the next
subprocess. The prompt always shows the current directory with `~` for your home
directory.

## Exit Code

The prompt indicator shows whether the last command succeeded:

- `>` — exit code 0 (success)
- `!` — nonzero exit code (failure)

```
[bash] ~/project > true
[bash] ~/project > false
[bash] ~/project ! <Shift+Tab>
[nu] ~/project !
```

The exit code carries across shell switches.

## How It Works

After every command, shannon reads the subprocess's resulting state from a
temporary file. The next command — even in a different shell — starts with that
state. See [Architecture](../02-architecture.md) for the full details on wrapper
scripts and state capture.

## The Strings-Only Boundary

Shannon only transfers string data between shells. This is a deliberate design
constraint:

- Environment variables are always strings
- The working directory is a path (string)
- The exit code is an integer

Shell-internal data structures (bash arrays, nushell tables, functions, aliases)
do not transfer. If you need to pass complex data between shells, serialize it
to a string (JSON, for example) and store it in an env var.
