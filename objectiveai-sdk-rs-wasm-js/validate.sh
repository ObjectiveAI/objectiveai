#!/usr/bin/env bash
# Validates that dist/ exists and its fingerprint matches the current source.
#
# Usage:
#   bash objectiveai-sdk-rs-wasm-js/validate.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
FINGERPRINT_FILE="$DIST_DIR/.fingerprint"

if [ ! -d "$DIST_DIR" ] || [ ! -f "$FINGERPRINT_FILE" ]; then
  echo "ERROR: dist/ is missing. Run build.sh first." >&2
  exit 1
fi

# Compute current fingerprint (source exits early if up to date, so we
# suppress that by only importing the compute logic).
source "$SCRIPT_DIR/fingerprint.sh" || true

STORED_FP=$(cat "$FINGERPRINT_FILE")
if [ "$CURRENT_FP" != "$STORED_FP" ]; then
  echo "ERROR: dist/ is stale. Fingerprint mismatch:" >&2
  echo "  stored:  ${STORED_FP:0:12}..." >&2
  echo "  current: ${CURRENT_FP:0:12}..." >&2
  echo "Run build.sh to rebuild." >&2
  exit 2
fi

echo "dist/ is valid (fingerprint: ${CURRENT_FP:0:12}...)"
