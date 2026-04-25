#!/usr/bin/env bash
# Builds and installs the ObjectiveAI CLI.
#
# - Builds objectiveai-cli in release mode (skips if fingerprint unchanged)
# - Copies the binary to ~/.objectiveai/ as 'objectiveai' (or 'objectiveai.exe' on Windows)
# - Adds ~/.objectiveai to PATH if not already present
#
# Usage:
#   bash objectiveai-cli/install.sh [--no-viewer] [--claude-agent-sdk-javascript]
#
#   --no-viewer                    Build without the embedded Tauri viewer
#   --claude-agent-sdk-javascript  Use the JavaScript Claude Agent SDK runner
#                                  (default: Python runner)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="$HOME/.objectiveai"

# Parse args
NO_VIEWER=0
CLAUDE_AGENT_SDK_JAVASCRIPT=0
for arg in "$@"; do
  case "$arg" in
    --no-viewer) NO_VIEWER=1 ;;
    --claude-agent-sdk-javascript) CLAUDE_AGENT_SDK_JAVASCRIPT=1 ;;
  esac
done

# Detect platform
case "$(uname -s)" in
  CYGWIN*|MINGW*|MSYS*) PLATFORM="windows" ;;
  Darwin*)              PLATFORM="macos"   ;;
  *)                    PLATFORM="linux"   ;;
esac

if [ "$PLATFORM" = "windows" ]; then
  SRC_NAME="objectiveai-cli.exe"
  DST_NAME="objectiveai.exe"
else
  SRC_NAME="objectiveai-cli"
  DST_NAME="objectiveai"
fi

# ── Fingerprint ────────────────────────────────────────────────────────
# Hash all source files that affect the CLI build. Skip the build if the
# installed binary's fingerprint matches.

FINGERPRINT_FILE="$INSTALL_DIR/.fingerprint"

# Cross-platform SHA-256 wrapper. macOS runners ship `shasum` (Perl) but
# not GNU `sha256sum`; Linux/Windows-Git-bash typically ship both — we
# prefer `sha256sum` when it exists so output format stays identical
# across the common case, and fall back to `shasum -a 256` otherwise.
if command -v sha256sum >/dev/null 2>&1; then
  _sha256() { sha256sum "$@"; }
else
  _sha256() { shasum -a 256 "$@"; }
fi

compute_fingerprint() {
  {
    # Bake build flags into fingerprint so variants don't collide
    echo "NO_VIEWER=$NO_VIEWER"
    echo "CLAUDE_AGENT_SDK_JAVASCRIPT=$CLAUDE_AGENT_SDK_JAVASCRIPT"

    # objectiveai-cli sources
    find "$SCRIPT_DIR/src" -type f -name '*.rs' | sort
    echo "$SCRIPT_DIR/Cargo.toml"

    # objectiveai-rs (core SDK)
    find "$REPO_ROOT/objectiveai-rs/src" -type f -name '*.rs' | sort
    echo "$REPO_ROOT/objectiveai-rs/Cargo.toml"

    # objectiveai-api (library dep)
    find "$REPO_ROOT/objectiveai-api/src" -type f -name '*.rs' | sort
    echo "$REPO_ROOT/objectiveai-api/Cargo.toml"

    # Claude Agent SDK runner (one of the two, based on flag)
    if [ "$CLAUDE_AGENT_SDK_JAVASCRIPT" = "1" ]; then
      echo "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-js/main.js"
      echo "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-js/package.json"
    else
      echo "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-py/main.py"
      echo "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-py/requirements.txt"
    fi

    # objectiveai-viewer (embedded binary, unless --no-viewer)
    if [ "$NO_VIEWER" = "0" ]; then
      find "$REPO_ROOT/objectiveai-viewer/src-tauri/src" -type f -name '*.rs' | sort
      echo "$REPO_ROOT/objectiveai-viewer/src-tauri/Cargo.toml"
      echo "$REPO_ROOT/objectiveai-viewer/src-tauri/tauri.conf.json"
      find "$REPO_ROOT/objectiveai-viewer/dist" -type f 2>/dev/null | sort
    fi

    # Shared lockfile
    echo "$REPO_ROOT/Cargo.lock"
  } | while IFS= read -r file; do
    if [ -f "$file" ]; then
      relpath="${file#"$REPO_ROOT/"}"
      printf '%s\n' "$relpath"
      # Strip the path from the hash line — sha256sum's default output
      # `<hash>  <path>` would otherwise embed the runner's absolute path
      # (different on Linux, macOS, Windows) and break cross-runner
      # fingerprint matching.
      _sha256 "$file" | awk '{print $1}'
    else
      printf '%s\n' "$file"
    fi
  done | _sha256 | awk '{print $1}'
}

