+++
status = "open"
opened = "2026-03-25"
+++

# Issue 28: Add /version meta-command

## Goal

Add a `/version` meta-command that prints shannon's version. Use
`env!("CARGO_PKG_VERSION")` to get the version at compile time.

## Background

Shannon has no way to display its version from inside the shell. A `/version`
command fits the existing meta-command pattern (`/switch`, `/ai`, `/help`).

### Changes needed

**`shannon/src/repl.rs`:**

1. Add `/version` case to `handle_meta_command`:
   ```rust
   "/version" => {
       eprintln!("shannon {}", env!("CARGO_PKG_VERSION"));
       true
   }
   ```
2. Add `/version` to `/help` output.
