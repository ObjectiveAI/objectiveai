#!/usr/bin/env bash
# Builds objectiveai-db-proxy and places the binary in
# embed/<target>/<profile>/. Skips the build if the source fingerprint
# hasn't changed. All arguments are forwarded to cargo build.
# Output is captured to .logs/build/objectiveai-db-proxy.txt.
#
# Default target is `<host-arch>-unknown-linux-musl` — same convention
# as objectiveai-mcp-laboratory, since this binary is meant to be copied
# into plugin containers. Override with `--target <triple>` for
# local-host builds.
#
# Usage:
#   bash objectiveai-db-proxy/build.sh [--release] [--target <triple>] [...]

set -euo pipefail

MODULE="objectiveai-db-proxy"
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

  # Build with a separate target dir to avoid cargo lock contention with
  # any embedder that's also building. Always pass --target so the output
  # lands in <target-dir>/<triple>/<profile>/.
  TARGET_DIR="$REPO_ROOT/target-$MODULE"

  # Ensure the target's std is installed (idempotent; needed for both native
  # musl on Linux and cross-compiling from another host).
  if command -v rustup >/dev/null 2>&1; then
    rustup target add "$TARGET" >/dev/null 2>&1 || true
  fi

  # Pick the build command. On a Linux host the musl target links locally, so
  # plain `cargo build` works with no extra toolchain. On a non-Linux host we
  # cross-compile to linux-musl with cargo-zigbuild (zig as the cross
  # linker/CC). That toolchain is REQUIRED — error clearly if it is missing
  # rather than producing a broken or wrong-platform binary.
  local -a BUILD_CMD
  if [ "$(uname -s)" = "Linux" ]; then
    BUILD_CMD=(cargo build)
  else
    if ! command -v cargo-zigbuild >/dev/null 2>&1; then
      echo "ERROR: building $MODULE for $TARGET from a non-Linux host needs cargo-zigbuild." >&2
      echo "  Install it:  cargo install cargo-zigbuild" >&2
      echo "  And install zig:  pip install ziglang   (or see https://ziglang.org/download/)" >&2
      return 1
    fi
    if ! command -v zig >/dev/null 2>&1 \
       && ! python3 -c "import ziglang" >/dev/null 2>&1 \
       && ! python -c "import ziglang" >/dev/null 2>&1; then
      echo "ERROR: cargo-zigbuild needs the zig compiler to build $TARGET." >&2
      echo "  Install zig:  pip install ziglang   (or see https://ziglang.org/download/)" >&2
      return 1
    fi
    BUILD_CMD=(cargo zigbuild)
  fi

  echo "Building $MODULE ($PROFILE, $TARGET) via ${BUILD_CMD[*]}..."
  if ! "${BUILD_CMD[@]}" -p "$MODULE" --target-dir "$TARGET_DIR" --target "$TARGET" "$@"; then
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

  BUILT="$TARGET_DIR/$TARGET/$PROFILE/$BINARY_NAME"
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
