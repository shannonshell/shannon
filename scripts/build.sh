#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

RELEASE=false
CLEAN=false

for arg in "$@"; do
  case "$arg" in
    --release) RELEASE=true ;;
    --clean)   CLEAN=true ;;
    *)
      echo "Usage: $0 [--release] [--clean]"
      exit 1
      ;;
  esac
done

cd "$REPO_DIR"

if $CLEAN; then
  echo "==> Cleaning..."
  cargo clean
fi

if $RELEASE; then
  echo "==> Building shannon (release)..."
  cargo build --release
  echo "  $REPO_DIR/target/release/shannon"
else
  echo "==> Building shannon (debug)..."
  cargo build
  echo "  $REPO_DIR/target/debug/shannon"
fi
