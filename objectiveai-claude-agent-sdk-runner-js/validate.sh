#!/usr/bin/env bash
# Validates that the JS runner embed exists and its fingerprint matches.
#
# Usage:
#   bash objectiveai-claude-agent-sdk-runner-js/validate.sh [--target <triple>] [--release]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Source fingerprint to compute current hash and get EMBED_DIR
source "$SCRIPT_DIR/fingerprint.sh" "$@" || {
  # fingerprint.sh returns 1 if up to date — that means valid
  exit 0
}

# If we get here, fingerprint changed → stale
echo "ERROR: embed/$TARGET/$PROFILE is stale. Fingerprint mismatch:" >&2
echo "  stored:  $(cat "$FINGERPRINT_FILE" 2>/dev/null || echo '(none)')..." >&2
echo "  current: ${CURRENT_FP:0:12}..." >&2
echo "Run build.sh to rebuild." >&2
exit 2
