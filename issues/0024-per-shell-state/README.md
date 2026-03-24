+++
status = "open"
opened = "2026-03-24"
+++

# Issue 24: Per-shell internal state with env propagation on switch

## Goal

Research whether shannon can maintain internal state per shell (nushell keeps
its Stack, bash/fish/zsh keep their own state) while propagating environment
variables when switching between shells. Determine feasibility for both the
embedded path (nushell) and the wrapper path (bash/fish/zsh).

## Background

### Current architecture

Shannon maintains a single `ShellState` (env vars, cwd, exit code) that is
shared across all shells. When the user runs a command, shannon injects this
state into the active shell and captures the updated state after execution. This
is the "strings only" boundary — only env vars, cwd, and exit code cross between
shells.

This works but has a limitation: each shell loses its internal state between
commands. Nushell's Stack is rebuilt from scratch each time. Bash doesn't
remember shell variables (non-exported), aliases set during the session, or
shell options.

### Desired architecture

Each shell maintains its own persistent internal state across commands:

- **Nushell:** The `EngineState` + `Stack` already persist across commands (the
  `NushellEngine` struct lives for the session). Nushell variables, custom
  commands, and internal state survive between commands.
- **Bash/fish/zsh:** Currently each command spawns a new subprocess. Internal
  state (shell variables, aliases, functions, options) is lost between commands.

When the user switches shells (Shift+Tab or `/switch`), environment variables
from the previous shell are propagated to the next shell. Internal state stays
with each shell.

### What "internal state" means per shell

**Nushell:** Variables (`$foo`), custom commands (`def`), modules, overlays.
These live in the Stack/EngineState and are already persistent.

**Bash:** Shell variables (non-exported), aliases, functions, shell options
(`set -o`, `shopt`), directory stack (`pushd`/`popd`).

**Fish:** Universal variables, abbreviations, functions defined in session.

**Zsh:** Shell variables, aliases, functions, options (`setopt`), named
directories.

### The two research questions

1. **Nushell (embedded):** Already has persistent state. When switching away
   from nushell, can we extract just the env vars (not internal nushell state)
   to propagate? When switching back, can we inject env vars without disturbing
   nushell's internal state? This likely already works — `inject_state` sets env
   vars and cwd, and nushell's Stack preserves everything else.

2. **Bash/fish/zsh (wrapper):** Currently each command is a new subprocess.
   Internal state is lost. To preserve it, we'd need a persistent subprocess (a
   long-running shell process that we send commands to). This is a fundamental
   change from the current "spawn, run, capture, exit" model. Is this feasible?
   What are the trade-offs? How would env capture work? How would stdio work
   (the user needs to see output and interact with programs)?

## Experiments

### Experiment 1: Verify nushell already preserves internal state

#### Description

Test whether nushell's embedded engine already preserves internal state (shell
variables, custom commands) across shell switches. Since `NushellEngine`
persists for the session and `inject_state` only sets env vars and cwd, internal
nushell state should survive switching away and back.

#### Verification

1. Switch to nushell, set a variable: `let myvar = 5`
2. Verify it persists: `echo $myvar` → `5`
3. Switch to bash (Shift+Tab), then to zsh, then to fish, then back to nushell
4. Verify the variable survived: `echo $myvar` → `5`

**Result:** Pass

Nushell already preserves internal state across shell switches. The
`NushellEngine` struct (EngineState + Stack) lives for the entire session.
`inject_state` only updates env vars and cwd — it doesn't touch nushell's
internal variables, custom commands, or other Stack state.

No code changes needed for nushell.

#### Conclusion

Nushell's embedded architecture gives us per-shell state for free. Research
question 1 is answered: yes, it works already. Remaining question: can we
achieve the same for bash/fish/zsh?

### Experiment 2: Research bash persistent subprocess feasibility

#### Description

Read the vendored bash source code to understand whether shannon can keep a bash
process alive between commands, send commands one at a time, and capture state
after each command. The key question: can we replace the "spawn, run, capture,
exit" wrapper model with a persistent interactive bash subprocess?

#### Findings

**How bash reads commands:**

Bash's main loop (`eval.c:reader_loop()`) calls `read_command()` →
`parse_command()` → `yyparse()`. The parser reads character by character via
`shell_getc()` in `y.tab.c`. Multi-line commands (if/then/fi, heredocs) are
handled by the parser — it shows `PS2` and reads continuation lines until the
command is syntactically complete. Once complete, the command is immediately
executed.

**Interactive vs non-interactive:**

The critical difference: prompts and `PROMPT_COMMAND` only run in interactive
mode. The check is `SHOULD_PROMPT()` in `y.tab.c`:

