#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
cd "$REPO_DIR"

echo "==> Releasing shannon v$VERSION"

# Bump version in all shannon crates
echo "==> Bumping versions to $VERSION..."

# shannonshell (root) — line 3 is the version
sed -i '' "3s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# shannon-nu-cli package version
sed -i '' "s/^version = .*/version = \"$VERSION\"/" nushell/crates/nu-cli/Cargo.toml

# shannon-nu-lsp package version
sed -i '' "s/^version = .*/version = \"$VERSION\"/" nushell/crates/nu-lsp/Cargo.toml

# Update dep versions on lines referencing shannon-* packages
# (only these lines have "shannon-nu-" so the pattern is safe)
sed -i '' "/shannon-nu-/s/version = \"[^\"]*\"/version = \"$VERSION\"/g" Cargo.toml
sed -i '' "/shannon-nu-/s/version = \"[^\"]*\"/version = \"$VERSION\"/g" nushell/Cargo.toml
sed -i '' "/shannon-nu-/s/version = \"[^\"]*\"/version = \"$VERSION\"/g" nushell/crates/nu-lsp/Cargo.toml

# Build
echo "==> Building (release)..."
cargo build --release

# Test
echo "==> Running tests..."
cargo test

# Publish to crates.io (3 crates, in dependency order)
echo "==> Publishing to crates.io..."

echo "  Publishing shannon-nu-cli@$VERSION..."
cargo publish --manifest-path nushell/Cargo.toml -p shannon-nu-cli --allow-dirty
echo "  Waiting for crates.io index..."
sleep 30

echo "  Publishing shannon-nu-lsp@$VERSION..."
cargo publish --manifest-path nushell/Cargo.toml -p shannon-nu-lsp --allow-dirty
echo "  Waiting for crates.io index..."
sleep 30

echo "  Publishing shannonshell@$VERSION..."
cargo publish --allow-dirty

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
