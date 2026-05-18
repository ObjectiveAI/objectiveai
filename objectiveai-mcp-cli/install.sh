#!/usr/bin/env bash
# Builds and installs a standalone, fully-featured objectiveai-mcp
# binary (the MCP (Model Context Protocol) server wrapper around the
# cli — published from the objectiveai-mcp-cli crate, but the binary
# itself is named `objectiveai-mcp`).
#
# Mirrors objectiveai-cli/install.sh's structure: build the per-host
# embedded dependencies first (claude-agent-sdk-runner, codex-sdk-
# runner, mcp-filesystem, viewer embed), then cargo-build
# objectiveai-mcp-cli in release mode with all passthrough features
# turned on, then copy the binary to ~/.objectiveai/.
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

# ── Build embedded binaries ────────────────────────────────────────────
# objectiveai-mcp-cli depends on objectiveai-cli, which embeds the
# viewer (via build.rs) and depends on objectiveai-api (which embeds
# claude-agent-sdk-runner + codex-sdk-runner + linux-musl
# mcp-filesystem). Same per-host artifact chain the cli release uses.

echo "Building embedded dependencies..."

# claude-agent-sdk-runner (native target, Python)
bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" --release

# codex-sdk-runner (native target, Python)
bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" --release

# mcp-filesystem (linux-musl, Docker container injection) — embedded by
# objectiveai-api with orchestrator-bollard. Match the host
# architecture (ARM hosts embed aarch64, x86_64 hosts embed x86_64) and
# always target linux-musl. Normalize macOS's `arm64` to Rust's
# `aarch64` triple.
MCP_ARCH=$(uname -m)
case "$MCP_ARCH" in
  arm64) MCP_ARCH=aarch64 ;;
esac
bash "$REPO_ROOT/objectiveai-mcp-filesystem/build.sh" --target "$MCP_ARCH-unknown-linux-musl" --release

# viewer (native target) — required because the `viewer` passthrough
# feature pulls in the cli's viewer feature which expects an embedded
# viewer binary.
bash "$REPO_ROOT/objectiveai-viewer/build.sh" --release

# ── Build mcp ──────────────────────────────────────────────────────────

# Fully-featured: all three passthrough features on for parity with
# the cli release, plus the updater so the shipped binary self-updates.
FEATURES="viewer,rustpython,systempython,updater"

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

mkdir -p "$INSTALL_DIR"
cp "$SRC" "$INSTALL_DIR/$DST_NAME"
chmod +x "$INSTALL_DIR/$DST_NAME"
echo "Installed $INSTALL_DIR/$DST_NAME"
