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

## Experiments

### Experiment 1: Research how nushell handles auto-printing

#### Description

Study the vendored nushell source to understand how the nushell REPL auto-prints
command results. The REPL does this automatically — `pwd` prints its result
without explicit `| print`. But `-c` mode doesn't. Understanding the mechanism
tells us whether we can replicate it in our wrapper, or whether we need a
fundamentally different approach.

#### Research tasks

**1. How does the nushell REPL auto-print?**

Look in `vendor/nushell/` for the REPL evaluation loop. When the user types
`pwd` and presses Enter, nushell evaluates it and prints the result. Find:

- Where does the REPL evaluate a command?
- Where does it decide to print the result?
- Is there a `display_output` hook or similar mechanism?
- Does it use a pipeline that ends in an implicit `print`?
- What's the difference between how the REPL handles evaluation vs `-c`?

**2. Why doesn't `-c` mode auto-print?**

Compare the `-c` code path to the REPL code path. Look in `src/run.rs` or the
main binary crate for how `-c` commands are evaluated. Specifically:

- Does `-c` mode use the same evaluation pipeline as the REPL?
- If not, what's different?
- Is there a flag or config to enable REPL-like behavior in `-c` mode?

**3. How do external commands (vim, htop) work in the REPL?**

In the nushell REPL, `vim` works fine — it gets the terminal. But in our
wrapper, `try { vim }` captures its output. Understand:

- Does the REPL treat external commands differently?
- Does it detect that a command is external vs internal?
- Does it skip the output capture for external commands?

**4. Can we use nushell's `display_output` hook?**

Nushell has a config option `hooks.display_output` that controls how pipeline
results are rendered. Check:

- What is the default `display_output` hook?
- Can we set it in our wrapper to force auto-printing?
- Does it apply in `-c` mode?

**5. Test alternative wrapper patterns:**

Try these patterns from the command line and document which ones work for both
`pwd` (value-returning) and `vim` (interactive):

```
# Pattern A: bare command (no try)
nu -c 'pwd'

# Pattern B: with semicolons separating capture
nu -c 'pwd; echo done'

# Pattern C: display_output hook
nu -c '$env.config.hooks.display_output = "print"; pwd'

# Pattern D: last expression is auto-printed?
nu -c 'let x = 1; pwd'

# Pattern E: explicit if/else on type
nu -c 'let out = (do { pwd }); if ($out | describe) != "nothing" { $out | print }'
```

For each, test with both `pwd` and `vim` (or `less` as a safer alternative).

#### Verification

1. The nushell REPL auto-print mechanism is documented with source file
   references.
2. The difference between REPL and `-c` mode is explained.
3. At least one wrapper pattern is found that works for both value-returning and
   interactive commands, OR a clear explanation of why no single pattern can
   work.
4. A recommendation is made for how to fix the nushell wrapper.
