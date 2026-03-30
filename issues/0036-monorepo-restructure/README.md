+++
status = "closed"
opened = "2026-03-30"
closed = "2026-03-30"
+++

# Issue 36: Monorepo restructure — eliminate submodules and crates.io publishing

## Goal

Merge nushell, brush, and reedline source directly into the shannon repo.
Eliminate submodules and crates.io publishing entirely. Enable one-liner install
via `cargo install --git https://github.com/shannonshell/shannon`.

## Background

### Current pain

Shannon depends on 40+ forked nushell crates, 3 brush crates, and 1 reedline
crate — all published to crates.io under `shannon-*` names. Publishing hits rate
limits (429 Too Many Requests), requires version bumping all 40 crates even when
only 2-3 changed, and is the most fragile part of the release process.

### Why we forked

1. **Nushell** — ModeDispatcher trait in nu-cli, BashHighlighter, Shift+Tab
   keybinding, config dir change, relaxed libc pin. Only nu-cli and nu-path have
   meaningful code changes.
2. **Brush** — crate renames only (no code changes). libc conflict with nushell
   forced the fork.
3. **Reedline** — crate rename only (no code changes).

### The monorepo approach

Merge all three projects directly into the shannon repo. No submodules, no
crates.io publishing. Path deps resolve everything at build time.

Install command becomes:

```sh
cargo install --git https://github.com/shannonshell/shannon
```

## Architecture

### Repo structure after restructure

```
shannon/                     (repo root = shannonshell crate)
├── Cargo.toml               ([[bin]] = shannon, path deps to subdirs)
├── src/                     (shannon source — main.rs, dispatcher.rs, etc.)
├── nushell/                 (merged nushell source, NOT a submodule)
│   ├── Cargo.toml           (nushell workspace — NOT part of shannon workspace)
│   └── crates/
│       ├── nu-cli/          (our ModeDispatcher + BashHighlighter changes)
│       ├── nu-path/         (config dir = "shannon")
│       └── ... (all other nu crates, unchanged)
├── brush/                   (merged brush source)
│   ├── brush-core/
│   ├── brush-builtins/
│   └── brush-parser/
├── reedline/                (merged reedline source)
├── docs/
├── website/
├── issues/
├── scripts/
├── vendor/                  (reference repos, gitignored)
└── LICENSE, NOTICE, CLAUDE.md, etc.
```

### Key change: crate names stay original

Since we're not publishing to crates.io, we don't need `shannon-*` prefixed
crate names. Revert all renames:

- `shannon-nu-cli` → `nu-cli`
- `shannon-brush-core` → `brush-core`
- `shannon-reedline` → `reedline`

Shannon's Cargo.toml uses path deps with original names:

```toml
nu-cli = { path = "nushell/crates/nu-cli" }
brush-core = { path = "brush/brush-core" }
reedline = { path = "reedline", features = ["sqlite", "bashisms"] }
```

No `package = "shannon-*"` needed. No version field needed (path deps don't need
versions for non-published crates).

### Workspace considerations

Shannon's Cargo.toml at the repo root is NOT a workspace. Nushell has its own
workspace at `nushell/Cargo.toml`. Brush has its own at `brush/Cargo.toml`.
These are independent — shannon just references specific crates via path.

Cargo handles this: path deps can point into another workspace's crates without
being part of that workspace.

## Upstream sync strategy

### Remotes

Add upstream remotes for each project:

```sh
git remote add upstream-nushell https://github.com/nushell/nushell
git remote add upstream-brush https://github.com/reubeno/brush
git remote add upstream-reedline https://github.com/nushell/reedline
```

### How to sync nushell upstream

Use `git subtree pull` to merge upstream changes into the nushell/ directory:

```sh
git subtree pull --prefix nushell upstream-nushell main --squash \
  -m "Merge nushell upstream main"
```

This fetches nushell's latest main, squashes it into one commit, and merges it
into the `nushell/` directory. Conflicts with our changes (ModeDispatcher in
nu-cli, config dir in nu-path) are resolved during the merge.

Our changes are small (~200 lines across 2-3 files), so conflicts should be rare
and easy to resolve.

### How to sync brush upstream

```sh
git subtree pull --prefix brush upstream-brush main --squash \
  -m "Merge brush upstream main"
```