```c
#define SHOULD_PROMPT() \
  (interactive && (bash_input.type == st_stdin || bash_input.type == st_stream))
```

Our current wrapper uses `bash -c "..."` which is non-interactive. A persistent
subprocess would need `bash -i` to get access to the hooks we need.

**Available hooks for command boundaries:**

- `PS0` (bash 4.4+): Expanded and printed to stderr _after_ parsing but _before_
  execution. Good for "command received" signal.
- `PROMPT_COMMAND`: Runs _after_ command execution and _before_ the next prompt.
  This is the key hook for state capture — we could emit env vars here.
- `trap DEBUG`: Fires before each command. Less useful than PROMPT_COMMAND for
  our purpose.

The combination of `PS0` (pre-execution marker) and `PROMPT_COMMAND`
(post-execution state capture) gives us command boundary detection.

**The fundamental problem: PTY requirement.**

For bash to be truly interactive (enabling `PROMPT_COMMAND`, `PS0`, signal
handling), it needs a TTY. The parent process would need to:

1. Create a pseudo-terminal (PTY) pair
2. Spawn bash with the slave end as its controlling terminal
3. Write commands to the master end
4. Read output from the master end
5. Parse output to separate user-visible output from state-capture markers
6. Handle terminal window size, raw mode, and signal forwarding

This is essentially what `expect`, `screen`, and `tmux` do. It's a significant
amount of complexity.

**The synchronization problem:**

Even with a PTY, the parent needs to know when a command is done:

1. Parent writes command to master PTY
2. Bash receives it, `PS0` fires (marker on stderr: "command starting")
3. Command runs, output goes to terminal
4. Command finishes, `PROMPT_COMMAND` fires (captures env to temp file or emits
   state on stderr)
5. Bash prints `PS1` prompt
6. Parent detects the prompt and reads captured state

This requires parsing the output stream to find markers mixed in with command
output. Fragile — a command could print text that matches the markers.

**Alternative: the wrapper model is actually well-suited.**

The current wrapper model sidesteps all of these problems:

