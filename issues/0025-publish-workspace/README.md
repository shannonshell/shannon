+++
status = "open"
opened = "2026-03-24"
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
