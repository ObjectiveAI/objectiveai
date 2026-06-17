#!/usr/bin/env bash
# Builds objectiveai-sdk-rs-wasm-js to dist/.
# Skips the build if the source fingerprint hasn't changed (SHA-based, like cargo).
# Output is captured to .logs/build/objectiveai-sdk-rs-wasm-js.txt.
#
# Usage:
#   bash objectiveai-sdk-rs-wasm-js/build.sh

set -euo pipefail

MODULE="objectiveai-sdk-rs-wasm-js"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

run() {
  # Check fingerprint — returns 1 if dist/ is up to date (not an error).
  if ! source "$SCRIPT_DIR/fingerprint.sh"; then
    return 0
  fi

  # Require wasm-pack from repo root bin/
  WASM_PACK="$REPO_ROOT/bin/wasm-pack"
  [ -x "$WASM_PACK" ] || { echo "ERROR: wasm-pack not found at $WASM_PACK. Run 'bash build.sh' from the repo root first." >&2; return 1; }

  # Build
  echo "Building wasm-pack (nodejs, release)..."
  if ! "$WASM_PACK" build "$SCRIPT_DIR" --target nodejs --release --out-dir dist; then
    return 1
  fi

  # Stamp the fingerprint only after successful build
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "Build complete (fingerprint: ${CURRENT_FP:0:12}...)"
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