- No PTY needed (child inherits shannon's terminal directly)
- No output parsing (command output goes straight to the user)
- Clear command boundaries (subprocess starts, runs, exits)
- State capture via temp file (clean separation from output)

The trade-off is: internal bash state (shell variables, aliases, functions) is
lost between commands.

**What internal state matters in practice?**

- **Shell variables** (`FOO=bar` without `export`): Rare in interactive use.
  Most users export variables they care about.
- **Aliases**: Typically set in `.bashrc`, which we already source via init
  files.
- **Functions**: Same as aliases — usually in dotfiles.
- **Shell options** (`set -o`, `shopt`): Rarely changed interactively.
- **Directory stack** (`pushd`/`popd`): Would be lost. Uncommon in practice.

The most impactful loss is aliases and functions defined during the session, but
this is an edge case.

#### Result

Research complete. Findings documented above.

**Persistent bash subprocess:** Feasible but requires PTY management, output
parsing, and marker-based synchronization. High complexity, fragile.

**Current wrapper model:** Loses internal state but is simple, robust, and
handles all the hard problems (stdio, signals, env capture) cleanly.

#### Conclusion

A persistent bash subprocess is technically possible using PTY +
`PROMPT_COMMAND` for state capture, but the complexity is high and the benefits
are marginal. The internal state that would be preserved (shell variables,
aliases, functions set during the session) is rarely needed in practice — most
of it comes from dotfiles which are already sourced.

The current wrapper model is a good design for bash/fish/zsh. The nushell
embedded model is where per-shell state really shines because it's free.

Next step: decide whether the PTY approach is worth pursuing, or if there's a
lighter-weight alternative (e.g., persisting specific state like aliases between
wrapper invocations by capturing and re-sourcing them).

### Experiment 3: Research embedding brush (Rust-based bash) like nushell

#### Description

[Brush](https://github.com/reubeno/brush) is a Rust reimplementation of bash.
Since it's written in Rust, it might be embeddable as a library — the same way
we embed nushell via `eval_source()`. This could give us persistent bash state
without PTY complexity.

Research questions:

1. **Crate structure:** Does brush expose a library crate? What's the public
   API? Can we create a shell instance, send it commands, and read state back?
2. **State model:** How does brush represent env vars, shell variables, cwd, and
   exit code? Can we read/write these between commands?
3. **Stdio:** When brush runs a command, does output go to the real terminal?
   Can interactive programs (vim, less, ssh) work?
4. **External commands:** Does brush spawn real subprocesses for external
   commands (ls, git, npm)? Or does it try to interpret everything internally?
5. **Compatibility:** How complete is brush's bash compatibility? Can it run
   real-world commands reliably?
6. **Signal handling:** Does brush handle SIGINT for subprocesses? How does it
   interact with our signal-hook setup?
7. **Integration pattern:** What would a `BrushEngine` (analogous to
   `NushellEngine`) look like? What crates would we depend on?

#### Method

Read the vendored brush source at `vendor/brush/`. Examine:

- `Cargo.toml` files for crate structure and public API surface
- `brush-core/` or similar for the shell engine
- How commands are evaluated (eval loop, AST execution)
- How env/variables/cwd are stored and accessible
- How external commands are spawned
- Any existing examples of embedding brush as a library

#### Findings

**Crate structure:**

Brush is a workspace with 10 crates. The key one is `brush-core` — an explicit
library crate with a public API designed for embedding. Other crates include
`brush-parser`, `brush-builtins`, `brush-interactive` (REPL UI using reedline),
and `brush-shell` (the binary).

**Public API — shell creation:**

```rust
let shell = brush_core::Shell::builder()
    .build()   // async
    .await?;
```

Builder supports: `.working_dir()`, `.var()`, `.builtin()`, `.interactive()`,
`.do_not_inherit_env()`, `.enable_option()`, `.rc()`, `.profile()`, etc.

**Public API — command execution:**

```rust
let result = shell.run_string(
    "echo hello",
    &source_info,
    &shell.default_exec_params(),
).await?;
// result.exit_code, result.is_success(), result.next_control_flow
```

Returns `ExecutionResult` with exit code and control flow (normal, break, exit).

**State access — fully readable and writable:**

```rust
shell.env()          // &ShellEnvironment (read env vars)
shell.env_mut()      // &mut ShellEnvironment (write env vars)
shell.working_dir()  // &Path
shell.working_dir_mut()  // &mut PathBuf
shell.last_exit_status() // u8
shell.set_last_exit_status(code)
```

Env vars can be iterated with `.iter_exported()`. Shell variables (non-exported)
are also accessible. This is cleaner than nushell's API.

**External commands:** Spawns real subprocesses via `tokio::process::Command`.
They inherit the parent's stdio — output goes directly to the terminal.
Interactive programs (vim, less, ssh) should work.

**Async requirement:** Everything is async, requiring a tokio runtime. This is
the main integration cost. Shannon already depends on tokio (for rig-core/AI),
so this is not a new dependency.

**Dependencies:** ~29 transitive deps. Tokio, nix, chrono, clap, regex. Moderate
weight, similar to the nushell crates.

**Examples:** `brush-core/examples/call-func.rs` demonstrates creating a shell,
running commands, and invoking functions. `custom-builtin.rs` shows extending
with custom builtins.

**Compatibility:** Brush aims for bash compatibility. It has extensive test
suites and compatibility tests. Less mature than real bash, but actively
developed.

#### Comparison with nushell embedding

| Aspect         | Nushell                        | Brush                             |
| -------------- | ------------------------------ | --------------------------------- |
| Creation       | Sync: `EngineState::new()`     | Async: `.builder().build().await` |
| Execute        | Sync: `eval_source()`          | Async: `run_string()`             |
| State access   | Stack + EngineState (indirect) | Direct methods on Shell           |
| Env vars       | `stack.add_env_var()`          | `shell.env_mut()`                 |
| Exit code      | i32 from eval_source           | `ExecutionResult.exit_code`       |
| External cmds  | Real subprocesses              | Real subprocesses                 |
| Stdio          | Inherited                      | Inherited                         |
| Async required | No                             | Yes (tokio)                       |

**What a `BrushEngine` would look like:**

```rust
pub struct BrushEngine {
    shell: brush_core::Shell,
    runtime: tokio::runtime::Runtime,
}

impl BrushEngine {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let shell = runtime.block_on(
            brush_core::Shell::builder().build()
        ).unwrap();
        BrushEngine { shell, runtime }
    }

    pub fn execute(&mut self, command: &str) -> ShellState {
        let result = self.runtime.block_on(
            self.shell.run_string(command, &source_info, &params)
        );
        // Read state from self.shell.env(), working_dir(), etc.
    }
}
```

The pattern mirrors `NushellEngine` almost exactly — create once, execute
commands, capture state. The async wrapping with `block_on` is the only
difference.

**Result:** Research complete.

Brush is embeddable as a library. `brush-core` provides a clean public API for
creating a shell, executing commands, and reading/writing state. External
commands spawn real subprocesses with inherited stdio. The only integration cost
is async (tokio), which shannon already has.

#### Conclusion

Brush can be embedded in shannon the same way nushell is. The API is actually
cleaner than nushell's — direct methods for env, cwd, and exit code instead of
reaching into Stack/EngineState internals.

Key advantages over the wrapper model:

- Persistent state (shell vars, aliases, functions survive between commands)
- No temp file for env capture (read directly from Shell struct)
- No wrapper script template needed
- Signal handling can be integrated (like nushell's Signals)

Key risks:

- Brush is less mature than bash (compatibility gaps)
- Async requirement adds complexity
- Unknown how well interactive programs (vim, less) work through the embedded
  engine vs direct subprocess

Next step: build a proof-of-concept `BrushEngine` and test it with real
commands.

### Experiment 4: Proof-of-concept BrushEngine

#### Description

Build a minimal `BrushEngine` that embeds brush-core, analogous to
`NushellEngine`. Add it to shannon, wire it into the REPL as the "bash" shell
(replacing the wrapper), and verify basic commands work.

This is a spike — the goal is to prove the integration works, not to ship it. If
it works, we'll have two embedded shells (nushell + brush) with persistent
state, and the wrapper model becomes a fallback for fish/zsh.

#### Changes

**`shannon/Cargo.toml`**:

- Add `brush-core` dependency (from crates.io or path to vendor)

**`shannon/src/brush_engine.rs`** (new file):

- `BrushEngine` struct holding `brush_core::Shell` and a tokio `Runtime`
- `new()` — create runtime, build shell
- `inject_state(&mut self, state: &ShellState)` — set env vars and cwd
- `execute(&mut self, command: &str) -> ShellState` — run command via
  `runtime.block_on(shell.run_string(...))`, capture env/cwd/exit code

**`shannon/src/lib.rs`**:

- Add `pub mod brush_engine;`

**`shannon/src/repl.rs`**:

- In `run_command`, add a branch for `shell.0 == "brush"` that uses
  `BrushEngine` (similar to the nushell branch)

**`shannon/src/main.rs`**:

- Create `BrushEngine` at startup
- Pass it to `repl::run`

**Integration test**:

- `test_brush_echo`, `test_brush_env_capture`, `test_brush_cwd_capture` — mirror
  the existing nushell tests

#### Verification

1. `cargo build` succeeds (brush-core compiles and links).
2. `cargo test` — new brush tests pass (echo, env capture, cwd).
3. Manual: switch to brush in shannon, run `echo hello`, `export FOO=bar`,
   `cd /tmp`, verify state persists between commands.
4. Manual: run `ls`, `git status` — external commands work with real stdio.
5. Manual: set a shell variable `FOO=bar` (no export), run another command,
   verify it persists (this is the whole point — wrapper model loses this).

**Result:** Pass

All verification steps confirmed. 95 tests pass (71 unit + 24 integration,
including 4 new brush tests). Brush is embedded alongside nushell as a fully
functional shell with persistent state.

Implementation notes:

- `brush-core` 0.4 + `brush-builtins` 0.1 from crates.io
- `BrushEngine` mirrors `NushellEngine`: create once, inject state, execute,
  capture state
- Async wrapping via `runtime.block_on()` (tokio runtime created once)
- Env capture uses a heuristic: track known keys from inject + parse command
  text for `export` patterns, then query brush's `env_str()`/`env_var()` for
  each key. Works for the PoC but brush-core would ideally expose
  `iter_exported()`.
- Builtins (export, cd, etc.) require `brush-builtins` crate with
  `default_builtins(BashMode)`

#### Conclusion

Brush can be embedded in shannon exactly like nushell. The PoC is fully
functional: echo, env capture, cwd, external commands, and state persistence all
work. Two embedded shells (nushell + brush) now coexist with persistent
per-shell state, while fish/zsh remain on the wrapper model.

### Experiment 5: Vendor brush-core source for full API access

#### Description

The crates.io version of brush-core (0.4.0) doesn't expose `env()` or
`env_mut()` on Shell, so we can't iterate exported env vars. The git main branch
already has these as public. We'll fork brush, maintain our patches on a
`shannon` branch, and add it as a submodule.

**Repo setup:**

1. Fork `reubeno/brush` → `shannonshell/shannon_brush`
2. Add upstream remote:
   `git remote add upstream https://github.com/reubeno/brush`
3. Create a `shannon` branch off upstream's main
4. Apply our patches on the `shannon` branch:
   - Rename crates: `brush-core` → `shannon-brush-core`, etc. (needed for
     crates.io — can't publish under someone else's crate name)
   - Any API changes we need (though `env()`/`env_mut()` are already public on
     main)
5. Add as submodule at `brush/` in the shannon repo, pinned to `shannon` branch

**Upstream sync:**

1. `git fetch upstream`
2. `git rebase upstream/main` on the `shannon` branch
3. Our patches replay on top of the new upstream base
4. Resolve conflicts if any (our changes are minimal — mostly crate renames)
5. Publish new version of `shannon-brush-*` crates
6. Update submodule pin in shannon

**Exit strategy:**

When upstream brush publishes `env()` / `env_mut()` as public:

1. Switch shannon's Cargo.toml to official `brush-core` / `brush-builtins`
2. Remove the submodule
3. Archive the fork

#### Changes

**Fork and submodule:**

- Create `shannonshell/shannon_brush` on GitHub
- Add submodule at `brush/` in the shannon repo
- On the `shannon` branch: rename crate packages to `shannon-brush-*`

**`shannon/Cargo.toml`:**

- Replace `brush-core = "0.4"` with
  `shannon-brush-core = { path = "brush/brush-core" }`
- Replace `brush-builtins = "0.1"` with
  `shannon-brush-builtins = { path = "brush/brush-builtins" }`
- (Use path deps for dev; publish `shannon-brush-*` to crates.io and switch to
  version deps for shannon releases)

**`shannon/src/brush_engine.rs`:**

- Update imports: `brush_core` → `shannon_brush_core`, `brush_builtins` →
  `shannon_brush_builtins`
- Replace heuristic env capture with `shell.env().iter_exported()`
- Remove `known_keys` tracking and `discover_new_keys` heuristic
- Use `shell.env_mut().set_global()` directly in inject_state

#### Verification

1. Submodule cloned and checked out on `shannon` branch.
2. `cargo build` succeeds with path dependencies.
3. `cargo test` — all tests pass.
4. `export FOO=bar` in brush, switch to nushell, `$env.FOO` shows `bar`.
5. Source a script that exports vars — they're captured correctly.

**Result:** Fail

Path dependencies from the submodule create a shared `Cargo.lock` between
shannon and brush. Brush's git main requires newer transitive deps (libc
0.2.183 via nix 0.31.2 and whoami 2.1.1) that conflict with nushell 0.111's
exact pin on `libc =0.2.178`. Cargo can't resolve a single libc version that
satisfies both.

Downgrading brush's deps would work but defeats the purpose — we want to track
upstream, not maintain a divergent dependency tree.

#### Conclusion

Path dependencies between shannon and brush don't work due to transitive
dependency conflicts between brush (git main) and nushell. The submodule
approach is wrong for this — it forces a shared resolver.

The correct approach: publish the fork to crates.io. When brush is a crates.io
dependency (not a path dep), it gets its own resolved dependency tree. No
conflicts. This is exactly how brush-core 0.4.0 from crates.io already works
with nushell — they each resolve their own libc version independently.

Next experiment: fork brush at latest main, rename packages to `shannon-brush-*`
(the only change — the API we need is already public on main), publish to
crates.io, and use version deps in shannon.

### Experiment 6: Fork, rename, publish shannon-brush to crates.io

#### Description

Fork brush, rename the crate packages, publish to crates.io, and use them in
shannon as normal version dependencies. The submodule is just for convenience
— it lives in the repo so we can make changes and publish, but shannon's
Cargo.toml points to crates.io, not a path.

**What we change in the fork:**

Only crate names. The `shannon` branch renames:
- `brush-parser` → `shannon-brush-parser`
- `brush-core` → `shannon-brush-core`
- `brush-builtins` → `shannon-brush-builtins`

No API changes needed — `env()` and `env_mut()` are already public on main.

Internal dependency references use `package = "shannon-brush-*"` so Rust import
names (`brush_core`, `brush_parser`) stay the same. No source code changes.

**Submodule role:**

The submodule at `brush/` is where we maintain the fork. It is NOT a path
dependency. Shannon depends on the published crates.io versions. The submodule
exists so we can:
- Make changes to the fork
- Publish new versions
- Keep the fork in sync with upstream

**Workflow to publish:**

1. `cd brush/`
2. Make changes on `shannon` branch
3. `cargo publish -p shannon-brush-parser`
4. `cargo publish -p shannon-brush-core`
5. `cargo publish -p shannon-brush-builtins`
6. Update versions in `shannon/Cargo.toml`

**Upstream sync:**

1. `cd brush/ && git fetch upstream && git rebase upstream/main`
2. Resolve conflicts (only crate name renames)
3. Publish new versions

#### Changes

**In the fork (`brush/` submodule):**
- Rename package names in Cargo.toml files (3 crates + internal references)
- Update version numbers if needed for initial publish

**`shannon/Cargo.toml`:**
- Replace `brush-core = "0.4"` with
  `shannon-brush-core = "0.4"` (or whatever version we publish)
- Replace `brush-builtins = "0.1"` with
  `shannon-brush-builtins = "0.1"`

**`shannon/src/brush_engine.rs`:**
- Update imports: `brush_core` → `shannon_brush_core`,
  `brush_builtins` → `shannon_brush_builtins`
- Replace heuristic env capture with `shell.env().iter_exported()`
- Remove `known_keys` tracking and `discover_new_keys` heuristic

#### Verification

1. `shannon-brush-*` crates published on crates.io.
2. Submodule exists at `brush/` but is NOT a path dependency.
3. `cargo build` succeeds with crates.io deps.
4. `cargo test` — all tests pass.
5. `export FOO=bar` in brush, switch to nushell, `$env.FOO` shows `bar`.
6. Source a script that exports vars — captured correctly (no heuristic).

**Result:** Fail

The `shannon-brush-*` crates were forked, renamed, and published to crates.io.
The submodule is set up correctly. But `cargo build` fails with the same libc
conflict as experiment 5.

The root cause was a wrong assumption: we believed crates.io deps get
independent dependency resolution, unlike path deps. They don't. Cargo resolves
all dependencies for a crate together in a single resolution pass, regardless
of whether they come from crates.io or paths.

The original `brush-core 0.4.0` from crates.io worked because it was published
with older transitive deps (nix 0.29, whoami 1.x) that were compatible with
nushell 0.111's `libc =0.2.178` pin. Our fork is from brush's latest main,
which uses nix 0.31.2 and whoami 2.1.1, both requiring libc >= 0.2.181. These
conflict with nushell's exact pin.

#### Conclusion

The libc version conflict is between nushell 0.111 (pins `libc =0.2.178`) and
brush's latest main (requires `libc >= 0.2.181` via nix 0.31.2 and whoami
2.1.1). This conflict exists regardless of whether brush is a path dep or a
crates.io dep — Cargo resolves them together either way.

Options:
1. Downgrade brush's deps to match nushell's libc pin (rejected — we want to
   track upstream brush, not maintain a divergent dependency tree)
2. Fork nushell to relax its libc pin
3. Wait for nushell to update (nushell 0.112+ may use a newer libc)
4. Use the original brush-core 0.4.0 from crates.io (works, but no `env()`
   API — back to the heuristic)

### Experiment 7: Fork nushell, shared workspace with brush and shannon

#### Description

The libc conflict is caused by nushell pinning `libc = "=0.2.178"` in its
workspace Cargo.toml. Brush's latest main requires `libc >= 0.2.181`. These
are incompatible under a single resolver.

Fix: fork nushell, relax the libc pin, rename all crates to `shannon-nu-*`,
and put everything in a shared workspace. Shannon, brush, and nushell all
resolve together with compatible deps.

**Repos:**
- `shannonshell/shannon` — main repo (already exists)
- `shannonshell/shannon_brush` — brush fork (already exists as submodule)
- `shannonshell/shannon_nushell` — nushell fork (just created)

**Nushell fork changes (on `shannon` branch):**
1. Relax `libc = "=0.2.178"` → `libc = "0.2"` in workspace Cargo.toml
2. Rename all 26 nu-* crate packages to `shannon-nu-*`
3. Add `package = "shannon-nu-*"` to all internal dependency references
   so Rust import names stay unchanged

**Shannon repo changes:**
1. Add `shannonshell/shannon_nushell` as submodule at `nushell/`
2. Create a root workspace Cargo.toml that includes all three:
   `shannon/`, `brush/brush-{core,builtins,parser}`,
   `nushell/crates/nu-*`
3. Update `shannon/Cargo.toml` to use path deps for both nushell and brush
4. Update `brush_engine.rs` to use `shannon_brush_core` imports and
   `shell.env().iter_exported()`
5. Update `nushell_engine.rs` imports if crate names change

**Workspace structure:**
```
shannon/              (repo root)
├── Cargo.toml        (workspace root)
├── shannon/          (shannonshell crate)
├── brush/            (submodule → shannonshell/shannon_brush)
│   ├── brush-core/
│   ├── brush-builtins/
│   └── brush-parser/
└── nushell/          (submodule → shannonshell/shannon_nushell)
    └── crates/
        ├── nu-cli/
        ├── nu-command/
        └── ... (26 crates)
```

#### Changes

**In nushell fork (`nushell/` submodule):**
- Relax libc pin: `"=0.2.178"` → `"0.2"`
- Rename 26 crate packages to `shannon-nu-*`
- Update all internal nu-* dependency references with `package = "shannon-nu-*"`

**In brush fork (`brush/` submodule):**
- Update brush's nu-* references if any (likely none — brush doesn't depend on
  nushell)

