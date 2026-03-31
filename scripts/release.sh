#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR"

echo "==> Releasing shannon v$VERSION"

# Update version in Cargo.toml
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Build
echo "==> Building (release)..."
cargo build --release

# Test
echo "==> Running tests..."
cargo test

# Commit and tag
echo "==> Committing and tagging..."
git add -A
if git diff --cached --quiet; then
  echo "  No changes to commit (version already set)"
else
  git commit -m "Release v$VERSION"
fi
git tag -f "v$VERSION"

# Push
echo "==> Pushing..."
git push
git push --tags

echo "==> Released shannon v$VERSION"
echo "  Install: cargo install --git https://github.com/shannonshell/shannon"