We have zero code changes in brush (only crate renames, which we're reverting).
So this should be conflict-free.

### How to sync reedline upstream

```sh
git subtree pull --prefix reedline upstream-reedline main --squash \
  -m "Merge reedline upstream main"
```

Zero code changes. Conflict-free.

### Sync frequency

- **Nushell** — sync before each Shannon release, or when a nushell release
  fixes bugs we care about
- **Brush** — sync occasionally, when brush adds features we need
- **Reedline** — sync when nushell requires a newer reedline (they're coupled)

### Sync script

Create `scripts/sync-upstream.sh`:

```sh
#!/usr/bin/env bash
set -euo pipefail

echo "==> Syncing nushell upstream..."
git subtree pull --prefix nushell upstream-nushell main --squash \
  -m "Merge nushell upstream $(date +%Y-%m-%d)"

echo "==> Syncing brush upstream..."
git subtree pull --prefix brush upstream-brush main --squash \
  -m "Merge brush upstream $(date +%Y-%m-%d)"

echo "==> Syncing reedline upstream..."
git subtree pull --prefix reedline upstream-reedline main --squash \
  -m "Merge reedline upstream $(date +%Y-%m-%d)"

echo "==> Building..."
cargo build

echo "==> Testing..."
cargo test

echo "==> Upstream sync complete."
```

## Migration steps

### Phase 1: Move shannon crate to repo root

Currently `shannon/src/main.rs` is the binary. Move it to the repo root:

- Move `shannon/src/` → `src/`
- Move `shannon/Cargo.toml` → `Cargo.toml`
- Move `shannon/Cargo.lock` → `Cargo.lock`
- Move `shannon/tests/` → `tests/`
- Delete `shannon/` directory

Update all path deps from `../nushell/` to `nushell/`, `../brush/` to `brush/`,
`../reedline/` to `reedline/`.

### Phase 2: Remove submodules

```sh
git submodule deinit -f nushell brush reedline
git rm nushell brush reedline
rm -rf .git/modules/nushell .git/modules/brush .git/modules/reedline
rm .gitmodules
```

### Phase 3: Merge repos with subtree add

```sh
git subtree add --prefix nushell \
  git@github.com:shannonshell/shannon_nushell.git shannon --squash

git subtree add --prefix brush \
  git@github.com:shannonshell/shannon_brush.git shannon --squash

git subtree add --prefix reedline \
  git@github.com:shannonshell/shannon_reedline.git shannon --squash
```

### Phase 4: Revert crate name renames

In nushell:

- Revert all `shannon-nu-*` → `nu-*` package names
- Remove all `package = "shannon-nu-*"` from dependency lines

In brush:

- Revert `shannon-brush-*` → `brush-*`

In reedline:

- Revert `shannon-reedline` → `reedline`

In shannon's Cargo.toml:

- Remove all `package = "shannon-*"` annotations
- Remove `version` from path deps (not needed for non-published crates)

### Phase 5: Fix libc conflict

With original crate names and path deps, the libc conflict returns (nushell pins
=0.2.178, brush needs >=0.2.181 via nix 0.31). Fix by EITHER:

- Relaxing nushell's libc pin (already done in our fork): `"=0.2.178"` → `"0.2"`
- OR downgrading brush's nix: `0.31.2` → `0.30` (compatible with libc 0.2.178)

Relaxing the pin is simpler and already proven to work.

### Phase 6: Update Cargo.toml

Shannon's root Cargo.toml becomes simpler:

```toml
[package]
name = "shannonshell"
version = "0.4.0"
edition = "2024"

[[bin]]
name = "shannon"
path = "src/main.rs"

[dependencies]
nu-cli = { path = "nushell/crates/nu-cli" }
nu-engine = { path = "nushell/crates/nu-engine", features = ["os"] }
nu-protocol = { path = "nushell/crates/nu-protocol", features = ["os"] }
# ... etc, no version or package fields
brush-core = { path = "brush/brush-core" }
brush-builtins = { path = "brush/brush-builtins" }
reedline = { path = "reedline", features = ["sqlite", "bashisms"] }
```

### Phase 7: Verify

1. `cargo build` from repo root succeeds
2. `cargo test` passes
3. `./target/debug/shannon` works (both modes, env sync, highlighting)
4. `cargo install --path .` works
5. From a fresh clone:
   `cargo install --git https://github.com/shannonshell/shannon` works

### Phase 8: Update release script

New release process:

- Tag the repo: `git tag v0.4.0`
- Push: `git push --tags`
- Optional: `cargo publish` for shannonshell to crates.io (publishes ONLY the
  shannon crate, using path deps that Cargo replaces with... wait, this won't
  work without published deps)

Actually: if we don't publish deps to crates.io, we can't publish shannonshell
either. The install path is purely `cargo install --git`. Crates.io is only for
discoverability (people can find it via search, but install via git).

OR: keep publishing shannonshell + the 2-3 crates we actually changed (nu-cli,
nu-path) and use official nushell crates for everything else. This is the
"downgrade nix" approach — fix the libc conflict so official crates work.

### Phase 9: Update docs

- Installation instructions: `cargo install --git`
- Remove all references to `shannon-*` crate names
- Update CLAUDE.md architecture section
- Update build/release scripts

## Verification

1. `cargo install --git https://github.com/shannonshell/shannon` works
2. Shannon starts, both modes work
3. `scripts/sync-upstream.sh` pulls latest nushell without breaking
4. `cargo build` takes reasonable time (~5 min first build)
5. Repo clone is reasonable size (<500MB with squashed merges)

## Result: FAIL

The implementation used `cp -r` to copy submodule content into the repo,
which destroyed ALL git history from nushell, brush, and reedline. The
`.git` directories were removed from the copies, so no commit history was
preserved. `git log nushell/` only showed shannon's own commits, not the
~11,000 nushell commits.

Additionally, `git subtree pull` would not work for future upstream syncs
because there was no proper merge base — git had no record of the
relationship between the copied files and the upstream repos.

The repo had to be manually restored from a backup of the pre-monorepo
state.

## Conclusion

`cp -r` is the wrong tool for merging repos. It treats files as new
additions with no history. The correct approach is `git subtree add`
WITHOUT `--squash`, which preserves the full commit history and creates
a proper merge base for future upstream syncs. See issue 37.