**New file: `Cargo.toml` at repo root:**
- Workspace definition listing all members

**`shannon/Cargo.toml`:**
- Replace crates.io deps with path deps to submodule crates

**`shannon/src/brush_engine.rs`:**
- Use `shannon_brush_core` imports
- Replace heuristic with `shell.env().iter_exported()`

**`shannon/src/nushell_engine.rs`:**
- Update imports if crate names change in Rust code (they shouldn't — we use
  `package = "shannon-nu-*"` to keep import names as `nu_protocol` etc.)

#### Verification

1. All submodules cloned, `shannon` branches checked out.
2. `cargo build` from workspace root succeeds.
3. `cargo test` — all shannon tests pass.
4. Manual: brush `export FOO=bar`, switch to nushell, `$env.FOO` shows `bar`.
5. Manual: both shells handle Ctrl+C correctly (no regressions).

**Result:** Fail

The libc pin was successfully relaxed and dependency resolution passed — no more
libc conflict. But the nushell fork is at the latest git main, which uses
reedline features (`EditCommandDiscriminants`, `MoveLineUp`, `MoveLineDown`)
that don't exist in reedline 0.46 from crates.io. Nushell's `[patch.crates-io]`
section overrides reedline with a git dependency to `nushell/reedline` main
branch. This means the nushell fork requires an unreleased version of reedline.

