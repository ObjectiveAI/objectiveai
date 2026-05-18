#!/usr/bin/env bash
# Builds and installs a standalone objectiveai-api server binary.
#
# Mirrors objectiveai-cli/install.sh's structure: build the per-host
# embedded dependencies first (claude-agent-sdk-runner, codex-sdk-runner,
# mcp-filesystem), then cargo-build objectiveai-api in release mode with
# all features turned on, then copy the binary to ~/.objectiveai/.
#
# Usage:
#   bash objectiveai-api/install.sh
#
# Default feature set: orchestrator-bollard (default) + sqlite-persistent-cache.
# That's the "fully featured" api server — Docker-orchestrator-enabled +
# persistent SQLite cache.

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

# ── Build embedded binaries ────────────────────────────────────────────
# objectiveai-api's build.rs always reaches into the two sibling sdk-runner
# crates and bakes their PyInstaller binaries via include_bytes!, and embeds
# the linux-musl mcp-filesystem under the orchestrator-bollard feature.

echo "Building embedded dependencies..."

# claude-agent-sdk-runner (native target, Python)
bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" --release

# codex-sdk-runner (native target, Python)
bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" --release

# mcp-filesystem (linux-musl, Docker container injection) — embedded by
# objectiveai-api with orchestrator-bollard. Match the host architecture
# (ARM hosts embed aarch64, x86_64 hosts embed x86_64) and always target
# linux-musl. Normalize macOS's `arm64` to Rust's `aarch64` triple.
MCP_ARCH=$(uname -m)
case "$MCP_ARCH" in
  arm64) MCP_ARCH=aarch64 ;;
esac
bash "$REPO_ROOT/objectiveai-mcp-filesystem/build.sh" --target "$MCP_ARCH-unknown-linux-musl" --release

# ── Build api ──────────────────────────────────────────────────────────

# Fully-featured: orchestrator-bollard + updater are on by default;
# explicitly opt into sqlite-persistent-cache so the shipped binary
# supports both in-memory and SQLite-backed caches. Explicit `updater`
# beats relying on defaults so this script can't accidentally ship a
# non-self-updating binary if defaults are ever pruned.
FEATURES="sqlite-persistent-cache,updater"

echo "Building objectiveai-api (release, features: default + $FEATURES)..."
cargo build --release -p objectiveai-api \
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
