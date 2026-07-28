#!/usr/bin/env bash
# Builds objectiveai-viewer (via `tauri build`, so the frontend + icon are
# embedded) and places the binary in embed/<profile>/ — debug and release
# coexist (embed/debug/objectiveai-viewer[.exe], embed/release/...[.exe]).
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

# CEF (the browser tabs' Chromium) compiles a C++ wrapper with cmake, and
# cef-dll-sys hardcodes the Ninja generator — so `ninja` must be on PATH or
# the build dies at "CMake was unable to find a build program". Visual
# Studio ships one; find it rather than making every developer install a
# separate ninja. No-op where ninja is already on PATH (Linux/mac, or a
# Windows box that has it).
ensure_ninja_on_path() {
  if command -v ninja >/dev/null 2>&1; then
    return 0
  fi
  local vs_ninja
  for vs_ninja in \
    "/c/Program Files/Microsoft Visual Studio"/*/*/Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja \
    "/c/Program Files (x86)/Microsoft Visual Studio"/*/*/Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja
  do
    if [ -x "$vs_ninja/ninja.exe" ]; then
      export PATH="$vs_ninja:$PATH"
      echo "$MODULE: using ninja from $vs_ninja"
      return 0
    fi
  done
  echo "$MODULE: WARNING — ninja not found; the CEF wrapper build will fail" >&2
}

run() {
  ensure_ninja_on_path

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

  # --features stdio: this script only ever builds the DAEMON-SPAWNED
  # binary (install.sh from-source and the release zips both come
  # through here), and that binary carries the daemon-owned stdin
  # channel — development-plugin registrations in, acks out,
  # EOF-after-first-frame as graceful shutdown. The dev viewer
  # (`pnpm tauri dev`) never runs this script and stays featureless,
  # which is the point: its stdin can be null, and a null stdin is an
  # instant EOF.
  local tauri_args=("--no-bundle" "--features" "stdio" "--target" "$TARGET")
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

  echo "Building $MODULE ($PROFILE, $TARGET) via tauri build..."
  if ! (cd "$SCRIPT_DIR" && pnpm exec tauri build "${tauri_args[@]}"); then
    return 1
  fi

  # Copy binary into embed/<profile>/ — EMBED_DIR is set + exported by
  # fingerprint.sh (sourced above), so debug and release land in separate
  # folders and never clobber each other.
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
