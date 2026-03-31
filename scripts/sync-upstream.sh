#!/usr/bin/env bash
set -euo pipefail

# NEVER use --squash. Full history must be preserved.

DATE=$(date +%Y-%m-%d)

echo "==> Syncing nushell upstream..."
git subtree pull --prefix nushell upstream-nushell main \
  -m "Merge nushell upstream $DATE"

echo "==> Syncing brush upstream..."
git subtree pull --prefix brush upstream-brush main \
  -m "Merge brush upstream $DATE"

echo "==> Syncing reedline upstream..."
git subtree pull --prefix reedline upstream-reedline main \
  -m "Merge reedline upstream $DATE"

echo "==> Building..."
cargo build

echo "==> Testing..."
cargo test

echo "==> Upstream sync complete."