CURRENT_FP=$(compute_fingerprint)

if [ -f "$FINGERPRINT_FILE" ]; then
  STORED_FP=$(cat "$FINGERPRINT_FILE")
  if [ "$CURRENT_FP" = "$STORED_FP" ] && [ -f "$INSTALL_DIR/$DST_NAME" ]; then
    echo "objectiveai is up to date (fingerprint: ${CURRENT_FP:0:12}...)"
    exit 0
  fi
fi

# ── Build embedded binaries ────────────────────────────────────────────
# The CLI embeds viewer (via build.rs), and objectiveai-api embeds
# mcp (linux-musl) and one of the two claude-agent-sdk-runners.

echo "Building embedded dependencies..."

# claude-agent-sdk-runner (native target) — build the one we're using
if [ "$CLAUDE_AGENT_SDK_JAVASCRIPT" = "1" ]; then
  bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-js/build.sh" --release
else
  bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-py/build.sh" --release
fi

# mcp (linux-musl, needed by objectiveai-api with orchestrator-bollard).
# Match the host architecture (ARM hosts embed aarch64, x86_64 hosts embed
# x86_64), but always target linux-musl. Normalize macOS's `arm64` to Rust's
# `aarch64` triple convention.
MCP_ARCH=$(uname -m)
case "$MCP_ARCH" in
  arm64) MCP_ARCH=aarch64 ;;
esac
bash "$REPO_ROOT/objectiveai-mcp/build.sh" --target "$MCP_ARCH-unknown-linux-musl" --release

# viewer (native target, unless --no-viewer)
if [ "$NO_VIEWER" = "0" ]; then
  bash "$REPO_ROOT/objectiveai-viewer/build.sh" --release
fi

# ── Build CLI ──────────────────────────────────────────────────────────

# Assemble feature list
FEATURES="rustpython,systempython,updater"
if [ "$CLAUDE_AGENT_SDK_JAVASCRIPT" = "1" ]; then
  FEATURES="$FEATURES,claude-agent-sdk-javascript"
else
  FEATURES="$FEATURES,claude-agent-sdk-python"
fi
if [ "$NO_VIEWER" = "0" ]; then
  FEATURES="$FEATURES,viewer"
fi

echo "Building objectiveai-cli (release, features: $FEATURES)..."
cargo build --release -p objectiveai-cli --no-default-features \
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
echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
echo "Installed $INSTALL_DIR/$DST_NAME"

# ── PATH ───────────────────────────────────────────────────────────────

add_to_path() {
  local shell_rc="$1"
  local line="export PATH=\"\$HOME/.objectiveai:\$PATH\""
  if [ -f "$shell_rc" ] && grep -qF '.objectiveai' "$shell_rc"; then
    return
  fi
  echo "" >> "$shell_rc"
  echo "# ObjectiveAI CLI" >> "$shell_rc"
  echo "$line" >> "$shell_rc"
  echo "Added to PATH in $shell_rc"
}

case "$PLATFORM" in
  windows)
    INSTALL_DIR_WIN="$(cygpath -w "$INSTALL_DIR")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r')
    if echo "$CURRENT_PATH" | grep -qiF '.objectiveai'; then
      echo "PATH already contains $INSTALL_DIR_WIN"
    else
      powershell.exe -NoProfile -Command \
        "[Environment]::SetEnvironmentVariable('Path', '$INSTALL_DIR_WIN;' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')" 2>/dev/null
      echo "Added $INSTALL_DIR_WIN to user PATH (restart your terminal to take effect)"
    fi
    ;;
  macos)
    add_to_path "$HOME/.zshrc"
    ;;
  linux)
    if [ -f "$HOME/.bashrc" ]; then
      add_to_path "$HOME/.bashrc"
    fi
    if [ -f "$HOME/.zshrc" ]; then
      add_to_path "$HOME/.zshrc"
    fi
    ;;
esac

echo ""
echo "Done! Run 'objectiveai --help' to get started."
echo "You may need to restart your terminal for PATH changes to take effect."
