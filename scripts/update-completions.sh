#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
FISH_COMPLETIONS="$REPO_DIR/vendor/fish/share/completions"
OUT_DIR="$REPO_DIR/completions"

if [ ! -d "$FISH_COMPLETIONS" ]; then
  echo "Error: vendor/fish not found. Clone fish into vendor/ first."
  exit 1
fi

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
cp "$FISH_COMPLETIONS"/*.fish "$OUT_DIR/"

COUNT=$(ls "$OUT_DIR"/*.fish | wc -l | tr -d ' ')
echo "Copied $COUNT fish completion files to completions/"
