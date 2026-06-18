#!/usr/bin/env bash
# Builds objectiveai-sdk-rs-wasm-js to dist/.
# Skips the build if the source fingerprint hasn't changed (SHA-based, like cargo).
# Output is captured to .logs/build/objectiveai-sdk-rs-wasm-js.txt.
#
# Build profile defaults to debug (wasm-pack --dev). Pass --release (or set
# OBJECTIVEAI_BUILD_RELEASE=1, which the root build.sh exports) for an
# optimized build (wasm-pack --release). The profile is part of the
# fingerprint, so switching profiles forces a rebuild.
#
# Usage:
#   bash objectiveai-sdk-rs-wasm-js/build.sh [--release]

set -euo pipefail

MODULE="objectiveai-sdk-rs-wasm-js"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

# ── Resolve build profile ──────────────────────────────────────────────
# --release flag or OBJECTIVEAI_BUILD_RELEASE=1 → release; default debug.
PROFILE="debug"
[ "${OBJECTIVEAI_BUILD_RELEASE:-}" = "1" ] && PROFILE="release"
for arg in "$@"; do
  case "$arg" in
    --release) PROFILE="release" ;;
    *) echo "$MODULE: unknown argument: $arg" >&2; exit 1 ;;
  esac
done
if [ "$PROFILE" = "release" ]; then
  WASM_PACK_PROFILE_FLAG="--release"
else
  WASM_PACK_PROFILE_FLAG="--dev"
fi

PROFILE_MARKER="$SCRIPT_DIR/dist/.profile"

run() {
  # Check fingerprint — returns 1 if dist/ source is up to date. The
  # fingerprint is profile-agnostic, so the built profile is tracked in a
  # separate dist/.profile marker: skip only when BOTH the source is
  # unchanged AND the existing dist/ matches the wanted profile.
  if ! source "$SCRIPT_DIR/fingerprint.sh"; then
    if [ -f "$PROFILE_MARKER" ] && [ "$(cat "$PROFILE_MARKER" 2>/dev/null)" = "$PROFILE" ]; then
      return 0
    fi
    echo "Build profile -> $PROFILE; rebuilding."
  fi

  # Require wasm-pack from repo root bin/
  WASM_PACK="$REPO_ROOT/bin/wasm-pack"
  [ -x "$WASM_PACK" ] || { echo "ERROR: wasm-pack not found at $WASM_PACK. Run 'bash build.sh' from the repo root first." >&2; return 1; }

  # Build
  echo "Building wasm-pack (nodejs, $PROFILE)..."
  if ! "$WASM_PACK" build "$SCRIPT_DIR" --target nodejs "$WASM_PACK_PROFILE_FLAG" --out-dir dist; then
    return 1
  fi

  # Stamp the fingerprint + profile marker only after successful build
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "$PROFILE" > "$PROFILE_MARKER"
  echo "Build complete ($PROFILE, fingerprint: ${CURRENT_FP:0:12}...)"
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS ($PROFILE)"
else
  echo "$MODULE: ERROR"
  exit 1
fi