Reedline is a separate repo (`nushell/reedline`), not part of the nushell
monorepo. It needs the same fork treatment: fork, rename to
`shannon-reedline`, publish to crates.io, and use it as a version dependency.

#### Conclusion

The nushell fork requires bleeding-edge reedline (not yet on crates.io).
Reedline must also be forked, renamed, and published. Three forks total:
nushell, brush, reedline.

### Experiment 8: Fork reedline, complete the shared workspace

#### Description

Same as experiment 7, plus fork reedline. Reedline is a single crate (no
workspace, no sub-crates) so the fork is trivial: rename to
`shannon-reedline`, publish, and point both shannon and nushell at it.

**All four repos:**
- `shannonshell/shannon` — main repo
- `shannonshell/shannon_brush` — brush fork (submodule at `brush/`)
- `shannonshell/shannon_nushell` — nushell fork (submodule at `nushell/`)
- `shannonshell/shannon_reedline` — reedline fork (submodule at `reedline/`)

**Reedline fork changes (on `shannon` branch):**
1. Rename package: `reedline` → `shannon-reedline`

**Nushell fork additional changes:**
1. Replace `[patch.crates-io]` reedline git override with a path dep to the
   reedline submodule
2. Update workspace.dependencies: `reedline = "0.46.0"` → path dep to
   `shannon-reedline`

