#!/usr/bin/env bash
# Builds objectiveai-sdk-rs-cffi to dist/ (WASM + native for host platform).
# Skips the build if the source fingerprint hasn't changed (SHA-based, like cargo).
# Output is captured to .logs/build/objectiveai-sdk-rs-cffi.txt.
#
# Usage:
#   bash objectiveai-sdk-rs-cffi/build.sh

set -euo pipefail

MODULE="objectiveai-sdk-rs-cffi"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

run() {
  # Check fingerprint — returns 1 if dist/ is up to date (not an error).
  if ! source "$SCRIPT_DIR/fingerprint.sh"; then
    return 0
  fi

  # --- WASM build (for Go SDK) ---
  TARGET="wasm32-wasip1"
  echo "Building cffi (wasm32-wasip1, release)..."
  if ! cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" --target "$TARGET" --release; then
    return 1
  fi

  mkdir -p "$SCRIPT_DIR/dist"
  if ! cp "$REPO_ROOT/target/$TARGET/release/objectiveai_cffi.wasm" "$SCRIPT_DIR/dist/"; then
    return 1
  fi

  # --- Native build (for .NET SDK) ---
  # Detect host target and RID
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)       NATIVE_TARGET="x86_64-unknown-linux-gnu";  RID="linux-x64";  LIB="libobjectiveai_cffi.so" ;;
    Linux-aarch64)       NATIVE_TARGET="aarch64-unknown-linux-gnu"; RID="linux-arm64"; LIB="libobjectiveai_cffi.so" ;;
    Darwin-x86_64)       NATIVE_TARGET="x86_64-apple-darwin";      RID="osx-x64";    LIB="libobjectiveai_cffi.dylib" ;;
    Darwin-arm64)        NATIVE_TARGET="aarch64-apple-darwin";      RID="osx-arm64";  LIB="libobjectiveai_cffi.dylib" ;;
    MINGW*-x86_64|MSYS*-x86_64|*_NT*-x86_64) NATIVE_TARGET="x86_64-pc-windows-msvc"; RID="win-x64"; LIB="objectiveai_cffi.dll" ;;
    MINGW*-aarch64|MSYS*-aarch64|*_NT*-aarch64) NATIVE_TARGET="aarch64-pc-windows-msvc"; RID="win-arm64"; LIB="objectiveai_cffi.dll" ;;
    *) echo "Unsupported host for native build: $(uname -s)-$(uname -m)"; return 1 ;;
  esac

  echo "Building cffi ($NATIVE_TARGET, release)..."
  if ! cargo build --manifest-path "$SCRIPT_DIR/Cargo.toml" --target "$NATIVE_TARGET" --release; then
    return 1
  fi

  local native_dir="$SCRIPT_DIR/dist/runtimes/$RID/native"
  mkdir -p "$native_dir"
  if ! cp "$REPO_ROOT/target/$NATIVE_TARGET/release/$LIB" "$native_dir/"; then
    return 1
  fi
  echo "Installed native $LIB -> dist/runtimes/$RID/native/"

  # Stamp the fingerprint only after successful build + copy
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "Build complete (fingerprint: ${CURRENT_FP:0:12}...)"
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
