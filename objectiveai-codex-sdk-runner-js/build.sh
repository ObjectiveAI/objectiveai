#!/usr/bin/env bash
# Builds the JS Codex SDK runner using yao-pkg.
# Places the binary in embed/<target>/<profile>/.
# Skips the build if the source fingerprint hasn't changed.
# Output is captured to .logs/build/objectiveai-codex-sdk-runner-js.txt.
#
# Usage:
#   bash objectiveai-codex-sdk-runner-js/build.sh [--release] [--target <triple>]

set -euo pipefail

MODULE="objectiveai-codex-sdk-runner-js"
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

  # Install dependencies if needed (includes devDependencies — esbuild).
  if [ ! -d "$SCRIPT_DIR/node_modules" ] || [ ! -d "$SCRIPT_DIR/node_modules/esbuild" ]; then
    echo "Installing dependencies..."
    (cd "$SCRIPT_DIR" && npm install 2>&1) || return 1
  fi

  # Pre-bundle ESM sources into a single CJS file that yao-pkg can ingest.
  # yao-pkg can't resolve ESM-only packages via its internal CJS loader, so we
  # collapse everything (main.js + @openai/codex-sdk) into one CJS bundle first.
  BUNDLE_DIR="$SCRIPT_DIR/.bundle"
  mkdir -p "$BUNDLE_DIR"
  echo "Bundling ESM sources with esbuild..."
  (cd "$SCRIPT_DIR" && ./node_modules/.bin/esbuild main.js \
    --bundle \
    --platform=node \
    --target=node20 \
    --format=cjs \
    --outfile="$BUNDLE_DIR/main.cjs") || return 1

  # esbuild emits `var import_meta = {};` for ESM sources that reference
  # import.meta.url. The Codex SDK calls `createRequire(import.meta.url)` at
  # module scope, which crashes on `undefined`. Inject a valid file URL so
  # module initialization succeeds. (We avoid moduleRequire at runtime by
  # always passing codexPathOverride in main.js.)
  local meta_shim="var import_meta = { url: require('url').pathToFileURL(__filename).href };"
  if [[ "$(uname -s)" == "Darwin"* ]]; then
    sed -i '' "s|var import_meta = {};|$meta_shim|" "$BUNDLE_DIR/main.cjs"
  else
    sed -i "s|var import_meta = {};|$meta_shim|" "$BUNDLE_DIR/main.cjs"
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
  (cd "$SCRIPT_DIR" && npx -y -p @yao-pkg/pkg pkg "$BUNDLE_DIR/main.cjs" \
    --target "$pkg_target" \
    --output "$EMBED_DIR/$binary_name") || return 1

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
