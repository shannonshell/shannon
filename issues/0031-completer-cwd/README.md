+++
status = "closed"
opened = "2026-03-25"
closed = "2026-03-25"
+++

# Issue 31: Completer uses stale cwd for file completions

## Goal

File completions should use the current working directory, not the directory
shannon was started in. After `cd /tmp`, completing `nvim READ` should look in
`/tmp`, not the original cwd.

## Background

`ShannonCompleter::new()` captures `std::env::current_dir()` once at creation.
The completer's `self.cwd` never updates when the user changes directories via
shell commands. File completions always search relative to the initial cwd.

### Root cause

`ShannonCompleter` stores `cwd: PathBuf` set once in `new()`. Shannon doesn't
update the process's actual working directory (it tracks cwd in `ShellState`
instead), so `std::env::current_dir()` is always the startup directory.

### Fix options

1. **Share cwd via `Arc<Mutex<PathBuf>>`** — the completer reads from a shared
   reference that the REPL updates after each command. Simple and correct.
2. **Update process cwd** — call `std::env::set_current_dir()` after each
   command. Side effects on the process, but makes `std::env::current_dir()`
   accurate everywhere.
3. **Rebuild completer each iteration** — expensive, but reedline doesn't expose
   a way to update a completer after creation.

Option 1 is cleanest. Option 2 is simplest but has broader implications.

### Research

Bash calls `chdir()` syscall when the user runs `cd` — the process cwd is always
the real cwd. Nushell does NOT call chdir — it only updates its internal Stack.
Shannon should follow bash's approach: update the process cwd after every
command so everything (completions, relative paths, subprocesses) works.

## Experiments

### Experiment 1: Update process cwd after each command

#### Description

Add `std::env::set_current_dir(&state.cwd)` in the REPL loop after each
`run_command` call. This makes the process cwd always match `ShellState.cwd`.
The completer's `std::env::current_dir()` in `new()` is still stale, but we also
fix it to call `current_dir()` on each completion instead of caching.

#### Changes

**`shannon/src/repl.rs`:**

- After each `run_command` call, add
  `let _ = std::env::set_current_dir(&state.cwd);`

**`shannon/src/completer.rs`:**

- Change `complete_file` to use `std::env::current_dir()` instead of `self.cwd`,
  since the process cwd is now always correct
- Remove the `cwd` field from `ShannonCompleter` (no longer needed)

#### Verification

1. `cargo test` passes.
2. Start shannon, `cd /tmp`, tab-complete a file in `/tmp` — works.
3. `cd ~`, tab-complete a file in home — works.
4. Open a fresh shannon pane — completions work from the start.

**Result:** Pass

All verification steps confirmed. 63 tests pass.

#### Conclusion

Process cwd now synced after every command. Completer reads live cwd.

## Conclusion

File completions now use the correct working directory. The process cwd is
synced via `set_current_dir` after each command, matching how bash works.
