#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$REPO_DIR/shannon/target/release/shannon"

if [ ! -f "$BINARY" ]; then
  echo "Error: Release build not found at $BINARY"
  echo "Run: scripts/build.sh --release"
  exit 1
fi

# Re-exec as root so we only prompt for the password once.
if [ "$(id -u)" -ne 0 ]; then
  exec sudo "$0" "$@"
fi

echo "==> Installing shannon to /usr/local/bin/shannon..."
cp "$BINARY" /usr/local/bin/shannon
codesign --force --sign - /usr/local/bin/shannon
echo "  Bin: /usr/local/bin/shannon"
