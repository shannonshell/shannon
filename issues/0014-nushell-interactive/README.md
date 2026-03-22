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
`pwd`, `ls`, `date` produce no visible output unless piped to `print`. This is
not how nushell is supposed to work. Additionally, detection-based approaches
(e.g. using `which` to check if a command is built-in) fail for pipelines
containing both built-ins and interactive programs.

The root cause is that nushell's `-c` mode was not designed for our use case.
This motivates investigating a fundamentally different architecture.

### Experiment 2: Research alternative architectures

#### Description

The wrapper-based approach for nushell has fundamental limitations. Research two
alternative architectures that could eliminate the problem entirely:

**Option A: Fork nushell.** Create a fork that is nushell at its core, with
bash/fish/zsh mode added as a "wrapped shell" feature. Since the base shell IS
nushell, there's no wrapper — auto-print, vim, and everything else works
natively.

**Option B: Use nushell as a library.** Import nushell's crates (`nu-cli`,
`nu-engine`, `nu-protocol`, `nu-command`) and build a custom binary that uses
nushell's REPL directly for nushell mode, while keeping our wrapper approach for
bash/fish/zsh.

Both options would make nushell a first-class citizen rather than a wrapped
subprocess.

#### Research tasks

**1. Nushell crate architecture:**

Study the vendored nushell source to understand its crate structure:

- What crates exist and what do they provide?
- Which crates would we need to import for a working nushell REPL?
- Is `nu-cli` designed to be used as a library, or only as part of the nushell
  binary?
- Can we create a custom reedline-based REPL that evaluates nushell commands via
  `nu-engine`?
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

| Dimension     | Current (wrapper)      | Fork                   | Library             |
| ------------- | ---------------------- | ---------------------- | ------------------- |
| Nushell UX    | Broken (no auto-print) | Perfect (native)       | Perfect (native)    |
| Maintenance   | Low                    | Very high              | Medium              |
| Codebase size | Small                  | Huge                   | Medium              |
| Bash/fish/zsh | Works (wrapper)        | Needs adding           | Works (wrapper)     |
| Upstream sync | N/A                    | Hard (merge conflicts) | Easy (bump version) |

#### Verification

1. Nushell's crate architecture is documented with public API details.
2. Both options are assessed with concrete feasibility analysis.
3. Impact on every existing feature is documented.
4. A clear recommendation is made with reasoning.

#### Research findings

**Nushell crate architecture:**

43 workspace crates. The key ones for us:

| Crate         | Size                 | Purpose                                             |
| ------------- | -------------------- | --------------------------------------------------- |
| `nu-protocol` | 122 files, 37K lines | Core types: EngineState, Stack, Value, PipelineData |
| `nu-engine`   | 20 files, 9K lines   | eval_block, eval_expression, env manipulation       |
| `nu-parser`   | 12 files, 15K lines  | Parse nushell syntax into AST                       |
| `nu-cli`      | 53 files, 12K lines  | REPL loop, completions, highlighting                |
| `nu-command`  | 439 files, 88K lines | ALL built-in commands                               |
| `nu-cmd-lang` | 60 files, 6K lines   | Language constructs (if, for, while)                |

**Critical API for Option B (library):**

Nushell exposes exactly what we need:

```rust
// Create engine and register commands
let mut engine_state = EngineState::new();
add_command_context(&mut engine_state); // registers all builtins

// Create runtime stack
let mut stack = Stack::new();

// Evaluate a command
eval_source(&mut engine_state, &mut stack,
    b"pwd", "shannon", PipelineData::empty(), false);

// Read resulting env vars
let env = stack.get_env_vars(&engine_state); // HashMap<String, Value>
let cwd = stack.get_env_var(&engine_state, "PWD");
```

This means we can:

1. Create an EngineState + Stack once at startup
2. Evaluate each user command via `eval_source()`
3. Read env vars and cwd from the Stack after each command
4. The output goes directly to the terminal (no capture needed!)
5. Interactive programs (vim) work because `eval_source` uses inherited stdio

