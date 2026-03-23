+++
status = "open"
opened = "2026-03-23"
+++

# Issue 18: Publish to crates.io

## Goal

Publish shannon to crates.io as `shannonshell` so users can install it with
`cargo install shannonshell`. Set up a Cargo workspace, a versioning process
with git tags, and a release script.

## Background

The crate name `shannon` is taken on crates.io. We'll use `shannonshell` as
the crate name, but the binary is still called `shannon`.

`tree-sitter-nu` is not on crates.io (the nushell team never published it).
We'll republish it as `tree-sitter-shannon-nu` to avoid name-squatting.

### Cargo workspace

Shannon becomes a workspace with two crates:

```
Cargo.toml                            ← workspace root
├── crates/tree-sitter-shannon-nu/    ← republished grammar
│   ├── Cargo.toml                    ← name = "tree-sitter-shannon-nu"
│   ├── bindings/rust/
│   ├── src/                          ← generated C parser
│   └── ...
├── src/                              ← shannonshell binary
├── completions/
├── themes/
└── ...
```

The root `Cargo.toml` is both the workspace definition and the `shannonshell`
package.

### What needs to happen

**1. Create `crates/tree-sitter-shannon-nu/`:**

- Copy source from `vendor/tree-sitter-nu/` into `crates/tree-sitter-shannon-nu/`
- Rename package to `tree-sitter-shannon-nu` in its Cargo.toml
- Set version to `0.1.0`
- Keep the MIT license and credit the nushell authors
- Verify with `cargo publish --dry-run -p tree-sitter-shannon-nu`

**2. Set up workspace in root `Cargo.toml`:**

```toml
[workspace]
members = [".", "crates/tree-sitter-shannon-nu"]
```

**3. Update root `Cargo.toml` for `shannonshell`:**

- Rename package to `shannonshell`
- Add `[[bin]]` section: `name = "shannon"`, `path = "src/main.rs"`
- Version: `0.1.0`
- Add metadata: description, license, repository, keywords, categories, readme
- Change `tree-sitter-nu` git dependency to
  `tree-sitter-shannon-nu = { path = "crates/tree-sitter-shannon-nu", version = "0.1" }`
- Add `include` to ensure `completions/`, `themes/`, `build.rs` are packaged

**4. Update `src/highlighter.rs`:**

- Change `use tree_sitter_nu::LANGUAGE` to `use tree_sitter_shannon_nu::LANGUAGE`

**5. Create `scripts/release.sh`:**

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

# Update versions
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
sed -i '' "s/^version = .*/version = \"$VERSION\"/" crates/tree-sitter-shannon-nu/Cargo.toml

# Test
cargo test

# Commit and tag
git add -A
git commit -m "Release v$VERSION"
git tag "v$VERSION"

# Publish (tree-sitter first, then shannon)
cargo publish -p tree-sitter-shannon-nu
echo "Waiting for crates.io index..."
sleep 15
cargo publish -p shannonshell

# Push
git push
git push --tags

echo "Published shannonshell v$VERSION"
```

**6. Verify:**

- `cargo publish --dry-run -p tree-sitter-shannon-nu`
- `cargo publish --dry-run -p shannonshell`
- Both must pass before real publish

### Include files for crates.io

The `shannonshell` crate must include these non-Rust files:

```toml
include = [
    "src/**/*.rs",
    "build.rs",
    "completions/**/*.fish",
    "themes/**/*.theme",
    "Cargo.toml",
    "LICENSE",
    "README.md",
]
```

### Metadata

```toml
[package]
name = "shannonshell"
version = "0.1.0"
edition = "2021"
description = "An AI-first shell with seamless access to bash, nushell, and any other shell"
license = "MIT"
repository = "https://github.com/user/shannon"
readme = "README.md"
keywords = ["shell", "nushell", "bash", "ai", "terminal"]
categories = ["command-line-utilities"]

[[bin]]
name = "shannon"
path = "src/main.rs"
```
