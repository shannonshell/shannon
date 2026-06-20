#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
REF="${1:-HEAD}"
OUT_DIR="${2:-/tmp/shannon-src}"

cd "$REPO_DIR"

VERSION="$(git show "${REF}:Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"
if [ -z "$VERSION" ]; then
  echo "could not read package version from ${REF}:Cargo.toml" >&2
  exit 1
fi
TREE="$(git rev-parse "${REF}^{tree}")"

mkdir -p "$OUT_DIR"
TARBALL="$OUT_DIR/shannon-${VERSION}.tar.gz"
TMP_TAR="$(mktemp)"
trap 'rm -f "$TMP_TAR"' EXIT

git archive \
  --format=tar \
  --mtime="1970-01-01 00:00:00 +0000" \
  --worktree-attributes \
  --prefix="shannon-${VERSION}/" \
  -o "$TMP_TAR" \
  "$TREE"
gzip -n -c "$TMP_TAR" > "$TARBALL"

SHA="$(shasum -a 256 "$TARBALL" | awk '{print $1}')"

echo "tarball: $TARBALL ($REF)"
echo "sha256:  $SHA"
