+++
status = "closed"
opened = "2026-03-24"
closed = "2026-03-25"
+++

# Issue 25: Publish all workspace crates to crates.io

## Goal

Publish `shannonshell` and all its forked dependencies (`shannon-reedline`,
`shannon-nu-*`, `shannon-brush-*`) to crates.io. Create a repeatable script that
publishes everything in the correct order.

## Background

Shannon depends on forked versions of three upstream projects:

- **reedline** → `shannon-reedline` (1 crate)
- **nushell** → `shannon-nu-*` + `shannon-nuon` (~32 crates)
- **brush** → `shannon-brush-parser`, `shannon-brush-core`,
  `shannon-brush-builtins` (3 crates)

These are currently used via path dependencies pointing to git submodules.
crates.io doesn't allow path deps — all dependencies must be published versions
on a registry.

The nushell fork crates already have both `path` and `version` fields in their
internal dependency references. When published, crates.io ignores the `path` and
uses the `version`. This should work as-is for the fork crates.

Shannon's own `Cargo.toml` currently uses only `path` deps (no `version` field).
Before publishing `shannonshell`, these need to be switched to version deps
pointing at the published fork crates.

### Publishing order

Crates must be published in topological dependency order — a crate can only be
published after all its dependencies are already on crates.io.

1. **shannon-reedline** (no internal deps)
2. **shannon-nu-\* leaf crates** (nu-glob, nu-path, nu-utils, nu-experimental,
   nu-system, nu-derive-value — no nu-\* deps)
3. **shannon-nu-\* middle crates** (nu-json, nu-protocol, nu-engine, nu-parser,
   etc. — depend on leaf crates)
4. **shannon-nu-\* top crates** (nu-command, nu-cli, nu-cmd-lang, etc.)
5. **shannon-brush-parser**, then **shannon-brush-core**, then
   **shannon-brush-builtins**
6. **shannonshell** (depends on all of the above)

### Tooling options

- **Manual script**: `cargo publish -p <crate>` in order, with waits between for
  crates.io indexing
- **cargo-workspaces**: Automates ordered publishing within a single workspace.
  Our crates span three workspaces, so it would need to be run three times.
- **cargo-release**: Version bumping + publishing. More opinionated.
- **Custom script**: A `scripts/publish.sh` that lists crates in order and
  publishes each one.

### Shannon Cargo.toml for publishing

Before publishing `shannonshell`, switch from path deps to version deps:

```toml
# From:
nu-cli = { path = "../nushell/crates/nu-cli", package = "shannon-nu-cli", ... }

# To:
nu-cli = { version = "0.111.1", package = "shannon-nu-cli", ... }
```

This could be automated in the publish script, or we could maintain both `path`
and `version` fields (cargo uses path for local builds, version for publishing).

### Dual path+version pattern

Cargo supports both `path` and `version` in the same dependency:

```toml
nu-cli = { path = "../nushell/crates/nu-cli", version = "0.111.1", package = "shannon-nu-cli" }
```

Locally, cargo uses the path. When publishing, it uses the version. This avoids
needing to switch Cargo.toml before each publish. The nushell fork crates
already use this pattern for their internal deps.

## Experiments

### Experiment 1: Extend release.sh with fork crate publishing

#### Description

`scripts/release.sh` already handles version bumping, testing, dry-run,
commit, tag, publish, and push for `shannonshell`. Extend it to also publish
all fork crates in dependency order before publishing shannonshell.

The script takes a version argument: `scripts/release.sh 0.2.0`.

Steps:

1. Compute the topological publish order for all ~37 fork crates
2. Add `version` fields to shannon's Cargo.toml (dual path+version pattern)
3. Extend `scripts/release.sh` to publish fork crates before shannonshell
4. Test with `--dry-run` to validate everything resolves

#### Changes

**`shannon/Cargo.toml`:**

- Add `version` fields alongside existing `path` deps (dual pattern):
  `nu-cli = { path = "../nushell/crates/nu-cli", version = "0.111.1", package = "shannon-nu-cli" }`

**`scripts/release.sh`:**

- Before publishing `shannonshell`, publish all fork crates in order:
  1. `cd reedline && cargo publish`
  2. `cd nushell && cargo publish -p <crate>` for each nu crate in topo order
  3. `cd brush && cargo publish -p <crate>` for each brush crate in order
