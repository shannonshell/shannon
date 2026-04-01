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

# Publish to crates.io (3 crates, in dependency order)
echo "==> Publishing to crates.io..."

try_publish() {
  local crate="$1"
  local manifest="${2:-}"
  local output
  local rc

  echo "  Publishing $crate..."
  if [ -n "$manifest" ]; then
    output=$(cargo publish --manifest-path "$manifest" -p "$crate" --allow-dirty 2>&1) || rc=$?
  else
    output=$(cargo publish --allow-dirty 2>&1) || rc=$?
  fi

  if echo "$output" | grep -q "already exists"; then
    echo "  $crate already published, skipping"
  elif [ "${rc:-0}" -ne 0 ]; then
    echo "$output" >&2
    exit 1
  else
    echo "$output"
    echo "  Waiting for crates.io index..."
    sleep 30
  fi
}

try_publish shannon-nu-cli nushell/Cargo.toml
try_publish shannon-nu-lsp nushell/Cargo.toml
try_publish shannonshell

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
echo "  Install: cargo install shannonshell"
echo "  Or: cargo install --git https://github.com/shannonshell/shannon"
