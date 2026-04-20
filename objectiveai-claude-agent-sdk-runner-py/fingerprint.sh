#!/usr/bin/env bash
# Computes a SHA256 fingerprint of all source files that affect the build.
# This mirrors cargo's approach: hash every input, so any change is detected
# regardless of filesystem timestamps, clock skew, or copied files.
#
# Usage:
#   source fingerprint.sh [--target <triple>] [--release]
#
# Exports: CURRENT_FP, FINGERPRINT_FILE, TARGET, PROFILE
# Returns 0 if fingerprint changed (build needed), 1 if up to date.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse --target and --release from args
TARGET=""
PROFILE="debug"
prev_was_target=0
for arg in "$@"; do
  if [ "$prev_was_target" = "1" ]; then
    TARGET="$arg"
    prev_was_target=0
    continue
  fi
  prev_was_target=0
  [ "$arg" = "--target" ] && prev_was_target=1
  [ "$arg" = "--release" ] && PROFILE="release"
done
if [ -z "$TARGET" ]; then
  TARGET=$(rustc -vV | grep '^host:' | awk '{print $2}')
fi

EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"
FINGERPRINT_FILE="$EMBED_DIR/.fingerprint"

compute_fingerprint() {
  {
    # Include profile in fingerprint so debug != release
    echo "PROFILE=$PROFILE"
    echo "$SCRIPT_DIR/main.py"
    echo "$SCRIPT_DIR/requirements.txt"
    echo "$SCRIPT_DIR/requirements-dev.txt"
  } | while IFS= read -r file; do
    if [ -f "$file" ]; then
      relpath="${file#"$SCRIPT_DIR/"}"
      printf '%s\n' "$relpath"
      sha256sum "$file"
    else
      printf '%s\n' "$file"
    fi
  done | sha256sum | awk '{print $1}'
}

CURRENT_FP=$(compute_fingerprint)
export CURRENT_FP FINGERPRINT_FILE TARGET PROFILE

if [ -f "$FINGERPRINT_FILE" ]; then
  STORED_FP=$(cat "$FINGERPRINT_FILE")
  if [ "$CURRENT_FP" = "$STORED_FP" ]; then
    echo "embed/$TARGET/$PROFILE is up to date (fingerprint: ${CURRENT_FP:0:12}...)"
    return 1 2>/dev/null || exit 1
  fi
  echo "Fingerprint changed: ${STORED_FP:0:12}... -> ${CURRENT_FP:0:12}..."
fi
