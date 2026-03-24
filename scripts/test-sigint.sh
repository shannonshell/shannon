#!/usr/bin/env bash
# Test that shannon survives SIGINT during subprocess execution.
# Spawns shannon, sends it a sleep command, sends SIGINT to the
# process group, then checks if shannon is still alive.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="${1:-$REPO_DIR/shannon/target/debug/shannon}"

if [ ! -f "$BINARY" ]; then
  echo "Error: binary not found at $BINARY"
  echo "Usage: $0 [path-to-shannon-binary]"
  exit 1
fi

echo "Testing SIGINT handling..."

# Start shannon in a new process group with a pseudo-terminal
# Use script(1) to give it a TTY
FIFO=$(mktemp -u)
mkfifo "$FIFO"

# Run shannon with a sleep command, capture its PID
script -q /dev/null "$BINARY" < "$FIFO" &>/dev/null &
SHANNON_PID=$!

sleep 1

# Send a sleep command
echo "sleep 10" > "$FIFO" &

sleep 0.5

# Send SIGINT to shannon (simulating Ctrl+C)
kill -INT "$SHANNON_PID" 2>/dev/null || true

sleep 0.5

# Check if shannon is still alive
if kill -0 "$SHANNON_PID" 2>/dev/null; then
  echo "PASS: shannon survived SIGINT"
  # Clean up — send exit command then kill
  echo "exit" > "$FIFO" &
  sleep 0.5
  kill "$SHANNON_PID" 2>/dev/null || true
  rm -f "$FIFO"
  exit 0
else
  echo "FAIL: shannon was killed by SIGINT"
  rm -f "$FIFO"
  exit 1
fi
