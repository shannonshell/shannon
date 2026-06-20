+++
status = "closed"
opened = "2026-06-19"
closed = "2026-06-19"
+++

# Issue 41: Upgrade nushell dependencies to 0.113.1

## Goal

Upgrade Shannon's embedded nushell dependency set from the current 0.112.x state
to the latest upstream nushell release, 0.113.1, while preserving Shannon's
fork-specific behavior and keeping the root dependency graph aligned.

## Background

Shannon vendors nushell and reedline directly in the repo and depends on their
crates through path dependencies. The embedded nushell tree currently reports
0.112.2:

- `nushell/Cargo.toml` has `version = "0.112.2"`.
- `nushell/crates/nu-protocol/Cargo.toml` has `version = "0.112.2"`.
- `build.rs` reads `nushell/crates/nu-protocol/Cargo.toml` to set
  `NUSHELL_VERSION` for Shannon's version output.

The root `Cargo.toml` is not fully aligned with that vendored tree. Many `nu-*`
path dependencies still declare `version = "0.112.1"`, while Shannon-renamed
crates use Shannon's package version:

- `nu-cli = { version = "0.5.5", package = "shannon-nu-cli", ... }`
- `nu-lsp = { version = "0.5.5", package = "shannon-nu-lsp", ... }`
- most other `nu-*` dependencies are pinned at `0.112.1`
- `reedline` is pinned at `0.47.0`

Upstream nushell's latest GitHub release is 0.113.1, published on 2026-05-30:

- https://github.com/nushell/nushell/releases/tag/0.113.1
- https://www.nushell.sh/blog/2026-05-30-nushell_v0_113_1.html

## Analysis

This is not just a version string bump. Shannon's binary copies and modifies
parts of nushell's startup and REPL path, and the vendored nushell fork includes
Shannon-specific changes in the `nu-cli` and `nu-lsp` package surface. A correct
upgrade must preserve the fork surface while replacing the rest of the embedded
nushell tree with upstream code.

The upgrade should use the existing merge guidance in
`skills/merge-upstream/SKILL.md`, including:

- preserve full subtree history; never use `git subtree --squash`
- expect nushell subtree conflicts and avoid hand-resolving a large conflicted
  tree when a wholesale upstream replacement is safer
- keep Shannon's `ModeDispatcher`, `BashHighlighter`, Shift+Tab, and package
  rename changes
- pull reedline in lockstep if upstream nushell requires a newer reedline
- regenerate lockfiles after the vendored trees and root dependency versions are
  aligned
- update Shannon's copied `src/main.rs`, `src/run.rs`, and related startup code
  for any upstream API churn

## Requirements

The completed upgrade should leave these versions aligned:

- vendored nushell package versions: 0.113.1
- root `Cargo.toml` `nu-*` dependency versions for upstream crates: 0.113.1
- Shannon-renamed crates: keep Shannon package naming and versioning, while
  updating their internal nushell dependency pins to 0.113.1 as needed
- reedline: whatever version nushell 0.113.1 requires, with Shannon's path
  dependency preserved

The verification bar should include:

- `cargo build`
- `cargo test`
- `./target/debug/shannon --version` shows the expected Shannon version and
  nushell 0.113.1
- interactive smoke test for nu mode, bash mode, Shift+Tab mode switching, and
  env/cwd propagation across modes

## Experiments

- [Experiment 1: Sync nushell 0.113.1 and align dependency graph](01-sync-nushell-0-113-1.md)
  — **Pass**

## Conclusion

Shannon's embedded Nushell dependency set is upgraded to upstream Nushell
0.113.1 with Reedline v0.48.0. The root dependency graph, vendored Nushell
workspace dependencies, lockfiles, and Shannon's version reporting are aligned.

The upgrade preserved Shannon's fork-specific shell behavior: `shannon-nu-cli`
and `shannon-nu-lsp` remain Shannon-renamed crates, the `ModeDispatcher` hook
continues dispatching non-nu modes, Bash highlighting remains wired in, the
mode-switch host command remains available for Shift+Tab, and bash-to-nu env/cwd
propagation passed PTY verification.

Verification passed with `cargo build`, `cargo test`, version output,
non-interactive nu smoke tests, and a PTY-backed nu/bash mode smoke test. The
only observed warning is an upstream `nu-command` unfulfilled lint expectation.
