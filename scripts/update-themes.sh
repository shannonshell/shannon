#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
FISH_THEMES="$REPO_DIR/vendor/fish/share/themes"
OUT_DIR="$REPO_DIR/themes"

if [ ! -d "$FISH_THEMES" ]; then
  echo "Error: vendor/fish not found. Clone fish into vendor/ first."
  exit 1
fi

mkdir -p "$OUT_DIR"

# Copy fish themes (don't overwrite custom themes like tokyo-night.theme)
for f in "$FISH_THEMES"/*.theme; do
  name=$(basename "$f")
  if [ ! -f "$OUT_DIR/$name" ] || grep -q "^# source: fish" "$OUT_DIR/$name" 2>/dev/null; then
    cp "$f" "$OUT_DIR/$name"
  fi
done

COUNT=$(ls "$OUT_DIR"/*.theme | wc -l | tr -d ' ')
echo "Copied fish themes to themes/ ($COUNT total including custom)"
