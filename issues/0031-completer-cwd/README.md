+++
status = "open"
opened = "2026-03-25"
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
