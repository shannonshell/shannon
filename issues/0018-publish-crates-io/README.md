+++
status = "open"
opened = "2026-03-23"
+++

# Issue 18: Publish to crates.io

## Goal

Publish shannon to crates.io as `shannonshell` so users can install it with
`cargo install shannonshell`. Set up a versioning process with git tags and
a release script.

## Background

The crate name `shannon` is taken on crates.io. We'll use `shannonshell` as
the crate name, but the binary is still called `shannon`.

### What needs to happen

1. **Update Cargo.toml** for crates.io publishing:
   - Rename package to `shannonshell`
   - Add `[[bin]]` section to name the binary `shannon`
   - Add required metadata: description, license, repository, keywords,
     categories, readme, edition
   - Version starts at `0.1.0`

2. **Path dependencies** — our nushell crates use crates.io versions (already
   done). The `tree-sitter-nu` git dependency may need attention — crates.io
   doesn't allow git dependencies. Check if there's a crates.io version.

3. **Build dependencies** — `build.rs` reads from `completions/` and `themes/`
   directories. These need to be included in the published crate via
   `include` in Cargo.toml.

4. **Release script** — `scripts/release.sh` that:
   - Takes a version number as argument
   - Updates version in Cargo.toml
   - Runs `cargo test` to verify
   - Commits the version bump
   - Tags the commit as `v{version}`
   - Runs `cargo publish`
   - Pushes the commit and tag

5. **Verify** — `cargo publish --dry-run` to check everything works before
   the real publish.

### Potential blockers

- `tree-sitter-nu` is a git dependency — crates.io may reject this. Need to
  check if it's on crates.io or find an alternative.
- `completions/` and `themes/` directories must be included in the crate
  package for `build.rs` to work.
- The `rig-core` dependency pulls in many transitive deps — verify they all
  resolve cleanly.

### tree-sitter-nu blocker

`tree-sitter-nu` is NOT on crates.io (checked — zero results). It's only
available as a git dependency from `github.com/nushell/tree-sitter-nu`.
crates.io rejects git dependencies.

Options:
1. **Vendor tree-sitter-nu** into our repo as a path dependency, then include
   the source in the published crate.
2. **Drop nushell tree-sitter highlighting** — since nushell is embedded, we
   could potentially use nushell's own syntax coloring. But this is a bigger
   change.
3. **Publish tree-sitter-nu ourselves** to crates.io (if license allows).
4. **Make nushell highlighting optional** via a feature flag — crates.io
   publish works without it.
