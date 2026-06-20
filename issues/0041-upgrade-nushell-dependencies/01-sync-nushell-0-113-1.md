# Experiment 1: Sync nushell 0.113.1 and align dependency graph

## Description

Upgrade Shannon's vendored nushell and reedline dependency set to upstream
nushell 0.113.1, preserving Shannon's fork surface and proving that the shell
still builds, reports the expected embedded nushell version, and preserves nu /
bash mode behavior.

This experiment intentionally covers the whole upgrade rather than splitting the
tree sync, dependency alignment, and API fixes into separate experiments. Those
steps are tightly coupled: the vendored nushell tree, reedline version, root
`Cargo.toml`, lockfiles, and Shannon's copied startup code must all agree before
the build can produce useful feedback.

## Changes

Planned file and tree changes:

- `nushell/` — replace upstream-owned files with nushell 0.113.1 while
  preserving Shannon's fork surface.
- `reedline/` — update to the version required by nushell 0.113.1, preserving
  Shannon's path dependency layout.
- `Cargo.toml` — align root `nu-*` path dependency versions to 0.113.1 for
  upstream crates; keep Shannon package names and versions for `shannon-nu-cli`
  and `shannon-nu-lsp`.
- `Cargo.lock`, `nushell/Cargo.lock`, and `reedline/Cargo.lock` — regenerate or
  accept upstream lockfile changes as needed after the vendored trees are
  aligned.
- `src/main.rs`, `src/run.rs`, and any other copied nushell startup files —
  update for upstream API churn while preserving Shannon startup behavior.
- `issues/0041-upgrade-nushell-dependencies/README.md` — update Experiment 1
  status after the result is known.
- `issues/0041-upgrade-nushell-dependencies/01-sync-nushell-0-113-1.md` — record
  review, result, and conclusion.

Shannon fork surface to preserve or re-apply:

- `nushell/Cargo.toml` — package renames, Shannon crate versions, and reedline
  path dependency.
- `nushell/crates/nu-cli/Cargo.toml` — `name = "shannon-nu-cli"` and tree-sitter
  dependencies for `BashHighlighter`.
- `nushell/crates/nu-cli/src/bash_highlight.rs` — bash syntax highlighter.
- `nushell/crates/nu-cli/src/mode_dispatcher.rs` — `ModeDispatcher` trait.
- `nushell/crates/nu-cli/src/lib.rs` — module declarations and re-exports for
  `BashHighlighter`, `ModeDispatcher`, and `ModeResult`.
- `nushell/crates/nu-cli/src/repl.rs` — Shannon mode switching, highlighter
  selection, and `ModeDispatcher::execute()` hook.
- `nushell/crates/nu-cli/src/nu_highlight.rs` — `NoOpHighlighter`.
- `nushell/crates/nu-command/src/platform/input/input_.rs` — Shannon's reedline
  default-buffer behavior.
- `nushell/crates/nu-lsp/Cargo.toml` — `name = "shannon-nu-lsp"` and dependency
  on `shannon-nu-cli`.

Implementation sequence:

1. Complete workflow preflight:
   - confirm the work is on a dedicated upgrade branch, not `main`
   - confirm the plan docs are the only uncommitted changes before the plan
     commit
   - after design review approval, format docs and commit the issue README plus
     this experiment plan before implementation begins
2. Add upstream remotes if they are missing:
   - `upstream-nushell = https://github.com/nushell/nushell`
   - `upstream-reedline = https://github.com/nushell/reedline`
3. Fetch upstream tags and branches.
4. Verify the existing Shannon fork surface before replacing anything:
   - identify the last nushell alignment/import commit, currently expected to be
     `bb981a484`
   - run a diff such as
     `git diff --stat bb981a484..HEAD -- nushell/ reedline/ Cargo.toml`
   - compare the result with the preservation list above and update the list if
     any additional Shannon-owned files are discovered
