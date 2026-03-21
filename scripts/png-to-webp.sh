#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "Usage: $0 <input.png> <output.webp>"
  exit 1
fi

cwebp -lossless "$1" -o "$2"
