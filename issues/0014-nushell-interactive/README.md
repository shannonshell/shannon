+++
status = "open"
opened = "2026-03-22"
+++

# Issue 14: Interactive programs don't work in nushell mode

## Goal

Fix the nushell wrapper so that interactive programs (vim, nvim, htop, less,
etc.) work correctly when run from nushell mode in shannon.

## Background

The nushell wrapper captures command output via:

```nushell
let __shannon_out = (try { {{command}} } catch { |e| $e.rendered | print -e; null })
if ($__shannon_out != null) and (($__shannon_out | describe) != "nothing") { $__shannon_out | print }
```

The `let x = (try { command })` pattern captures stdout into a variable. This
works for commands that produce values (`pwd`, `ls`, `echo`) because we
explicitly `| print` afterward. But it breaks interactive programs like vim
because their stdout is redirected to the variable instead of the terminal.

### The tension

Nushell in `-c` mode doesn't auto-print intermediate results. If we just write
`{{command}}` on its own line, value-returning commands like `pwd` produce no
visible output. We need explicit printing.

But the capture pattern (`let x = (...)`) redirects stdout, which breaks
interactive programs.

### What we need

A wrapper that:

1. Lets interactive programs have direct terminal access (stdin/stdout)
2. Prints the output of value-returning commands (`pwd`, `ls`)
3. Captures errors gracefully
4. Captures env vars, cwd, and exit code after execution

### Possible approaches

**A. Detect interactive commands** — maintain a list of known interactive
programs (vim, nvim, htop, less, etc.) and use a different wrapper for them.
Fragile — the list will always be incomplete.

**B. Run command first, print if it returns a value** — use nushell's type
system to check if the command returned something printable, without capturing
stdout. May not be possible in nushell's `-c` mode.

**C. Use `do { } | print` pattern** — `do { command } | print` lets the command
run and pipes its output to print. But this also captures stdout, breaking
interactive programs.

**D. Accept the limitation** — document that interactive programs should be run
from bash or fish mode (Shift+Tab). This is the simplest "fix" but a poor user
experience.

**E. Run without capture, add explicit print for known value commands** — the
inverse of A: run everything directly, add `| print` only for commands known to
return values. Less fragile since most commands write to stdout directly.

### Research needed

- How does standalone nushell handle this in its own REPL? It auto-prints
  returned values. What mechanism does it use?
- Can we access nushell's auto-print behavior from `-c` mode?
- Is there a nushell flag or config that enables auto-printing in `-c` mode?