5. Snapshot Shannon fork files from the current branch.
6. Preserve upstream history with non-squashed subtree pulls pinned to release
   refs:
   - run `git subtree pull --prefix nushell upstream-nushell 0.113.1` without
     `--squash`, using a non-interactive merge message such as
     `-m "Merge nushell upstream 0.113.1"`
   - inspect `nushell/Cargo.toml` at tag `0.113.1` to identify the required
     reedline version
   - run `git subtree pull --prefix reedline upstream-reedline vX.Y.Z` without
     `--squash`, where `vX.Y.Z` is the reedline tag required by nushell 0.113.1,
     using a non-interactive merge message such as
     `-m "Merge reedline upstream vX.Y.Z"`
7. If the nushell subtree pull leaves an untrustworthy conflicted tree, commit
   the non-squashed subtree merge state as described in
   `skills/merge-upstream/SKILL.md`, then wholesale-replace `nushell/` from the
   exact upstream tag `0.113.1`. Do not use `main` as the archive or checkout
   source for this issue.
8. If reedline conflicts or requires replacement, use the exact reedline tag
   identified from nushell 0.113.1. Do not use reedline `main` unless nushell
   0.113.1 explicitly requires an unreleased commit and the issue documents why.
9. Re-apply Shannon's fork files by porting the Shannon hunks onto upstream
   0.113.1 rather than blindly copying stale files where upstream changed the
   surrounding code.
10. Align all internal nushell dependency versions and root path dependency
    versions.
11. Regenerate lockfiles.
12. Fix compile errors caused by upstream API churn.
13. Run verification and record the result.

## Verification

Required checks:

1. `cargo build`
2. `cargo test`
3. `./target/debug/shannon --version`
   - Expected: Shannon version remains the crate version.
   - Expected: embedded nushell version reports 0.113.1.
4. Non-interactive nu command smoke test:
   - run a simple nu command through `target/debug/shannon`.
   - verify exit status 0 and expected output.
5. Interactive smoke test when a TTY is available:
   - start `./target/debug/shannon`
   - run a nushell command such as `version`
   - press Shift+Tab and verify bash mode is selected
   - run a bash command such as `echo $HOME`
   - press Shift+Tab and verify nu mode is restored
   - verify cwd/env propagation across modes

If interactive verification cannot be performed in the current environment,
record that limitation explicitly and run a concrete PTY-backed automated smoke
test instead. The fallback check must:

- start `./target/debug/shannon` under a pseudo-terminal
- run a nu-mode command and verify expected output
- trigger the same mode-switch path used by Shift+Tab, either by sending the
  Shift+Tab escape sequence if the PTY harness supports it or by sending the
  host command text `__shannon_switch`
- run a bash-mode command and verify expected output
- switch back to nu mode
- verify at least one cwd or env propagation behavior across the mode boundary

Pass criteria:

- vendored nushell reports 0.113.1 in `nushell/Cargo.toml` and
  `nushell/crates/nu-protocol/Cargo.toml`
- root `Cargo.toml` upstream `nu-*` path dependencies are aligned to 0.113.1
- Shannon package renames and fork-specific files are preserved
- required build and test checks pass
- Shannon's version output shows nushell 0.113.1

Fail criteria:

- the upgrade cannot build without removing Shannon's mode dispatch behavior
- upstream dependency versions cannot be aligned without changing Shannon crate
  identity or package names
- build or tests fail in a way that requires a separate design decision

## Design Review

Initial Codex design review: **Changes required**. The review identified five
required fixes:

- add explicit non-squashed subtree pull steps so upstream history is preserved
- resolve the `0.113.1` tag versus `main` source-of-truth ambiguity
- add clean branch/preflight and fork-surface verification steps
- specify a concrete dispatcher smoke test when interactive TTY testing is not
  available
- include the required plan-commit gate before implementation

All five findings were addressed in this experiment plan before requesting a
second review.

Second Codex design review: **Approved**. No required findings remained. The
review suggested adding explicit non-interactive subtree merge messages; that
suggestion was applied before the plan commit.
