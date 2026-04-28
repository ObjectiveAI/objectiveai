#!/usr/bin/env bash
# Computes a SHA256 fingerprint of all source files that affect the
# objectiveai-mcp-filesystem build (its own src + Cargo.lock; no
# transitive workspace deps).
#
# Usage:
#   source fingerprint.sh [--target <triple>] [--release]
#
# Exports: CURRENT_FP, FINGERPRINT_FILE, TARGET, PROFILE
# Returns 0 if fingerprint changed (build needed), 1 if up to date.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Parse --target and --release from args. Default target is musl —
# this binary is meant to be embedded into the same kind of Docker
# container injection scenarios as the legacy objectiveai-mcp.
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
  arch=$(rustc -vV | grep '^host:' | awk '{print $2}' | cut -d- -f1)
  TARGET="${arch}-unknown-linux-musl"
fi

EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"
FINGERPRINT_FILE="$EMBED_DIR/.fingerprint"

# macOS ships `shasum` (Perl) but not GNU `sha256sum`; prefer the latter
# when present so hashes match across Linux-based builders exactly.
if command -v sha256sum >/dev/null 2>&1; then
  _sha256() { sha256sum "$@"; }
else
  _sha256() { shasum -a 256 "$@"; }
fi

compute_fingerprint() {
  {
    # Include profile in fingerprint so debug != release
    echo "PROFILE=$PROFILE"
    find "$SCRIPT_DIR/src" -type f -name '*.rs' | sort
    echo "$SCRIPT_DIR/Cargo.toml"
    echo "$REPO_ROOT/Cargo.lock"
  } | while IFS= read -r file; do
    if [ -f "$file" ]; then
      relpath="${file#"$REPO_ROOT/"}"
      printf '%s\n' "$relpath"
      # Strip the path from the hash line — sha256sum's default output
      # `<hash>  <path>` would otherwise embed the runner's absolute path
      # (different on Linux, macOS, Windows) and break cross-runner
      # fingerprint matching.
      _sha256 "$file" | awk '{print $1}'
    else
      printf '%s\n' "$file"
    fi
  done | _sha256 | awk '{print $1}'
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
