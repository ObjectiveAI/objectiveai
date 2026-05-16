#!/usr/bin/env bash
# Generates Go types from JSON schemas and installs CFFI WASM binary.
# Output is captured to .logs/build/objectiveai-sdk-go.txt.
#
# Usage:
#   bash objectiveai-sdk-go/build.sh

set -euo pipefail

MODULE="objectiveai-sdk-go"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

run() {
  # Generate types from JSON schemas, then install CFFI WASM binary.
  # Chained with && so a failure in the first command aborts the second
  # (set -e is disabled inside `if` conditions, so rely on exit status).
  go run "$SCRIPT_DIR/scripts/install_go.go" && \
    go run "$SCRIPT_DIR/scripts/install_cffi.go"
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
