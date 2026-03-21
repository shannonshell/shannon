# Architecture

This document explains how shannon works under the hood. It's aimed at someone
who wants to understand the design, not just use the shell.

## The Subprocess Model

Shannon spawns a fresh subprocess for every command. There are no persistent
shell sessions. When you type `ls -la`, shannon runs:

```
bash -c '<wrapper script containing ls -la>'
```

The subprocess inherits stdio directly — you see output in real time, and
interactive commands like `vim` or `htop` work normally.

After the subprocess exits, shannon reads the captured state and uses it for the
next command.

## Wrapper Scripts

Shannon doesn't run your command directly. It wraps it in a script that captures
state after execution.

### Bash Wrapper

```bash
<your command>
__shannon_ec=$?
(export -p; echo "__SHANNON_CWD=$(pwd)"; echo "__SHANNON_EXIT=$__shannon_ec") > '/tmp/shannon_XXXX.env'
exit $__shannon_ec
```

1. Run the user's command.
2. Save the exit code.
3. Dump all exported variables (`export -p`), the cwd, and the exit code to a
   temp file.
4. Exit with the original exit code.

### Nushell Wrapper

```nushell
let __shannon_out = (try { <your command> } catch { |e| $e.rendered | print -e; null })
if ($__shannon_out != null) and (($__shannon_out | describe) != "nothing") { $__shannon_out | print }
let shannon_exit = (if ($env | get -o LAST_EXIT_CODE | is-not-empty) { $env.LAST_EXIT_CODE } else { 0 })
$env | reject config? | insert __SHANNON_CWD (pwd) | insert __SHANNON_EXIT ($shannon_exit | into string) | to json --serialize | save --force '/tmp/shannon_XXXX.env'
```

1. Run the user's command inside try/catch (nushell errors are exceptions).
2. Print the result explicitly (nushell's `echo` returns a value, it doesn't
   print).
3. Capture the entire `$env` as JSON, including cwd and exit code markers.

## State Capture and Parsing

After the subprocess exits, shannon reads the temp file and parses it:

- **Bash:** parses `declare -x KEY="VALUE"` lines, unescaping quotes and
  backslashes.
- **Nushell:** parses JSON. List values (like PATH) are joined with `:`.
  Non-string values are dropped.

Special markers (`__SHANNON_CWD`, `__SHANNON_EXIT`) are extracted and removed
from the env map. The resulting state — env vars, cwd, exit code — is stored
and injected into the next subprocess.

If parsing fails, the previous state is preserved. Shannon doesn't crash on a
bad parse — it degrades gracefully.

## The Strings-Only Boundary

Only three things cross the shell boundary:

1. **Environment variables** — always strings
2. **Working directory** — a path
3. **Exit code** — an integer

This is deliberate. Bash arrays, nushell tables, shell functions, and aliases
are internal to their shell. Trying to translate them between shells would be
fragile and lossy. The strings-only policy keeps the boundary clean and
predictable.

## Why Not Persistent Sessions?

A persistent shell session (keeping bash running in the background) would avoid
the wrapper script overhead. But it creates problems:

- **State capture is harder** — you'd need to query the running shell for its
  env after every command, which is fragile.
- **Shell switching is complex** — you'd need to manage multiple background
  processes and multiplex stdio.
- **Interactive commands** — programs like `vim` need direct terminal access,
  which is harder to provide through a multiplexer.

The subprocess-per-command model is simpler and more reliable. The overhead of
spawning a process is negligible for interactive use.

## Line Editor

Shannon uses [reedline](https://github.com/nushell/reedline) as its line
editor. Reedline provides:

- Emacs keybindings
- File-backed history with reverse search
- Syntax highlighting via the `Highlighter` trait
- Tab completion via the `Completer` trait and menus
- Bracketed paste mode

Shannon implements custom `Highlighter` and `Completer` traits backed by
tree-sitter and filesystem traversal, respectively. The Shift+Tab shell switch
is a custom keybinding that triggers a host command.
