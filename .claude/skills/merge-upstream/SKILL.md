---
name: merge-upstream
description: "Merge new upstream releases of nushell and reedline into Shannon"
---

# Merge Upstream

Shannon tracks nushell and reedline as git subtrees under `nushell/` and
`reedline/`. Every few nushell releases, we pull upstream to pick up new
features and fixes. This skill captures what actually works — because
`scripts/sync-upstream.sh` alone is not enough when upstream has drifted.

## Why the script alone doesn't work

`scripts/sync-upstream.sh` runs `git subtree pull` for nushell and reedline,
then `cargo build && cargo test`. That's fine for tiny drifts. For real
upstream releases it fails because:

1. **Subtree auto-merge is noisy.** A hundred or more files will conflict —
   most of them files Shannon never touched. The auto-merger gets confused by
   rename chains and workspace-wide edits.
2. **Auto-merged files end up half-upstream, half-fork.** Even files that
   don't show conflicts can end up with stale content that references
   removed APIs. You won't see this until `cargo build` fails deep in
   `nu-protocol` or `nu-parser`.
3. **New upstream files get missed.** When auto-merge fights with the
   subtree prefix, new files upstream added may not appear in our tree at
   all — the build will complain about missing modules.
4. **Shannon's own `src/` lags upstream API.** `src/main.rs` and `src/run.rs`
   are copied from nushell's binary and drift every release.
5. **Root `Cargo.toml` version pins don't auto-bump.** The Shannon crate
   still references the old `0.N.0` nu-* versions.
6. **Reedline must move in lockstep.** Nushell's workspace pins a specific
   reedline version; if you only pull nushell, the build fails because our
   vendored reedline is stale.

## Shannon's fork surface (the only 10 files we preserve)

Keep these exact files across upgrades. Everything else in `nushell/` should
come from upstream verbatim:

- `nushell/Cargo.toml` — workspace reedline path dep, shannon package
  renames (`shannon-nu-cli`, `shannon-nu-lsp`), shannon crate versions
- `nushell/crates/nu-cli/Cargo.toml` — `name = "shannon-nu-cli"`, tree-sitter
  deps for `BashHighlighter`
- `nushell/crates/nu-cli/src/bash_highlight.rs` — NEW, tree-sitter-based
  bash syntax highlighter
- `nushell/crates/nu-cli/src/mode_dispatcher.rs` — NEW, `ModeDispatcher` trait
- `nushell/crates/nu-cli/src/lib.rs` — declares `mod bash_highlight`,
  `mod mode_dispatcher`, and re-exports `BashHighlighter`,
  `ModeDispatcher`, `ModeResult`
- `nushell/crates/nu-cli/src/repl.rs` — dispatch hook in `loop_iteration()`
  that forwards to `ModeDispatcher::execute()` when `$env.SHANNON_MODE` is
  not `"nu"`; also a few smaller tweaks
- `nushell/crates/nu-cli/src/nu_highlight.rs` — small tweak
- `nushell/crates/nu-command/src/platform/input/input_.rs` — small tweak
- `nushell/crates/nu-lsp/Cargo.toml` — `name = "shannon-nu-lsp"`, references
  `shannon-nu-cli`

Verify this list against the current state before you start:

```sh
git diff --stat <last-nushell-import-commit>..HEAD -- nushell/
```

If new Shannon-modified files appear, add them to the preserve list below.

## The reliable procedure

Work on a branch — never on main.

### 1. Preflight

Clean working tree. Fetch upstream. Count the drift.

```sh
git status                                       # must be clean
git fetch upstream-nushell upstream-reedline
git log --oneline <last-merge-base>..upstream-nushell/main | wc -l
```

Large drifts (100+ commits) are the norm — that's fine, just plan for
conflicts.

### 2. Branch off

```sh
git checkout -b upgrade/nushell-$(date +%Y-%m-%d)
```

### 3. Pull nushell (expect conflicts)

```sh
git subtree pull --prefix nushell upstream-nushell main \
  -m "Merge nushell upstream $(date +%Y-%m-%d)"
```

This will fail with "Automatic merge failed". That is expected.

### 4. Save Shannon's fork files to /tmp

Before doing anything destructive:

