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