**Shannon Cargo.toml:**
1. Replace `reedline = { version = "0.46.0", ... }` with path dep to
   `shannon-reedline` submodule

Since both shannon and nushell depend on reedline, they must agree on the same
crate. With path deps pointing to the same submodule, they resolve to the same
instance.

#### Changes

**Reedline fork (`reedline/` submodule):**
- Rename `name = "reedline"` → `name = "shannon-reedline"` in Cargo.toml

**Nushell fork (`nushell/` submodule):**
- All changes from experiment 7 (libc pin, 40 crate renames, 206 dep refs)
- Replace `[patch.crates-io]` reedline git override with:
  `reedline = { path = "../reedline", package = "shannon-reedline" }`
- Update `[workspace.dependencies]`:
  `reedline = { path = "../reedline", package = "shannon-reedline" }`

**Shannon (`shannon/Cargo.toml`):**
- Path deps to nushell and brush submodules (from experiment 7)
- Replace `reedline = { version = "0.46.0", features = ["sqlite"] }` with
  `reedline = { path = "../reedline", package = "shannon-reedline", features = ["sqlite"] }`

**`shannon/src/brush_engine.rs`:**
- Replace heuristic with `shell.env().iter_exported()` (from experiment 7)

#### Verification

1. All three submodules cloned, `shannon` branches checked out.
2. `cargo build` succeeds (no libc conflict, no reedline mismatch).
3. `cargo test -p shannonshell` — all tests pass.
4. Manual: brush `export FOO=bar`, switch to nushell, `$env.FOO` shows `bar`.
5. Manual: Ctrl+C works in both shells.

