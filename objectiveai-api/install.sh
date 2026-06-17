#!/usr/bin/env bash
# Builds and installs a standalone objectiveai-api server binary:
# cargo-build objectiveai-api in release mode, then copy the binary to
# ~/.objectiveai/bin/.
#
# The api embeds nothing of its own — it spawns the claude-agent-sdk /
# codex-sdk runners at runtime from <OBJECTIVEAI_DIR>/bin/, which are
# built and shipped as their own release artifacts.
#
# Usage:
#   bash objectiveai-api/install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="$HOME/.objectiveai"

# Detect platform
case "$(uname -s)" in
  CYGWIN*|MINGW*|MSYS*) PLATFORM="windows" ;;
  Darwin*)              PLATFORM="macos"   ;;
  *)                    PLATFORM="linux"   ;;
esac

if [ "$PLATFORM" = "windows" ]; then
  SRC_NAME="objectiveai-api.exe"
  DST_NAME="objectiveai-api.exe"
else
  SRC_NAME="objectiveai-api"
  DST_NAME="objectiveai-api"
fi

# ── Build api ──────────────────────────────────────────────────────────

echo "Building objectiveai-api (release)..."
cargo build --release -p objectiveai-api \
  --manifest-path "$REPO_ROOT/Cargo.toml"

SRC="$REPO_ROOT/target/release/$SRC_NAME"
if [ ! -f "$SRC" ]; then
  echo "ERROR: expected binary at $SRC" >&2
  exit 1
fi

# ── Install ────────────────────────────────────────────────────────────
# api/viewer/mcp land in <base>/bin/ — the cli manages them from there.
# (The cli itself stays at <base>/objectiveai{.exe}.)

BIN_DIR="$INSTALL_DIR/bin"
mkdir -p "$BIN_DIR"
cp "$SRC" "$BIN_DIR/$DST_NAME"
chmod +x "$BIN_DIR/$DST_NAME"
echo "Installed $BIN_DIR/$DST_NAME"
