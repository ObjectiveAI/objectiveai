#!/usr/bin/env bash
# Builds and installs a standalone, fully-featured objectiveai-viewer
# binary. Single self-contained executable, no installer bundles.
#
# Uses `tauri build --no-bundle` — the same raw-binary path the
# objectiveai-cli's build.rs embed uses. The output binary is
# byte-identical to the one the CLI embeds via include_bytes!; the
# only difference here is staging (release upload at
# ~/.objectiveai/objectiveai-viewer instead of the embed/ tree).
#
# Usage:
#   bash objectiveai-viewer/install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="$HOME/.objectiveai"

# Detect platform + Rust target triple
HOST_ARCH=$(uname -m)
case "$HOST_ARCH" in
  arm64) HOST_ARCH=aarch64 ;;
esac
case "$(uname -s)" in
  CYGWIN*|MINGW*|MSYS*)
    HOST_TARGET="$HOST_ARCH-pc-windows-msvc"
    SRC_NAME="objectiveai-viewer.exe"
    DST_NAME="objectiveai-viewer.exe"
    ;;
  Darwin*)
    HOST_TARGET="$HOST_ARCH-apple-darwin"
    SRC_NAME="objectiveai-viewer"
    DST_NAME="objectiveai-viewer"
    ;;
  *)
    HOST_TARGET="$HOST_ARCH-unknown-linux-gnu"
    SRC_NAME="objectiveai-viewer"
    DST_NAME="objectiveai-viewer"
    ;;
esac

# ── Install JS workspace deps ──────────────────────────────────────────
# tauri-build runs `pnpm run build` (the viewer's beforeBuildCommand)
# which needs node_modules.

echo "Installing JS workspace dependencies..."
(cd "$REPO_ROOT" && pnpm install --frozen-lockfile)

# ── Build viewer binary ────────────────────────────────────────────────
# `tauri build --no-bundle` produces just the raw exe — no .dmg /
# .msi / .AppImage installer wrappers. Matches the CLI embed path
# (objectiveai-viewer/build.sh also uses --no-bundle). cli_run is
# always compiled in; it spawns `~/.objectiveai/objectiveai` at
# runtime, so no cargo feature is needed.

echo "Building objectiveai-viewer (release, target: $HOST_TARGET)..."
(cd "$SCRIPT_DIR/src-tauri" && pnpm exec tauri build --no-bundle --target "$HOST_TARGET")

SRC="$REPO_ROOT/target/$HOST_TARGET/release/$SRC_NAME"
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
