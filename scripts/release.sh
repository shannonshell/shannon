#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR/shannon"

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
cd "$REPO_DIR"
git add -A
if git diff --cached --quiet; then
  echo "  No changes to commit (version already set)"
else
  git commit -m "Release v$VERSION"
fi
git tag -f "v$VERSION"

# Publish
echo "==> Publishing to crates.io..."
cd "$REPO_DIR/shannon"
cargo publish

# Push
echo "==> Pushing..."
cd "$REPO_DIR"
git push
git push --tags

echo "==> Published shannonshell v$VERSION"
