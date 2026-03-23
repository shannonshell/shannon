#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR"

echo "==> Releasing shannonshell v$VERSION"

# Update version in Cargo.toml
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Test
echo "==> Running tests..."
cargo test

# Dry run
echo "==> Dry run..."
cargo publish --dry-run

# Commit and tag
echo "==> Committing and tagging..."
git add -A
git commit -m "Release v$VERSION"
git tag "v$VERSION"

# Publish
echo "==> Publishing to crates.io..."
cargo publish

# Push
echo "==> Pushing..."
git push
git push --tags

echo "==> Published shannonshell v$VERSION"
