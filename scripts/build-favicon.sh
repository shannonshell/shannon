#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

INPUT="$REPO_DIR/assets/shannon-1.png"
OUTPUT="$REPO_DIR/website/public/favicon.ico"
TMP_PNG="$(mktemp).png"

sips -z 32 32 "$INPUT" --out "$TMP_PNG" >/dev/null 2>&1
# ICO is just a BMP/PNG container — a 32x32 PNG works as favicon
cp "$TMP_PNG" "$OUTPUT"
rm -f "$TMP_PNG"
echo "  $INPUT → $OUTPUT"
