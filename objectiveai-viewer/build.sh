#!/usr/bin/env bash
# Builds objectiveai-viewer and places the binary in embed/<target>/<profile>/.
# Skips the build if the source fingerprint hasn't changed.
# All arguments are forwarded to cargo build.
# Output is captured to .logs/build/objectiveai-viewer.txt.
#
# Usage:
#   bash objectiveai-viewer/build.sh [--release] [--target <triple>] [...]

set -euo pipefail

MODULE="objectiveai-viewer"
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

  # Use `tauri build --no-bundle` — this is the ONLY supported way to build
  # a Tauri app and properly embed the frontend. It:
  #   1. Runs `beforeBuildCommand` from tauri.conf.json (builds the frontend)
  #   2. Invokes cargo with the right flags
  #   3. Makes tauri-build's include of frontendDist actually reflect latest dist
  # Plain `cargo build -p <crate>` skips step 1 and can use stale cached
  # builds that don't re-embed updated frontend assets.
  #
  # --no-bundle skips the installer/msi/nsis generation (we just want the exe).

  local tauri_args=("--no-bundle" "--target" "$TARGET")
  if [ "$PROFILE" = "release" ]; then
    :  # tauri build defaults to release
  else
    tauri_args+=("--debug")
  fi

  # Ensure JS dependencies (including @tauri-apps/cli) are installed.
  # On a fresh checkout — local or in CI — there is no node_modules yet,
  # so `pnpm exec tauri` would fail with "Command \"tauri\" not found".
  # We run install from the repo root so pnpm picks up the workspace.
  echo "Installing $MODULE dependencies via pnpm..."
  if ! (cd "$REPO_ROOT" && pnpm install --frozen-lockfile); then
    return 1
  fi

  # Build the viewer's workspace dependency before the frontend
  # compiles. @objectiveai/function-tree is `workspace:*` — its
  # dist/ (type declarations included) must exist or tsc fails with
  # TS2307 on a fresh checkout (CI release legs build the viewer
  # directly, without the root build.sh's function-tree phase).
  echo "Building @objectiveai/function-tree (workspace dependency)..."
  if ! (cd "$REPO_ROOT" && pnpm --filter @objectiveai/function-tree run build); then
    return 1
  fi

  echo "Building $MODULE ($PROFILE, $TARGET) via tauri build..."
  if ! (cd "$SCRIPT_DIR" && pnpm exec tauri build "${tauri_args[@]}"); then
    return 1
  fi

  # Copy binary to embed/<target>/<profile>/
  EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"
  mkdir -p "$EMBED_DIR"

  if [[ "$TARGET" == *"windows"* ]]; then
    BINARY_NAME="$MODULE.exe"
  else
    BINARY_NAME="$MODULE"
  fi

  # tauri build uses the workspace's default target dir: target/<triple>/<profile>/
  BUILT="$REPO_ROOT/target/$TARGET/$PROFILE/$BINARY_NAME"
  if [ ! -f "$BUILT" ]; then
    echo "ERROR: expected binary at $BUILT" >&2
    return 1
  fi

  cp "$BUILT" "$EMBED_DIR/$BINARY_NAME"

  # Stamp the fingerprint only after successful build
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "Build complete (fingerprint: ${CURRENT_FP:0:12}...)"
}

if run "$@" > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi
