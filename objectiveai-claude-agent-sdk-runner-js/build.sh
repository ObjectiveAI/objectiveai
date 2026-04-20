#!/usr/bin/env bash
# Builds the JS Claude Agent SDK runner using yao-pkg.
# Places the binary in embed/<target>/<profile>/.
# Skips the build if the source fingerprint hasn't changed.
# Output is captured to .logs/build/objectiveai-claude-agent-sdk-runner-js.txt.
#
# Usage:
#   bash objectiveai-claude-agent-sdk-runner-js/build.sh [--release] [--target <triple>]

set -euo pipefail

MODULE="objectiveai-claude-agent-sdk-runner-js"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

run() {
  # Check fingerprint — returns 1 if embed/ is up to date (not an error).
  if ! source "$SCRIPT_DIR/fingerprint.sh" "$@"; then
    return 0
  fi

  # Install dependencies if needed
  if [ ! -d "$SCRIPT_DIR/node_modules" ]; then
    echo "Installing dependencies..."
    (cd "$SCRIPT_DIR" && npm install --omit=dev 2>&1) || return 1
  fi

  # Determine pkg target
  local pkg_target
  case "$TARGET" in
    x86_64-pc-windows-msvc|x86_64-pc-windows-gnu)
      pkg_target="node20-win-x64" ;;
    aarch64-pc-windows-msvc)
      pkg_target="node20-win-arm64" ;;
    x86_64-apple-darwin)
      pkg_target="node20-macos-x64" ;;
    aarch64-apple-darwin)
      pkg_target="node20-macos-arm64" ;;
    x86_64-unknown-linux-*)
      pkg_target="node20-linux-x64" ;;
    aarch64-unknown-linux-*)
      pkg_target="node20-linux-arm64" ;;
    *)
      echo "ERROR: unsupported target $TARGET for pkg" >&2
      return 1 ;;
  esac

  # Binary name
  local binary_name
  if [[ "$TARGET" == *"windows"* ]]; then
    binary_name="$MODULE.exe"
  else
    binary_name="$MODULE"
  fi

  # Build with yao-pkg
  EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"
  mkdir -p "$EMBED_DIR"

  echo "Building $MODULE ($PROFILE, $TARGET)..."
  (cd "$SCRIPT_DIR" && npx -y -p @yao-pkg/pkg pkg main.js \
    --target "$pkg_target" \
    --output "$EMBED_DIR/$binary_name" \
    --config package.json) || return 1

  # Stamp the fingerprint
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "Build complete (fingerprint: ${CURRENT_FP:0:12}...)"
}

if run "$@" > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi
