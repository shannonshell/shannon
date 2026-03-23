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

### tree-sitter-nu: vendor inline

`tree-sitter-nu` is not on crates.io. Rather than publishing it as a separate
crate, we vendor it directly into our project. The C parser source and Rust
bindings live in `tree-sitter-nu/` in our repo. Our `build.rs` compiles the C
source via the `cc` crate. No separate package, no workspace — just one crate
to publish.

### What needs to happen

**1. Vendor tree-sitter-nu into the project:**

- Copy source from `vendor/tree-sitter-nu/` into `tree-sitter-nu/` in the
  repo root (or `src/tree_sitter_nu/` — wherever makes sense)
- Include the C source files (`src/parser.c`, `src/scanner.c`, headers)
- Include the Rust bindings (`bindings/rust/lib.rs`, `bindings/rust/build.rs`)
- Add `cc` as a build dependency
- Update our `build.rs` to compile the C parser
- Create a `src/tree_sitter_nu.rs` module that exposes the language function
- Remove the `tree-sitter-nu` git dependency from Cargo.toml

**2. Update root `Cargo.toml` for `shannonshell`:**

- Rename package to `shannonshell`
- Add `[[bin]]` section: `name = "shannon"`, `path = "src/main.rs"`
- Version: `0.1.0`
- Add metadata: description, license, repository, keywords, categories, readme
- Add `cc` to build-dependencies
- Remove `tree-sitter-nu` git dependency
- Add `include` to ensure all needed files are packaged:
  ```toml
  include = [
      "src/**/*.rs",
      "build.rs",
      "completions/**/*.fish",
      "themes/**/*.theme",
      "tree-sitter-nu/**/*.c",
      "tree-sitter-nu/**/*.h",
      "tree-sitter-nu/**/*.rs",
      "tree-sitter-nu/**/*.json",
      "Cargo.toml",
      "LICENSE",
      "README.md",
  ]
  ```

**3. Update `src/highlighter.rs`:**

- Change `use tree_sitter_nu::LANGUAGE` to use our vendored module

**4. Create `scripts/release.sh`:**

```bash
#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

# Update version
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Test
cargo test

# Commit and tag
git add -A
git commit -m "Release v$VERSION"
git tag "v$VERSION"

# Publish
cargo publish

# Push
git push
git push --tags

echo "Published shannonshell v$VERSION"
```

**5. Verify:**

- `cargo publish --dry-run` must pass before real publish
- `cargo install --path .` must produce a working `shannon` binary

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

### Simplification

No workspace needed. One crate (`shannonshell`), one publish. The
tree-sitter-nu grammar is vendored inline — compiled by our build.rs, no
separate package.
