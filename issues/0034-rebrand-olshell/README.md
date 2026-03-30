+++
status = "open"
opened = "2026-03-30"
+++

# Issue 34: Rebrand shannon to olshell

## Goal

Rename the project from "shannon" to "olshell". The name immediately conveys
to a nushell audience what this is: nushell with the "old shell" (bash) built
in. The name "shannon" no longer has special meaning after removing AI mode.

## Scope

Rebrand everything EXCEPT the forked dependency crates on crates.io. The
`shannon-nu-*`, `shannon-brush-*`, and `shannon-reedline` packages stay as-is.
Only the main package (`shannonshell`) is republished as `olshell`.

### What changes

**Binary and crate:**
- Binary name: `shannon` → `olshell`
- Crate name on crates.io: `shannonshell` → `olshell`
- `Cargo.toml` package name, `[[bin]]` name

**Config directory:**
- `~/.config/shannon/` → `~/.config/olshell/`
- `nu-path/src/helpers.rs` in nushell fork: `"shannon"` → `"olshell"`
- `main.rs` XDG validation: `.join("shannon")` → `.join("olshell")`

**Env vars:**
- `SHANNON_MODE` → `OLSHELL_MODE`
- `SHANNON_DEPTH` → `OLSHELL_DEPTH` (if still used)

**Keybinding:**
- `__shannon_switch` → `__olshell_switch`

**Code references:**
- `ShannonDispatcher` → `OlshellDispatcher`
- `shannonshell::` imports → `olshell::` imports
- Error messages mentioning "shannon"
- Banner text

**Nushell fork:**
- Config dir name in `nu-path`
- `SHANNON_MODE` references in `repl.rs`
- `__shannon_switch` in keybinding setup

**Documentation:**
- CLAUDE.md — all references
- README.md — all references
- docs/*.md — all references
- Issue docs are immutable — leave as-is

**GitHub:**
- Repository: `shannonshell/shannon` → new org or rename
- Submodule URLs if org changes

**Scripts:**
- `build.sh`, `release.sh` — update any "shannon" references

### What stays the same

- `shannon-nu-*` crates on crates.io (keep names)
- `shannon-brush-*` crates on crates.io (keep names)
- `shannon-reedline` crate on crates.io (keep name)
- Submodule fork repos (keep current names/URLs)
- `Cargo.toml` dependency references with `package = "shannon-*"` (unchanged)
- Closed issues (immutable)

### env.sh / env.nu

- `env.sh` stays as `env.sh` (it's a filename, not branded)
- Users will need to move their config from `~/.config/shannon/` to
  `~/.config/olshell/`

### Migration for existing users

- Old config dir `~/.config/shannon/` is not auto-migrated
- Document: "move your config from `~/.config/shannon/` to `~/.config/olshell/`"
- Old `shannonshell` crate stays on crates.io (users can still install old versions)
