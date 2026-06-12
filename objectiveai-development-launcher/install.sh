#!/usr/bin/env bash
# Installs the development launcher binary as drop-in replacements
# for the five production executables under `~/.objectiveai/`:
#
#   objectiveai{.exe}             -> cargo run -p objectiveai-cli
#   bin/objectiveai-api{.exe}     -> cargo run -p objectiveai-api
#   bin/objectiveai-viewer{.exe}  -> cargo run -p objectiveai-viewer
#   bin/objectiveai-mcp{.exe}     -> cargo run -p objectiveai-mcp
#   bin/objectiveai-db{.exe}      -> cargo run -p objectiveai-db
#
# (The same binary, built WITHOUT the baked repo root, is also
# committed at the repo's `.objectiveai/bin/*.exe` shim paths — see
# the doc comment in src/main.rs for the dual root resolution.)
#
# After running this script, anything that spawns those binaries
# (cli `api spawn` / `viewer spawn`, scripts, the viewer's
# embedded cli invocation, etc.) picks up local source changes
# through a fresh `cargo build` — no per-crate install step
# needed between edit and run.
#
# Overwrites any existing binary at these paths. To get back to the
# real release binaries, rerun the root `install.sh` (without --dev).
#
# Usage:
#   bash objectiveai-development-launcher/install.sh
#
# Or via the root installer's `--dev` flag from a clone:
#   bash install.sh --dev

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

case "$(uname -s)" in
  CYGWIN*|MINGW*|MSYS*) EXT=".exe" ;;
  *)                    EXT=""     ;;
esac

INSTALL_DIR="$HOME/.objectiveai"
BIN_DIR="$INSTALL_DIR/bin"

# Build the launcher with the repo root baked in.
OBJECTIVEAI_REPO_ROOT="$REPO_ROOT" \
  cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

SRC="$SCRIPT_DIR/target/release/objectiveai-development-launcher$EXT"
if [ ! -f "$SRC" ]; then
  echo "ERROR: launcher missing at $SRC" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR" "$BIN_DIR"

# Install one launcher at each of the canonical install paths.
# The launcher dispatches off its own filename, so the same .exe
# byte-for-byte works at all four locations.
#
# Layout matches the root install.sh:
#   cli at $INSTALL_DIR/objectiveai$EXT
#   others at $INSTALL_DIR/bin/objectiveai-<x>$EXT

install_launcher() {
  local name="$1" dst_dir="$2"
  local dst="$dst_dir/$name$EXT"
  cp "$SRC" "$dst"
  chmod +x "$dst" 2>/dev/null || true
  echo "Installed development launcher: $dst (-> cargo run -p <pkg>)"
}

install_launcher "objectiveai"        "$INSTALL_DIR"
install_launcher "objectiveai-api"    "$BIN_DIR"
install_launcher "objectiveai-viewer" "$BIN_DIR"
install_launcher "objectiveai-mcp"    "$BIN_DIR"
install_launcher "objectiveai-db"     "$BIN_DIR"
