#!/usr/bin/env bash
# Builds and installs the standalone objectiveai-db binary — the
# ObjectiveAI database server (embedded-postgres vehicle).
#
# Usage:
#   bash objectiveai-db/install.sh

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
  SRC_NAME="objectiveai-db.exe"
  DST_NAME="objectiveai-db.exe"
else
  SRC_NAME="objectiveai-db"
  DST_NAME="objectiveai-db"
fi

# ── Build db ────────────────────────────────────────────────────────────

# Default features only. The bundled postgres archive (~163M) is baked
# into the binary by postgresql_embedded's build script — set
# GITHUB_TOKEN in CI to dodge its GitHub API rate limits.
echo "Building objectiveai-db (release)..."
cargo build --release -p objectiveai-db \
  --manifest-path "$REPO_ROOT/Cargo.toml"

SRC="$REPO_ROOT/target/release/$SRC_NAME"
if [ ! -f "$SRC" ]; then
  echo "ERROR: expected binary at $SRC" >&2
  exit 1
fi

# ── Install ────────────────────────────────────────────────────────────
# Every binary lands in <dir>/bin/ (machine-wide, shared by states).

BIN_DIR="$INSTALL_DIR/bin"
mkdir -p "$BIN_DIR"
cp "$SRC" "$BIN_DIR/$DST_NAME"
chmod +x "$BIN_DIR/$DST_NAME"
echo "Installed $BIN_DIR/$DST_NAME"
