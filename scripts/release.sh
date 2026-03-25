#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version> [--dry-run]}"
DRY_RUN=false
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR/shannon"

echo "==> Releasing shannonshell v$VERSION"
if $DRY_RUN; then
  echo "  (dry run — nothing will be published)"
fi

# Update version in Cargo.toml
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Test
echo "==> Running tests..."
cargo test

# Dry run
echo "==> Dry run..."
cargo publish --dry-run --allow-dirty

if $DRY_RUN; then
  echo "==> Dry run complete. No changes committed or published."
  exit 0
fi

# Publish
echo "==> Publishing to crates.io..."
cargo publish --allow-dirty

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

# Push
echo "==> Pushing..."
git push
git push --tags

echo "==> Published shannonshell v$VERSION"
