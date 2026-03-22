+++
status = "open"
opened = "2026-03-22"
+++

# Issue 8: Shell configuration (rc files)

## Goal

Determine how shannon should handle shell configuration — rc files that set
PATH, aliases, environment variables, and other startup state. Currently shannon
runs shells with no rc file, which means users get a bare environment missing
their PATH additions, aliases, and other customizations.

## Background

Every shell loads configuration files at startup:

- **bash** — `~/.bashrc` (interactive non-login), `~/.bash_profile` or
  `~/.profile` (login)
- **nushell** — `~/.config/nushell/config.nu` and `~/.config/nushell/env.nu`
- **zsh** — `~/.zshrc` (interactive), `~/.zprofile` (login)
- **fish** — `~/.config/fish/config.fish`

Shannon currently runs `bash -c '<wrapper>'` and `nu -c '<wrapper>'`, which
skips all rc files. This means:

1. **PATH is incomplete** — additions from `.bashrc` or `env.nu` are missing.
   Tools installed via homebrew, cargo, nvm, pyenv, etc. may not be found.
2. **Aliases are missing** — user-defined aliases don't exist.
3. **Prompt customizations are irrelevant** — shannon owns the prompt, but
   environment setup in rc files matters.
4. **Shell options are unset** — things like `shopt` settings in bash or nushell
   config options.

### The design tension

There are two approaches, each with trade-offs:

**Option A: Source the user's existing rc files.**

- Pros: Works immediately. User's PATH, env vars, and setup are all present.
  Zero configuration needed.
- Cons: User rc files may do things that conflict with shannon — set prompts,
  configure completions, start background processes, print welcome messages.
  Some rc files assume a fully interactive session and may break under
  `bash -c`.

**Option B: Shannon-specific rc files (e.g. `~/.config/shannon/bashrc`).**

- Pros: Clean separation. Users can set up exactly what they need for shannon
  without worrying about conflicts. Shannon controls the experience.
- Cons: Requires manual setup. New users get a broken PATH until they create the
  file. Duplication of common config between the user's regular rc and shannon's
  rc.

**Option C: Hybrid — source the user's rc file, then source a shannon
override.**

- Pros: Gets the user's environment by default, allows shannon-specific tweaks.
- Cons: Still inherits rc file conflicts. More complex.

### What other tools do

- **tmux** — inherits the environment of the launching shell. Shells inside tmux
  load their own rc files normally.
- **VS Code terminal** — runs a login shell to get the full environment, then
  sources rc files normally.
- **nushell (standalone)** — loads `env.nu` then `config.nu` on every startup.

### Responsibility question

Shannon is not a new language — it delegates to real shells. But it owns the
environment that gets passed to those shells. `ShellState::from_current_env()`
seeds the initial state from the launching shell's environment, and then
`executor.rs` clears the subprocess env (`env_clear()`) and re-injects shannon's
state. This means:

- If the user launches shannon from a fully-configured terminal, the initial
  environment is complete (PATH, etc. from their `.bashrc` or `.zshrc`).
- But if shannon is launched from a bare context (e.g. a `.desktop` file, a
  login shell, or a service), the environment may be minimal.
- Sub-shells never load their own rc files because we use `bash -c` / `nu -c`,
  which skips them.

This makes it shannon's responsibility to ensure the environment is usable,
because shannon controls what the sub-shells see.

### Option D: Shannon startup script

A new option emerged from discussion: shannon could run a startup script at
launch — written in any shell shannon supports — to configure the environment.
For example, `~/.config/shannon/env.sh` or `~/.config/shannon/env.nu`. This
script would run once at startup, its resulting env vars would be captured, and
that state would seed `ShellState` for the session.

This is different from per-shell rc files. It's a single, shell-agnostic
environment setup that runs once. The user could also use a simple `.env` file
for the common case of just setting variables.

### Questions to answer

1. Where does shannon's initial environment actually come from today? What does
   `ShellState::from_current_env()` capture, and what's missing compared to a
   fully-configured shell?
2. What do the sub-shells see? Since `executor.rs` calls `env_clear()` then
   `envs(&state.env)`, the sub-shells only see what shannon gives them. Does
   this mean the sub-shells' own rc files are irrelevant even if we could load
   them?
3. How do bash and nushell handle rc files under `-c` mode? Does `bash -c` skip
   `.bashrc`? Does `nu -c` skip `config.nu`? What flags exist to force rc
   loading (e.g. `bash --rcfile`, `bash -l`)?
4. If we run a startup script once at launch, how do we capture its resulting
   environment? Can we reuse the existing wrapper/capture mechanism from
   `executor.rs`?
5. Should shannon support a `.env` file (simple key=value) as well as a startup
   script (full shell)?
