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

### Questions to answer

1. What happens when we source a typical `.bashrc` under `bash -c`? Does it
   break? What about `env.nu`?
2. Can we source rc files without triggering interactive-only features (prompts,
   completions, welcome messages)?
3. Is there a way to get just the environment (PATH, env vars) without the
   interactive side effects?
4. Should shannon support both approaches — use user rc by default, with an
   option to use shannon-specific rc files instead?
5. What's the minimal change to the wrapper scripts in `executor.rs` to source
   an rc file before the user's command?
