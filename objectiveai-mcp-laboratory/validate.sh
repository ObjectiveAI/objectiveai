#!/usr/bin/env bash
# Validates that embed/<target>/<profile>/ exists and its fingerprint matches.
# Used by the embedder's build.rs to fail fast when the embedded binary
# is stale.
#
# Usage:
#   bash objectiveai-mcp-laboratory/validate.sh [--target <triple>] [--release]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Source fingerprint.sh to get CURRENT_FP, FINGERPRINT_FILE, TARGET, PROFILE.
# It returns 1 (up to date) or 0 (changed) — suppress the early exit.
source "$SCRIPT_DIR/fingerprint.sh" "$@" || true

EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"

if [ ! -d "$EMBED_DIR" ] || [ ! -f "$FINGERPRINT_FILE" ]; then
  echo "ERROR: embed/$TARGET/$PROFILE is missing. Run build.sh first." >&2
  exit 1
fi

STORED_FP=$(cat "$FINGERPRINT_FILE")
if [ "$CURRENT_FP" != "$STORED_FP" ]; then
  echo "ERROR: embed/$TARGET/$PROFILE is stale. Fingerprint mismatch:" >&2
  echo "  stored:  ${STORED_FP:0:12}..." >&2
  echo "  current: ${CURRENT_FP:0:12}..." >&2
  echo "Run build.sh to rebuild." >&2
  exit 2
fi

echo "embed/$TARGET/$PROFILE is valid (fingerprint: ${CURRENT_FP:0:12}...)"