6. What's the interaction with the launching shell's environment? Should shannon
   merge its startup script output with the inherited env, or replace it
   entirely?

## Experiments

### Experiment 1: Research environment flow and rc file behavior

#### Description

Trace exactly how environment variables flow through shannon today, and
determine what each shell does with rc files under `-c` mode. The goal is to
understand the current behavior well enough to make a design decision about
where configuration responsibility belongs.

#### Research tasks

**1. Trace shannon's current environment flow:**

- Read `ShellState::from_current_env()` — what does `std::env::vars()` capture?
- Read `execute_command()` — confirm that `env_clear()` + `envs(&state.env)`
  means sub-shells see only what shannon provides.
- Launch shannon from a terminal and inspect what env vars exist. Compare to the
  launching shell's env. Are they identical?

**2. Test bash rc file behavior under `-c`:**

- Run `bash -c 'echo $PATH'` and compare to `bash -l -c 'echo $PATH'`.
- Check if `bash --rcfile ~/.bashrc -c 'echo hello'` works.
- Check if `bash -ic 'echo hello'` loads `.bashrc` (interactive flag).
- Read the vendored bash source to understand which startup files are loaded
  under which conditions (`-c`, `-i`, `-l` combinations).

**3. Test nushell rc file behavior under `-c`:**

- Run `nu -c '$env.PATH'` and compare to a normal nushell session.
- Check if `nu -c` loads `env.nu` or `config.nu`.
- Read the vendored nushell source to understand startup file loading under `-c`
  mode.

**4. Evaluate the startup script approach:**

- Could we run `bash -l -c 'export -p'` once at startup to capture a
  fully-configured bash environment? How long does this take?
- Could we run a user-provided script (e.g. `~/.config/shannon/env.sh`) using
  the existing wrapper mechanism and capture the resulting env?
- What would a `.env` file loader look like? Simple `KEY=VALUE` parsing with
  comment support.

#### Verification

1. The environment flow through shannon is fully documented (from launch to
   sub-shell).
2. Bash and nushell `-c` rc file behavior is tested and documented.
3. Each design option (A through D) has concrete pros/cons based on real
   behavior, not assumptions.
4. A recommendation is made for which approach to implement in Experiment 2.

#### Results

**1. Shannon's current environment flow:**

- `ShellState::from_current_env()` calls `std::env::vars()`, which captures
  every env var from the launching process. If shannon is launched from a
  fully-configured terminal, the initial state is complete.
- `execute_command()` calls `env_clear()` then `envs(&state.env)`. Sub-shells
  see ONLY what shannon provides. They never load their own rc files.
- Tested: the current shell has 58 env vars, and shannon inherits all of them.
  PATH has 20 entries, matching the launching shell exactly.

This confirms: **shannon owns the environment**. Sub-shells are completely
dependent on what shannon gives them.

**2. Bash rc file behavior under `-c`:**

Tested and confirmed against vendored bash source (`shell.c:1147`):

| Invocation        | Login files | .bashrc | BASH_ENV |
| ----------------- | ----------- | ------- | -------- |
| `bash -c cmd`     | No          | No      | Yes      |
| `bash -l -c cmd`  | Yes         | No      | Yes      |
| `bash -i -c cmd`  | No          | Yes     | No       |
| `bash -il -c cmd` | Yes         | Yes     | No       |

Key finding: **`BASH_ENV` is sourced by `bash -c`**. This is bash's intended
mechanism for configuring non-interactive shells. Shannon could set `BASH_ENV`
to point to a user config file, and bash would source it before every command.

`bash -l -c 'export -p'` captures a full login environment in ~5ms. This is fast
enough to run once at startup.

**3. Nushell rc file behavior under `-c`:**

Tested and confirmed against vendored nushell source (`src/run.rs:35-72`):

| Invocation                 | env.nu       | config.nu | Plugins |
| -------------------------- | ------------ | --------- | ------- |
| `nu -c cmd`                | Default only | No        | Yes     |
| `nu -n -c cmd`             | No           | No        | No      |
| `nu -l -c cmd`             | Custom       | Custom    | Yes     |
| `nu --env-config F -c cmd` | Custom F     | No        | Yes     |

Key finding: `nu -c` loads only the **built-in default** env.nu, not the user's
custom one. The user's `~/.config/nushell/env.nu` is skipped. To load it, use
`--env-config` or `-l`.

Tested: `nu -c` gives 64 env columns, `nu -n -c` gives 62. The difference is the
built-in defaults (NU_LIB_DIRS, etc.).

**4. Startup script approach evaluation:**

