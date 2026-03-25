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

## Experiments

### Experiment 1: Add /version meta-command

#### Description

Add `/version` to `handle_meta_command` and update `/help`.

#### Changes

**`shannon/src/repl.rs`** — add `/version` case and update `/help`.

#### Verification

1. `cargo test` passes.
2. `/version` prints `shannon 0.2.1` (or current version).
3. `/help` lists `/version`.
