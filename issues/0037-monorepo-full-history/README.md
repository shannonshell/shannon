+++
status = "open"
opened = "2026-03-30"
+++

# Issue 37: Monorepo with full history preservation

## Goal

Merge nushell, brush, and reedline into the shannon repo as a monorepo,
preserving the FULL git history of ALL projects. Enable one-liner install via
`cargo install --git`. Eliminate submodules and crates.io publishing.

## Background

Issue 36 attempted this but failed by using `cp -r` which destroyed all git
history from the merged projects. This issue uses
`git merge
--allow-unrelated-histories` to properly merge each project's full
commit history into the shannon repo.

### Why full history matters

- `git log nushell/crates/nu-cli/src/repl.rs` shows every upstream change
- `git blame` works on merged files — trace any line to its original author
- `git subtree pull --squash` works for future upstream syncs (proper merge
  base)
- Debugging: `git bisect` can trace regressions across the full history

### Requirements

1. Full git history of nushell, brush, and reedline preserved
2. All files moved into subdirectories (`nushell/`, `brush/`, `reedline/`)
3. Shannon crate moved to repo root
4. `cargo build` works from repo root
5. `cargo install --git https://github.com/shannonshell/shannon` works
6. Future upstream sync via `git subtree pull` works
7. Crate names reverted to originals (no `shannon-*` prefixes)

### Approach: git subtree add

`git subtree add` is the correct tool. It:

1. Fetches the remote repo's full history
2. Rewrites paths so all files land under a prefix directory
3. Creates a merge commit connecting the histories
4. Preserves full blame, log, and bisect across all commits

This is different from `--squash` which collapses history into one commit.

### Steps

**Phase 1: Prepare**

- Commit all pending changes
- Create a backup branch: `git branch backup-pre-monorepo`
- Verify all submodule forks are pushed to GitHub

**Phase 2: Remove submodules**

```sh
git submodule deinit -f nushell brush reedline
git rm nushell brush reedline
rm -rf .git/modules/nushell .git/modules/brush .git/modules/reedline
rm .gitmodules
git commit -m "Remove submodules for monorepo migration"
```

**Phase 3: Move shannon crate to repo root**

```sh
mv shannon/src src
mv shannon/Cargo.toml Cargo.toml
mv shannon/Cargo.lock Cargo.lock
mv shannon/tests tests
mv shannon/README.md README.md
mv shannon/build.rs build.rs
rm -rf shannon/target shannon/LICENSE shannon/tree-sitter-nu
rmdir shannon
# Update path deps: ../nushell/ → nushell/, etc.
git add -A
git commit -m "Move shannon crate to repo root"
```

**Phase 4: Merge with full history**

Add remotes for our forks and use `git subtree add` WITHOUT `--squash`:

```sh
# Nushell — full history
git subtree add --prefix nushell \
  git@github.com:shannonshell/shannon_nushell.git shannon

# Brush — full history
git subtree add --prefix brush \
  git@github.com:shannonshell/shannon_brush.git shannon

# Reedline — full history
git subtree add --prefix reedline \
  git@github.com:shannonshell/shannon_reedline.git shannon
```

Each `subtree add` creates a merge commit that grafts the project's full history
under the specified prefix. `git log nushell/` will show every nushell commit.
`git blame nushell/crates/nu-cli/src/repl.rs` works.

**Note:** This will significantly increase the repo's git history size. Nushell
alone has ~11K commits. This is acceptable — it's how monorepos work (Chromium,
Android, etc. have millions of commits).

**Phase 5: Revert crate name renames**

Revert all `shannon-nu-*` → `nu-*`, `shannon-brush-*` → `brush-*`,
`shannon-reedline` → `reedline` in all Cargo.toml files. Same script as
issue 36.

Update root Cargo.toml: remove `version` and `package` fields from path deps.

**Phase 6: Verify**

```sh
cargo build
cargo test
./target/debug/shannon  # smoke test
git log --oneline nushell/ | head -5  # verify history preserved
git log --oneline brush/ | head -5
git log --oneline reedline/ | head -5
```

**Phase 7: Add upstream remotes for future sync**

```sh
git remote add upstream-nushell https://github.com/nushell/nushell
git remote add upstream-brush https://github.com/reubeno/brush
git remote add upstream-reedline https://github.com/nushell/reedline
```

Future sync (with squash to avoid importing every upstream commit):

```sh
git subtree pull --prefix nushell upstream-nushell main --squash
```

Because we did `subtree add` with full history, git has a proper merge base.
Future `subtree pull` (even with `--squash`) works correctly.

**Phase 8: Update docs, scripts, release process**

- CLAUDE.md: monorepo structure, no submodules
- README.md: `cargo install --git` installation
- scripts/release.sh: simplify to tag + push
- scripts/sync-upstream.sh: subtree pull script
- scripts/build.sh: build from repo root

### Risk mitigation

- **Backup branch** created before starting
- **Fork repos on GitHub** remain as-is (can always re-clone)
- **Each phase committed separately** so we can roll back to any point
- **Build verified after each phase**

### Expected repo size

- Current: ~50MB (shannon only)
- After merge: ~500MB-1GB (nushell's 11K commits + brush + reedline)
- Clone time: ~30-60 seconds (acceptable for one-time install)