Tested: `BASH_ENV=/path/to/file bash -c 'export -p'` works — the file is sourced
and its exports are captured. This is a viable mechanism for per-command config.

However, for a one-time startup capture, `bash -l -c 'export -p'` takes ~5ms and
gives the full login environment. This could seed `ShellState` at launch.

A `.env` file would be simple KEY=VALUE parsing — trivial to implement in Rust.

**Result:** Pass

#### Conclusion

The environment is shannon's responsibility. Sub-shells see only what shannon
provides via `env_clear()` + `envs()`. The sub-shells' own rc files are
irrelevant — they're never loaded under `-c` mode anyway.

**Recommendation for Experiment 2:**

Use a layered approach:

1. **Inherit the launching shell's environment** (current behavior via
   `from_current_env()`). This handles the common case where shannon is launched
   from a configured terminal.

2. **Support an optional shannon startup script** at `~/.config/shannon/env.sh`
   (or `env.nu`, or any supported shell). If this file exists, run it once at
   startup using the existing wrapper mechanism and merge the resulting env vars
   into `ShellState`. This handles the case where the user wants extra
   configuration specific to shannon, or launches shannon from a bare context.

3. **Do NOT source per-shell rc files on every command.** The per-command
   overhead is unnecessary and risks side effects. Shannon's `env_clear()`
   design means the sub-shells' rc files can't help anyway — they'd need to be
   sourced inside the wrapper, adding latency to every keystroke.

The startup script approach (Option D from the background) is the best fit
because:

- It runs once, not per-command (fast).
- It uses any shell the user prefers (not locked to bash or nushell).
- It's optional — users who launch from a configured terminal need nothing.
- It reuses the existing wrapper/capture mechanism.
- `BASH_ENV` could optionally be set to this file for bash sub-shells, giving
  them access to the config without modifying the wrapper.

### Experiment 2: Implement config.sh startup script

#### Description

Add support for an optional `~/.config/shannon/config.sh` that runs once at
startup via bash. If the file exists, shannon executes it using the existing
bash wrapper mechanism, captures the resulting environment, and merges it into
the initial `ShellState`. If the file doesn't exist, behavior is unchanged.

The script is always bash — shannon requires bash, and the primary use case
is `export PATH="$PATH:/new/path"` which bash handles perfectly. Users who
want to source their `.bashrc` can write `source ~/.bashrc` in `config.sh`
as a deliberate choice.

#### Changes

**`src/main.rs`** — after `ShellState::from_current_env()`, call a new
function to run the startup script:

```rust
let mut state = ShellState::from_current_env();
state = run_startup_script(state);
```

**`src/executor.rs`** — add `pub fn run_startup_script(state: ShellState) -> ShellState`:

1. Build the config path: `dirs::config_dir().join("shannon/config.sh")`.
2. If the file doesn't exist, return `state` unchanged.
3. Build a bash wrapper that sources the file: `source '/path/to/config.sh'`.
   Use the existing `build_bash_wrapper` with the source command as the
   "user command".
4. Run it with `Command::new("bash").args(["-c", &wrapper])`, injecting the
   current `state.env` and `state.cwd` (same pattern as `execute_command`).
5. Parse the captured env with `parse_bash_env`.
6. Return the new `ShellState` with merged env vars, preserving the original
   cwd and exit code 0.
7. If anything fails (bad script, parse error), print a warning to stderr
   and return the original state. Shannon should never fail to start because
   of a broken config.sh.

**`src/executor.rs` tests** — add:

- `test_run_startup_script_with_file` — create a temp dir with a `config.sh`
  that exports `SHANNON_TEST=from_config`. Verify the returned state contains
  the new var.
- `test_run_startup_script_missing_file` — no config.sh exists. Verify state
  is returned unchanged.
- `test_run_startup_script_bad_script` — config.sh contains `exit 1`. Verify
  shannon doesn't crash, returns original state, and prints a warning.
- `test_run_startup_script_preserves_existing_env` — config.sh exports a new
  var. Verify existing vars (like HOME) are still present in the result.
- `test_run_startup_script_path_append` — config.sh does
  `export PATH="$PATH:/custom/bin"`. Verify the returned PATH contains
  `/custom/bin`.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes — all new and existing tests green.
3. Create `~/.config/shannon/config.sh` with
   `export SHANNON_TEST="it works"`. Run `cargo run`, type
   `echo $SHANNON_TEST` — prints "it works".
4. Delete `config.sh`. Run `cargo run` — starts normally, no errors.
5. Create a broken `config.sh` with `exit 1`. Run `cargo run` — prints a
   warning, starts normally.
6. Create `config.sh` with `export PATH="$PATH:/test/path"`. Run `cargo run`,
   type `echo $PATH` — includes `/test/path`.