- Sleep between publishes for crates.io indexing
- Update version in shannonshell's Cargo.toml (already does this)
- Fork crate versions stay at their current versions (we don't bump them
  unless we change them — they track upstream versions)
- Support `--dry-run` flag to validate without publishing

#### Verification

1. `scripts/release.sh 0.2.0 --dry-run` passes for all crates.
2. `scripts/release.sh 0.2.0` publishes all crates successfully.
3. `cargo install shannonshell` works from a clean environment.

**Result:** Fail

Multiple blocking issues encountered:

1. **crates.io rate limits:** New crate creation is limited to ~10 per time
   window. With 37 crates to publish, the script hits rate limits repeatedly
   and requires multiple `--resume` runs hours apart.
2. **Dev-dependency cycles:** Cargo requires all dependencies (including
   dev-deps) to exist on the registry during packaging. `cargo-workspaces`
   strips dev-deps to break cycles, but this causes feature-forward errors
   (`nu-test-support/network` referenced in features but dep stripped).
3. **`cargo-workspaces` unreliability:** Reports "published" for rate-limited
   crates that actually failed. Doesn't detect partial failures correctly.
4. **Missing version fields:** Workspace dependencies need both `path` and
   `version` for publishing, discovered incrementally through failures.
5. **Feature forwards to dev-deps:** `nu-command` and `nu-cmd-lang` had
   feature entries referencing `nu-test-support` which breaks when dev-deps
   are stripped. Required manual fixes in the nushell fork.

Approximately 21 of 37 crates were successfully published across multiple
runs. The remaining 11+ crates need more rate-limit cooldown cycles.

#### Conclusion

The initial experiment design (manual topological ordering in `release.sh`)
failed due to dev-dep cycles that cargo can't resolve during packaging. Switched
to `cargo-workspaces` which strips dev-deps automatically. The rate limits
required ~24 hours of incremental `--resume` runs to publish all crates.

Despite being marked as a failure, all 37 crates were eventually published
through persistent re-runs of `cargo workspaces publish`. The experiment
failed as an automated process but succeeded as a manual one.

### Additional fixes discovered during publishing

Changes made to the nushell fork that weren't in the original experiment design:

1. **Excluded plugin crates from workspace** — `nu_plugin_*` binary crates
   broke when renamed (binary target paths changed). Not needed by shannon,
   so removed from workspace members.
2. **Removed feature forwards to dev-deps** — `nu-command/Cargo.toml` had
   `nu-test-support/network` and `nu-test-support/rustls-tls` in features.
   `nu-cmd-lang/Cargo.toml` had `nu-test-support/os`. These break when
   `cargo-workspaces` strips dev-deps. Removed the feature forwards.
3. **Added version to reedline workspace dep** — `reedline` in nushell's
   `[workspace.dependencies]` had only `path`, no `version`. Cargo requires
   `version` for publishing.
4. **Marked root `nu` package as `publish = false`** — Prevents
   `cargo-workspaces` from trying to publish to the real `nu` crate on
   crates.io which we don't own.
5. **Dual path+version pattern in shannon's Cargo.toml** — All path deps now
   include `version` fields so local builds use paths and `cargo publish` uses
   registry versions.

### Final publish workflow

For future releases, `release.sh` handles shannonshell. For the fork crates
(only needed when they change):

```bash
cd reedline && cargo publish --allow-dirty
cd ../nushell && cargo workspaces publish --from-git --allow-dirty --no-verify --yes
cd ../brush && cargo publish -p shannon-brush-parser --allow-dirty && \
  cargo publish -p shannon-brush-core --allow-dirty && \
  cargo publish -p shannon-brush-builtins --allow-dirty
```

Future publishes update existing crates (no rate limit on updates).

## Conclusion

All 37 fork crates and shannonshell v0.2.0 are published on crates.io. The
first publish took ~24 hours due to crates.io rate limits on new crate
creation (~1 new crate per 10-minute window). This is a one-time cost —
subsequent releases update existing crates with no rate limiting.

Key infrastructure:
- `scripts/release.sh <version>` — bumps version, tests, publishes
  shannonshell, commits, tags, pushes. Supports `--dry-run` and `--resume`.
- `cargo workspaces publish` — handles nushell crate ordering and dev-dep
  cycle stripping automatically.
- Dual `path` + `version` deps in shannon's Cargo.toml — local builds use
  submodule paths, publishing uses registry versions.