**Result:** Partial

All three forks are set up as submodules (nushell, brush, reedline). The libc
pin is relaxed, all crates renamed, path deps wired up. `cargo build` and all
95 tests pass. Basic commands work in both nushell and brush. The heuristic env
capture in brush is replaced with `shell.env().iter_exported()`.

Two regressions:
1. **Nushell Ctrl+C broken:** `sleep 10sec` + Ctrl+C prints `^C` but doesn't
   kill the process. The signal-hook integration from issue 22 may not be
   compatible with the newer nushell version from the fork.
2. **Brush Ctrl+C broken:** `sleep 10` + Ctrl+C same behavior — prints `^C`
   but process continues. Brush has no signal integration yet (noted as a
   known gap from experiment 4).

Three additional minor fixes were needed for API changes:
- `brush_core::SourceInfo` parameter added to `run_string()`
- `ExecutionExitCode::BrokenPipe` variant added
- `reedline::Signal` has new variants (added wildcard match)

#### Conclusion

The core goal — shared workspace with all three forks, proper env capture via
`iter_exported()`, no dependency conflicts — is achieved. Ctrl+C handling needs
to be fixed in a follow-up experiment.

### Experiment 9: Fix Ctrl+C with reedline's break_signal

#### Description

Reedline's git main added `with_break_signal(Arc<AtomicBool>)` (commit
`5c2f105`). When configured, reedline polls non-blocking instead of blocking
on `event::read()`. When the AtomicBool is set, reedline returns
`Signal::ExternalBreak`.

