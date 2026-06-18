#!/usr/bin/env bash
# Computes a SHA256 fingerprint of all source files that affect the viewer build.
#
# Usage:
#   source fingerprint.sh [--target <triple>] [--release]
#
# Exports: CURRENT_FP, FINGERPRINT_FILE, TARGET, PROFILE, EMBED_DIR
#          (EMBED_DIR is embed/<profile>/ — debug and release stay separate)
# Returns 0 if fingerprint changed (build needed), 1 if up to date.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

# The binary lands in embed/<profile>/ — debug and release coexist in
# separate folders, each with its own .fingerprint (no <target> subdir;
# each checkout builds for its own host). TARGET/PROFILE drive the tauri
# build; PROFILE also selects the per-profile embed dir and is folded into
# the hash, so debug vs release never share a fingerprint or a binary.
EMBED_DIR="$SCRIPT_DIR/embed/$PROFILE"
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
    # Backend (Rust) sources
    find "$SCRIPT_DIR/src-tauri/src" -type f -name '*.rs' | sort
    echo "$SCRIPT_DIR/src-tauri/Cargo.toml"
    echo "$SCRIPT_DIR/src-tauri/tauri.conf.json"
    # Frontend (TS/CSS/HTML) sources — fingerprint inputs, NOT outputs.
    # `dist/` is regenerated on every build (tauri's beforeBuildCommand
    # runs `pnpm run build`), so hashing dist would invalidate the
    # fingerprint between stamp-time and validate-time.
    find "$SCRIPT_DIR/src" -type f 2>/dev/null | sort
    echo "$SCRIPT_DIR/index.html"
    echo "$SCRIPT_DIR/package.json"
    echo "$SCRIPT_DIR/tsconfig.json"
    echo "$SCRIPT_DIR/vite.config.ts"
    find "$REPO_ROOT/objectiveai-sdk-rs/src" -type f -name '*.rs' | sort
    echo "$REPO_ROOT/objectiveai-sdk-rs/Cargo.toml"
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
      # Non-file lines (like PROFILE=release) — hash as-is
      printf '%s\n' "$file"
    fi
  done | _sha256 | awk '{print $1}'
}

CURRENT_FP=$(compute_fingerprint)
export CURRENT_FP FINGERPRINT_FILE TARGET PROFILE EMBED_DIR

if [ -f "$FINGERPRINT_FILE" ]; then
  STORED_FP=$(cat "$FINGERPRINT_FILE")
  if [ "$CURRENT_FP" = "$STORED_FP" ]; then
    echo "embed/ is up to date ($PROFILE, fingerprint: ${CURRENT_FP:0:12}...)"
    return 1 2>/dev/null || exit 1
  fi
  echo "Fingerprint changed: ${STORED_FP:0:12}... -> ${CURRENT_FP:0:12}..."
fi
