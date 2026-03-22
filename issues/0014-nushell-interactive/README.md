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

#### Results

**1. How the nushell REPL auto-prints:**

Both REPL and `-c` mode call the same `print_pipeline()` function
(`crates/nu-cli/src/util.rs:216`). It uses a `display_output` hook (default:
`"if (term size).columns >= 100 { table -e } else { table }"`) to format and
print the LAST expression's result.

Key: only the **last expression** in a `-c` script is auto-printed. Earlier
expressions are discarded silently. This is consistent with the REPL (one
expression per prompt).

**2. Why our wrapper doesn't auto-print:**

Our wrapper puts env capture code AFTER the command:

```nushell
let __out = (try { COMMAND } catch { ... })
if ... { $__out | print }
$env | ... | save --force '...'
```

The `$env | ... | save` is the last expression, so nushell auto-prints nothing
useful. The explicit `$__out | print` handles printing but `try { }` captures
stdout, breaking interactive programs.

**3. External commands in the REPL:**

External commands return `PipelineData::ByteStream` which bypasses the
`display_output` hook and writes directly to the terminal. In the REPL, `vim`
works because its ByteStream goes straight to the terminal. In our wrapper,
`try { vim }` or `let x = (vim)` intercepts the ByteStream.

**4. The fundamental tension (no single pattern works):**

Tested all patterns. Results:

| Pattern                          | pwd prints | vim works        | env captured |
| -------------------------------- | ---------- | ---------------- | ------------ |
| `try { CMD } + print` (current)  | Yes        | No               | Yes          |
| `CMD` bare + env capture after   | No         | Yes              | Yes          |
| `CMD \| print` + env capture     | Yes        | Probably no      | Yes          |
| `let r = (do { CMD }); save; $r` | Yes (auto) | No (do captures) | No (scoped)  |
| `save env first; CMD last`       | Yes (auto) | Yes              | No (pre-cmd) |

No single wrapper handles: value-returning commands (pwd, ls), interactive
programs (vim), AND post-command env capture.

**5. Recommendation:**

Switch the nushell wrapper to run commands **bare** (no try/do/let capture).
Accept that value-returning nushell builtins (pwd, ls) won't auto-print their
results in `-c` mode. This fixes vim and all interactive programs.

The trade-off is acceptable because:

- Most commands users type produce output via stdout (echo, grep, cat, git
  status) which works fine bare.
- `ls` in nushell does print via stdout (it's an external command on most
  systems, or nushell's `ls` outputs a table via ByteStream).
- `pwd` not printing is the main loss, but `echo (pwd)` or `pwd | print` works
  as a workaround.
- Interactive programs (vim, nvim, htop, less) are far more important to support
  than auto-printing `pwd`.

**Result:** Pass

The research is complete. No single wrapper pattern solves all cases. The
recommendation is to use bare commands (no capture) for nushell, accepting that
some builtins won't auto-print but interactive programs will work.

#### Conclusion

The nushell `-c` mode auto-print is controlled by the `display_output` hook and
only applies to the last expression. Our wrapper's env capture code after the
command means the command is never the last expression. The `try { }` and
`do { }` patterns capture stdout, breaking interactive programs. The cleanest
fix is to run commands bare and sacrifice auto-print for nushell builtins.

However, the bare wrapper has a significant UX cost: nushell builtins like
`pwd`, `ls`, `date` produce no visible output unless piped to `print`. This
is not how nushell is supposed to work. Additionally, detection-based
approaches (e.g. using `which` to check if a command is built-in) fail for
pipelines containing both built-ins and interactive programs.

The root cause is that nushell's `-c` mode was not designed for our use case.
This motivates investigating a fundamentally different architecture.

### Experiment 2: Research alternative architectures

#### Description

The wrapper-based approach for nushell has fundamental limitations. Research
two alternative architectures that could eliminate the problem entirely:

**Option A: Fork nushell.** Create a fork that is nushell at its core, with
bash/fish/zsh mode added as a "wrapped shell" feature. Since the base shell
IS nushell, there's no wrapper — auto-print, vim, and everything else works
natively.

**Option B: Use nushell as a library.** Import nushell's crates (`nu-cli`,
`nu-engine`, `nu-protocol`, `nu-command`) and build a custom binary that
uses nushell's REPL directly for nushell mode, while keeping our wrapper
approach for bash/fish/zsh.

Both options would make nushell a first-class citizen rather than a wrapped
subprocess.

#### Research tasks

**1. Nushell crate architecture:**

Study the vendored nushell source to understand its crate structure:

- What crates exist and what do they provide?
- Which crates would we need to import for a working nushell REPL?
- Is `nu-cli` designed to be used as a library, or only as part of the
  nushell binary?
- Can we create a custom reedline-based REPL that evaluates nushell commands
  via `nu-engine`?
- What is the public API surface of `nu-cli`, `nu-engine`, `nu-protocol`?

**2. Feasibility of Option A (fork):**

- How large is the nushell codebase? How many crates, how many lines?
- What would we need to modify to add bash mode?
- How hard is it to stay in sync with upstream nushell releases?
- What's the maintenance burden?
- Are there precedents for nushell forks?

**3. Feasibility of Option B (library):**

- Can we instantiate a nushell `EngineState` and evaluate commands without
  running nushell's full REPL?
- Can we capture the resulting env state after evaluation (like our wrapper
  does, but via the Rust API)?
- Can we share reedline between our shell and nushell's evaluation engine?
- What about nushell's config system — can we initialize it programmatically?
- Does nushell expose hooks for command-not-found (useful for AI mode)?

**4. Impact on current architecture:**

For each option, assess:

- What happens to our existing wrapper system for bash/fish/zsh?
- What happens to config.toml and the shell config system?
- What happens to our completions (fish-based)?
- What happens to syntax highlighting?
- What happens to AI mode?
- What happens to history (SQLite)?

**5. Compare and recommend:**

| Dimension | Current (wrapper) | Fork | Library |
|-----------|------------------|------|---------|
| Nushell UX | Broken (no auto-print) | Perfect (native) | Perfect (native) |
| Maintenance | Low | Very high | Medium |
| Codebase size | Small | Huge | Medium |
| Bash/fish/zsh | Works (wrapper) | Needs adding | Works (wrapper) |
| Upstream sync | N/A | Hard (merge conflicts) | Easy (bump version) |

#### Verification

1. Nushell's crate architecture is documented with public API details.
2. Both options are assessed with concrete feasibility analysis.
3. Impact on every existing feature is documented.
4. A clear recommendation is made with reasoning.