Shannon already has the interrupt `Arc<AtomicBool>` (from issue 22's
signal-hook setup) — it just needs to pass it to reedline. Without it,
reedline blocks forever after a subprocess is killed by Ctrl+C, making it
appear that Ctrl+C doesn't work.

#### Changes

**`shannon/src/repl.rs`** — `build_editor()`:
- Add `interrupt: &Arc<AtomicBool>` parameter
- Add `.with_break_signal(interrupt.clone())` to the reedline builder chain

**`shannon/src/repl.rs`** — all `build_editor()` call sites:
- Pass the `interrupt` Arc

**`shannon/src/repl.rs`** — `Signal::ExternalBreak` handling:
- The existing `Ok(_) => continue` wildcard already handles this, but make it
  explicit for clarity:
  `Ok(Signal::ExternalBreak(_)) => continue`

#### Verification

1. `cargo build` succeeds.
2. `cargo test` — all tests pass.
3. Nushell: `sleep 10sec` + Ctrl+C → process killed, prompt returns.
4. Brush: `sleep 10` + Ctrl+C → process killed, prompt returns.
5. Bash (wrapper): `sleep 10` + Ctrl+C → still works (no regression).

**Result:** Fail

Build succeeds, all 95 tests pass, but Ctrl+C behavior is unchanged. Passing
the interrupt Arc to reedline via `with_break_signal()` and handling
`Signal::ExternalBreak` did not fix the issue. The problem is deeper than
reedline's event loop — the signal is not reaching nushell's or brush's
subprocess at all.

#### Conclusion

The break_signal feature only helps reedline wake up from its event loop when
an external interrupt occurs. It doesn't fix the underlying problem: the
subprocess running inside nushell or brush is not receiving SIGINT. The issue
is in how signals are delivered during embedded execution, not in how reedline
polls for events.

### Experiment 10: Debug logging to trace signal delivery

#### Description

Add temporary debug logging to `/tmp/shannon-debug.log` at every point in the
signal chain. Run `sleep 10sec` in nushell, press Ctrl+C, and read the log to
find exactly where the chain breaks.

#### Log points

1. **signal-hook handler fires** — in `repl.rs` after `signal_hook::flag::register`,
   add a second handler via `signal_hook::low_level::register` that writes a
   log line when SIGINT arrives.
2. **nushell engine execute starts** — in `nushell_engine.rs` `execute()`,
   log before `eval_source`.
3. **nushell signals reset** — log after `reset_signals()`.
4. **nushell signals interrupted check** — log the state of the interrupt
   AtomicBool before and after `eval_source`.
5. **run_command entry** — in `repl.rs` `run_command()`, log which shell path
   is taken (nushell/brush/wrapper).
6. **ExternalBreak received** — log when reedline returns ExternalBreak.

All logs go to `/tmp/shannon-debug.log` with timestamps.

#### Changes

**`shannon/src/repl.rs`:**
- Add a debug log helper function that appends to `/tmp/shannon-debug.log`
- Add log calls at signal registration, run_command entry, and
  ExternalBreak handling

**`shannon/src/nushell_engine.rs`:**
- Add log calls before/after reset_signals and eval_source
- Log the interrupt AtomicBool value

#### Verification

1. `cargo build` succeeds.
2. Run shannon, switch to nushell, run `sleep 10sec`.
3. In another terminal: `tail -f /tmp/shannon-debug.log`.
4. Press Ctrl+C.
5. Read the log to determine where the signal chain breaks.