```sh
mkdir -p /tmp/shannon_patches
cp nushell/Cargo.toml                                       /tmp/shannon_patches/Cargo.toml
cp nushell/crates/nu-cli/Cargo.toml                         /tmp/shannon_patches/nu-cli-Cargo.toml
cp nushell/crates/nu-cli/src/bash_highlight.rs              /tmp/shannon_patches/bash_highlight.rs
cp nushell/crates/nu-cli/src/mode_dispatcher.rs             /tmp/shannon_patches/mode_dispatcher.rs
cp nushell/crates/nu-cli/src/lib.rs                         /tmp/shannon_patches/lib.rs
cp nushell/crates/nu-cli/src/nu_highlight.rs                /tmp/shannon_patches/nu_highlight.rs
cp nushell/crates/nu-cli/src/repl.rs                        /tmp/shannon_patches/repl.rs
cp nushell/crates/nu-command/src/platform/input/input_.rs   /tmp/shannon_patches/input_.rs
cp nushell/crates/nu-lsp/Cargo.toml                         /tmp/shannon_patches/nu-lsp-Cargo.toml
```

The unresolved conflict markers in those files are fine — they're snapshots,
not for reuse. What you actually need from them is the Shannon side of each
hunk, which you'll recreate by hand in step 7. In practice the easier
workflow is: commit the broken merge first (step 5), then re-export clean
Shannon versions from the main branch:

```sh
git show main:nushell/Cargo.toml > /tmp/shannon_patches/Cargo.toml
# ... and so on
```

### 5. Commit the busted merge so you have a clean slate

Don't try to hand-resolve 100+ conflicts. Just stage whatever's there and
commit it — you're about to overwrite the tree anyway.

```sh
git checkout --theirs -- $(git diff --name-only --diff-filter=U)
git add -A
git -c core.editor=true commit --no-edit
```

### 6. Wholesale-replace the nushell/ tree with upstream

This is the step that makes everything else tractable. Wipe `nushell/` and
re-populate from `upstream-nushell/main`:

```sh
git rm -rqf nushell/
mkdir -p nushell
git archive upstream-nushell/main | tar -x -C nushell/
```

You now have a pristine copy of upstream's tree at `nushell/`, free of any
auto-merge weirdness.

### 7. Re-apply Shannon's fork files

```sh
cp /tmp/shannon_patches/Cargo.toml          nushell/Cargo.toml
cp /tmp/shannon_patches/nu-cli-Cargo.toml   nushell/crates/nu-cli/Cargo.toml
cp /tmp/shannon_patches/bash_highlight.rs   nushell/crates/nu-cli/src/bash_highlight.rs
cp /tmp/shannon_patches/mode_dispatcher.rs  nushell/crates/nu-cli/src/mode_dispatcher.rs
cp /tmp/shannon_patches/lib.rs              nushell/crates/nu-cli/src/lib.rs
cp /tmp/shannon_patches/nu_highlight.rs     nushell/crates/nu-cli/src/nu_highlight.rs
cp /tmp/shannon_patches/repl.rs             nushell/crates/nu-cli/src/repl.rs
cp /tmp/shannon_patches/input_.rs           nushell/crates/nu-command/src/platform/input/input_.rs
cp /tmp/shannon_patches/nu-lsp-Cargo.toml   nushell/crates/nu-lsp/Cargo.toml
```

Then update the Shannon files for upstream API churn:

- **`nushell/Cargo.toml`** — bump all `version = "0.OLD.0"` entries in the
  `[dependencies]` block to match the new upstream version. Keep the
  `shannon-nu-cli` / `shannon-nu-lsp` package renames and Shannon crate
  versions. Bump `reedline` in `[workspace.dependencies]` to the new
  version and keep `path = "../reedline"`. The `[workspace.package]` and
  `[[test]]` blocks may be new from upstream — preserve them.
- **`nushell/crates/nu-cli/Cargo.toml`** — bump all `version = "0.OLD.0"`
  in both `[dev-dependencies]` and `[dependencies]`. Add
  `rust-version.workspace = true` and `autotests = false` if upstream
  introduced them.
- **`nushell/crates/nu-lsp/Cargo.toml`** — same pattern.

### 8. Pull reedline

```sh
git subtree pull --prefix reedline upstream-reedline main \
  -m "Merge reedline upstream $(date +%Y-%m-%d)"
# Resolve Cargo.lock conflict by taking upstream:
git checkout --theirs -- reedline/Cargo.lock
git add reedline/Cargo.lock
git -c core.editor=true commit --no-edit
```

Reedline has no Shannon-side changes, so conflicts are minimal (usually just
`Cargo.lock`).

### 9. Regenerate Cargo.lock files

```sh
rm nushell/Cargo.lock
(cd nushell && cargo generate-lockfile)
```

