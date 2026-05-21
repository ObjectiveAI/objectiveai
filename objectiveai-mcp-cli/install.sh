#!/usr/bin/env bash
# Builds and installs a standalone objectiveai-mcp binary (the MCP
# (Model Context Protocol) server wrapper around the cli — published
# from the objectiveai-mcp-cli crate, but the binary itself is named
# `objectiveai-mcp`).
#
# The cli no longer embeds the viewer or api, so the build chain is
# now just `cargo build` with the cli's passthrough features.
#
# Usage:
#   bash objectiveai-mcp-cli/install.sh

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
  SRC_NAME="objectiveai-mcp.exe"
  DST_NAME="objectiveai-mcp.exe"
else
  SRC_NAME="objectiveai-mcp"
  DST_NAME="objectiveai-mcp"
fi

# ── Build mcp ──────────────────────────────────────────────────────────

# Pass through the cli's python features for parity with the cli release.
FEATURES="rustpython,systempython"

echo "Building objectiveai-mcp (release, features: $FEATURES)..."
cargo build --release -p objectiveai-mcp-cli \
  --features "$FEATURES" \
  --manifest-path "$REPO_ROOT/Cargo.toml"

SRC="$REPO_ROOT/target/release/$SRC_NAME"
if [ ! -f "$SRC" ]; then
  echo "ERROR: expected binary at $SRC" >&2
  exit 1
fi

# ── Install ────────────────────────────────────────────────────────────
# api/viewer/mcp land in <base>/bin/; only the cli sits at <base>/.

BIN_DIR="$INSTALL_DIR/bin"
mkdir -p "$BIN_DIR"
cp "$SRC" "$BIN_DIR/$DST_NAME"
chmod +x "$BIN_DIR/$DST_NAME"
echo "Installed $BIN_DIR/$DST_NAME"
