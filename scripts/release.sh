#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version> [--dry-run] [--resume]}"
DRY_RUN=false
RESUME=false
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    --resume)  RESUME=true ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

# Check for cargo-workspaces
if ! command -v cargo-workspaces &>/dev/null; then
  echo "Error: cargo-workspaces not found. Install with:"
  echo "  cargo install cargo-workspaces"
  exit 1
fi

echo "==> Releasing shannonshell v$VERSION"
if $DRY_RUN; then
  echo "  (dry run — nothing will be published)"
fi

# --- Step 1: Update shannonshell version ---
echo "==> Updating version..."
cd "$REPO_DIR/shannon"
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# --- Step 2: Test ---
if $RESUME; then
  echo "==> Skipping tests (--resume)"
else
  echo "==> Running tests..."
  cargo test
fi

# --- Step 3: Publish reedline (single crate) ---
echo "==> Publishing shannon-reedline..."
cd "$REPO_DIR/reedline"
if $DRY_RUN; then
  cargo publish --allow-dirty --dry-run
elif cargo publish --allow-dirty 2>&1 | tee /dev/stderr | grep -q "already exists"; then
  echo "  (already published, skipping)"
fi
echo "  Waiting for crates.io indexing..."
$DRY_RUN || sleep 10

# --- Step 4: Publish nushell crates (cargo-workspaces handles ordering + cycles) ---
echo "==> Publishing shannon-nu-* crates via cargo-workspaces..."
cd "$REPO_DIR/nushell"
if $DRY_RUN; then
  echo "  (skipping dry-run for workspace publish — not supported by cargo-workspaces)"
else
  cargo workspaces publish --from-git --allow-dirty --no-verify \
    --yes 2>&1 || true
  # cargo-workspaces may report errors for already-published crates; that's OK
fi

# --- Step 5: Publish brush crates (only our three renamed crates) ---
echo "==> Publishing shannon-brush-* crates..."
cd "$REPO_DIR/brush"
for crate in shannon-brush-parser shannon-brush-core shannon-brush-builtins; do
  echo "  Publishing $crate..."
  if $DRY_RUN; then
    cargo publish -p "$crate" --allow-dirty --dry-run 2>&1 || true
  else
    output=$(cargo publish -p "$crate" --allow-dirty 2>&1) || true
    echo "  $(echo "$output" | tail -1)"
    if echo "$output" | grep -q "already exists"; then
      echo "  (already published, skipping)"
    fi
  fi
done

# --- Step 6: Publish shannonshell ---
echo "==> Publishing shannonshell..."
cd "$REPO_DIR/shannon"
if $DRY_RUN; then
  cargo publish --allow-dirty --dry-run
else
  cargo publish --allow-dirty
fi

if $DRY_RUN; then
  echo "==> Dry run complete. No changes committed or published."
  exit 0
fi

# --- Step 7: Commit, tag, push ---
echo "==> Committing and tagging..."
cd "$REPO_DIR"
git add -A
if git diff --cached --quiet; then
  echo "  No changes to commit (version already set)"
else
  git commit -m "Release v$VERSION"
fi
git tag -f "v$VERSION"

echo "==> Pushing..."
git push
git push --tags

echo "==> Published shannonshell v$VERSION"