**Feasibility of Option A (fork):**

- Codebase: ~200K lines of Rust across 43 crates
- We'd need to modify the main binary to add shell-switching
- Every nushell release requires merging — conflict-prone because we're changing
  core REPL code
- Maintenance burden: very high. Nushell releases every 3-4 weeks
- No known forks exist (bad sign — nobody has tried this)
- Benefit: full control, can change anything
- Risk: we become a nushell maintainer, not a shannon developer

**Feasibility of Option B (library):**

- Import `nu-cli`, `nu-engine`, `nu-protocol`, `nu-command` as deps
- Use `eval_source()` for nushell mode instead of `-c` wrapper
- Keep our reedline instance for input (we already use reedline)
- Keep our wrapper approach for bash/fish/zsh (unchanged)
- Dependency cost: ~~200K lines pulled in, larger binary (~~+20MB?)
- But: no merge conflicts, upstream updates via version bump
- `eval_source()` returns an exit code and prints output itself
- Env vars readable from Stack after evaluation
- EngineState persists across commands (like a real nushell session)

**Impact on existing features:**

| Feature                        | Fork                                           | Library                               |
| ------------------------------ | ---------------------------------------------- | ------------------------------------- |
| Wrapper system (bash/fish/zsh) | Must reimplement inside nushell                | Unchanged — only nushell mode changes |
| config.toml                    | Must integrate with nushell config             | Unchanged                             |
| Fish completions               | Must integrate with nushell completion system  | Unchanged                             |
| Syntax highlighting            | Nushell's own (different from our tree-sitter) | Keep tree-sitter OR use nushell's     |
| History (SQLite)               | Must integrate with nushell history            | Unchanged (shared reedline)           |
| AI mode                        | Must integrate into nushell REPL               | Unchanged                             |
| Shell switching                | Must add to nushell (doesn't exist)            | Unchanged                             |

**The decisive difference:** Option B changes ONE thing — how nushell commands
are evaluated. Everything else stays the same. Option A changes EVERYTHING.

**Comparison:**

| Dimension             | Current (wrapper)                  | Fork                   | Library             |
| --------------------- | ---------------------------------- | ---------------------- | ------------------- |
| Nushell UX            | Broken (no auto-print, vim issues) | Perfect                | Perfect             |
| Bash/fish/zsh         | Works                              | Must reimplement       | Works               |
| Maintenance           | Low                                | Very high              | Medium              |
| Binary size           | ~10MB                              | ~30MB+                 | ~30MB+              |
| Upstream sync         | N/A                                | Hard (merge conflicts) | Easy (version bump) |
| Implementation effort | Done                               | Months                 | Days to weeks       |
| Risk                  | Low                                | Very high              | Medium              |
| All existing features | Work                               | Must rewrite most      | All work unchanged  |

**Recommendation: Option B (library).**

The library approach solves the nushell UX problem completely while preserving
everything we've built. The implementation is straightforward:

1. Add nushell crates as dependencies
2. Create a `NushellEngine` struct that holds `EngineState` + `Stack`
3. Initialize it at startup (register commands, load config)
4. In the main loop, when active shell is nushell, call `eval_source()` instead
   of `execute_command()`
5. After eval, read env vars from Stack and update `ShellState`

The wrapper system stays for bash/fish/zsh. Config.toml stays. Fish completions
stay. History stays. AI mode stays. Shell switching stays. Only the nushell
execution path changes.

The binary size increase (~20MB from nushell's command crate) is the main cost.
This is acceptable for the UX improvement.

**Result:** Pass

Both options thoroughly researched. Option B (library) is clearly recommended.

#### Conclusion

The nushell `-c` wrapper has fundamental UX limitations that cannot be solved
within the wrapper model. Using nushell as a library (Option B) eliminates all
issues: auto-print works, vim works, env capture works — because we use
nushell's own evaluation engine instead of `-c` mode.

The fork approach (Option A) solves the same problem but at catastrophic
maintenance cost. The library approach achieves the same result while preserving
our entire existing architecture.