The root `Cargo.lock` regenerates on the next `cargo build`.

### 10. Bump root `Cargo.toml`

In `/Users/ryan/dev/shannon/Cargo.toml`, update:

- Every `nu-* = { version = "0.OLD.0", ... }` to the new version
- `reedline = { version = "0.OLD.0", ... }` to the new version

A sed one-liner works if the old version is unique:

```sh
sed -i '' 's/version = "0.111.0"/version = "0.112.2"/g' Cargo.toml
sed -i '' 's/version = "0.46.0"/version = "0.47.0"/g' Cargo.toml
```

(macOS `sed` uses `-i ''`. Linux: `sed -i`.)

### 11. Update Shannon's `src/main.rs` and `src/run.rs` for API churn

Shannon's `src/` is copied from nushell's binary and drifts every release.
Diff against upstream to find what changed:

```sh
diff src/main.rs nushell/src/main.rs
diff src/run.rs  nushell/src/run.rs
```

Common changes:

- **`std::time::Instant` → `nu_utils::time::Instant`.** Nushell migrated to
  its own `Instant` wrapper. Replace everywhere in Shannon's `src/`.
- **`nu_protocol::location!()` removed.** Calls to
  `IoError::new_internal_with_path(err, msg, location!(), path)` now take
  only `(err, msg, path)` — drop the `location!()` argument.
- **`ShellError::GenericError` → `ShellError::Generic`.** (Currently emits
  deprecation warnings; not a build failure yet.)
- **`evaluate_repl` signature changes.** Check the argument list against
  upstream if you get a type mismatch.

The diff against upstream's equivalent file is the fastest way to find all
call sites that need updating.

### 12. Build

```sh
cargo build
```

First build errors will usually be in `nu-parser` or `nu-protocol` complaining
about missing exports. If you see this after a wholesale tree replace, it's
almost always **stale incremental compilation artifacts** from an earlier
failed build. Force a rebuild of the affected crate:

```sh
touch nushell/crates/nu-experimental/src/lib.rs  # or whichever crate is stuck
cargo build
```

Avoid `cargo clean` — per `nushell/CLAUDE.md`, it just wastes compile time.

Once `nushell/` compiles, the next errors will be in `shannonshell` itself
(`src/main.rs`, `src/run.rs`) — those are the API-churn fixes from step 11.

### 13. Smoke test

```sh
./target/debug/shannon --version
./target/debug/shannon
```

In the interactive shell:

1. Type a nushell command (e.g. `ls`) — verify nu mode works
2. Press `Shift+Tab` — verify mode switches to bash
3. Type a bash command (e.g. `echo $HOME`) — verify bash mode works
4. Press `Shift+Tab` — verify it switches back to nu
5. Verify env vars propagate across the switch (e.g. `cd /tmp` in bash,
   then back to nu and check `pwd`)

The build passing is **not** sufficient — Shannon's `ModeDispatcher` hook
lives in `repl.rs`, which upstream rewrites frequently. A merge can
compile fine but silently break the dispatcher.

### 14. Commit the work

At this point you should have on the branch:

1. The busted-merge commit (nushell subtree pull)
2. The reedline merge commit
3. One or two commits for the wholesale tree replace + Shannon fork
   re-application + root `Cargo.toml` / `src/` API updates

Merge to main when ready:

```sh
git checkout main
git merge --no-ff upgrade/nushell-$(date +%Y-%m-%d)
```

## Things that will bite you

- **Don't use `--squash` with `git subtree`.** Shannon's `CLAUDE.md`
  explicitly forbids it. Full history across merged projects must be
  preserved for blame/log/bisect.
- **Don't hand-resolve 100+ conflicts.** Wholesale replace is faster and
  correct. Conflict-by-conflict resolution leaves stale auto-merged
  content in files you don't notice until build time.
- **Don't forget reedline.** Pull both or the build will fail on version
  pinning.
- **Don't forget the root `Cargo.toml`.** Bumping only
  `nushell/Cargo.toml` is not enough.
- **Don't skip the interactive smoke test.** `cargo build` does not
  exercise the dispatcher hook.
- **The `scripts/sync-upstream.sh` script is not the source of truth.**
  This skill is. Update the script if you want, but don't rely on it
  alone for real upgrades.

## After the upgrade

Consider opening an issue under `issues/` to track any cleanup work —
deprecation warnings to address, features upstream added that Shannon
could expose (e.g. `ExternalHinter` in `repl.rs`), or new commands that
should be wired up.
