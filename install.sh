#!/usr/bin/env bash
# ObjectiveAI installer — downloads the pre-built release binaries.
#
# Default: installs `objectiveai` (CLI), `objectiveai-api` (server),
# `objectiveai-viewer` (Tauri desktop app), and `objectiveai-mcp`
# (MCP server) from the latest GitHub Release.
#
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
#
# Flags (compose freely):
#   --no-viewer   skip the standalone viewer binary AND use the CLI variant
#                 without an embedded viewer.
#   --no-api      skip the standalone API server binary.
#   --no-mcp      skip the standalone MCP server binary.
#   --cli-only    skip viewer, api, and mcp (only install the CLI).
#
# All binaries land in ~/.objectiveai/ (or ~/.objectiveai/*.exe on Windows)
# and are added to PATH. No toolchain required.
#
# For a from-source install, clone the repo and run the per-crate
# install.sh scripts under objectiveai-cli/, objectiveai-api/,
# objectiveai-viewer/, objectiveai-mcp-cli/.

set -euo pipefail

REPO="ObjectiveAI/objectiveai"
INSTALL_DIR="$HOME/.objectiveai"

NO_VIEWER=0
INSTALL_API=1
INSTALL_VIEWER=1
INSTALL_MCP=1

for arg in "$@"; do
  case "$arg" in
    --no-viewer)
      NO_VIEWER=1
      INSTALL_VIEWER=0
      ;;
    --no-api)
      INSTALL_API=0
      ;;
    --no-mcp)
      INSTALL_MCP=0
      ;;
    --cli-only)
      NO_VIEWER=1
      INSTALL_API=0
      INSTALL_VIEWER=0
      INSTALL_MCP=0
      ;;
    -h|--help)
      sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg" >&2
      exit 2
      ;;
  esac
done

# ── Detect platform ───────────────────────────────────────────────────

case "$(uname -s)" in
  Linux*)               PLATFORM="linux"   ;;
  Darwin*)              PLATFORM="macos"   ;;
  CYGWIN*|MINGW*|MSYS*) PLATFORM="windows" ;;
  *)
    echo "unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64)  ARCH="x86_64"  ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *)
    echo "unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# Only these platform/arch combos have release assets.
SUPPORTED=0
case "$PLATFORM-$ARCH" in
  linux-x86_64|linux-aarch64|macos-x86_64|macos-aarch64|windows-x86_64) SUPPORTED=1 ;;
esac
if [ "$SUPPORTED" = "0" ]; then
  echo "no release asset for $PLATFORM-$ARCH" >&2
  exit 1
fi

if [ "$PLATFORM" = "windows" ]; then
  EXE_SUFFIX=".exe"
else
  EXE_SUFFIX=""
fi

# ── Download helper ───────────────────────────────────────────────────

# install_binary <asset_filename> <dst_filename>
#
# Fetches the asset from /releases/latest/download/ and installs it at
# $INSTALL_DIR/$DST_NAME with the executable bit set.
install_binary() {
  local asset="$1" dst_name="$2"
  local url="https://github.com/${REPO}/releases/latest/download/${asset}"
  local tmp dst
  tmp=$(mktemp -t objectiveai.XXXXXX)
  # shellcheck disable=SC2064
  trap "rm -f '$tmp'" RETURN

  echo "Downloading $asset..."
  if command -v curl >/dev/null 2>&1; then
    # -L follows the redirect from /releases/latest/download/ to the
    # actual asset URL; -f fails hard on 4xx/5xx instead of writing HTML.
    curl -fSL --progress-bar "$url" -o "$tmp"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$tmp" "$url"
  else
    echo "need curl or wget to download" >&2
    return 1
  fi

  if [ ! -s "$tmp" ]; then
    echo "download produced an empty file" >&2
    return 1
  fi

  mkdir -p "$INSTALL_DIR"
  dst="$INSTALL_DIR/$dst_name"
  # `mv` onto a running Windows exe fails ("in use"); prefer `cp` so a
  # later install over an in-use binary degrades to a clearer error.
  cp "$tmp" "$dst"
  chmod +x "$dst"
  echo "Installed $dst"
}

# ── Install binaries ──────────────────────────────────────────────────

# CLI — always installed. The `-no-viewer` variant is a smaller build
# that strips the embedded Tauri viewer.
CLI_ASSET="objectiveai-${PLATFORM}-${ARCH}"
if [ "$NO_VIEWER" = "1" ]; then
  CLI_ASSET="${CLI_ASSET}-no-viewer"
fi
CLI_ASSET="${CLI_ASSET}${EXE_SUFFIX}"
install_binary "$CLI_ASSET" "objectiveai${EXE_SUFFIX}"

# API server — standalone objectiveai-api binary.
if [ "$INSTALL_API" = "1" ]; then
  install_binary \
    "objectiveai-api-${PLATFORM}-${ARCH}${EXE_SUFFIX}" \
    "objectiveai-api${EXE_SUFFIX}"
fi

# Viewer — standalone Tauri desktop app.
if [ "$INSTALL_VIEWER" = "1" ]; then
  install_binary \
    "objectiveai-viewer-${PLATFORM}-${ARCH}${EXE_SUFFIX}" \
    "objectiveai-viewer${EXE_SUFFIX}"
fi

# MCP — standalone MCP (Model Context Protocol) server.
if [ "$INSTALL_MCP" = "1" ]; then
  install_binary \
    "objectiveai-mcp-${PLATFORM}-${ARCH}${EXE_SUFFIX}" \
    "objectiveai-mcp${EXE_SUFFIX}"
fi

# ── PATH ──────────────────────────────────────────────────────────────
#
# A child process can't mutate its parent shell's environment, so the
# canonical pattern (rustup, etc.) is to write a sourceable env file.
# Future shells pick it up via a one-liner appended to the user's rc;
# the current shell sources it on demand.

write_env_file() {
  cat > "$INSTALL_DIR/env" <<'EOF'
#!/bin/sh
# objectiveai shell setup. Source this file from your shell rc, or run
#   . "$HOME/.objectiveai/env"
# to put the objectiveai binaries on PATH for the current shell.

case ":${PATH}:" in
    *:"$HOME/.objectiveai":*) ;;
    *) export PATH="$HOME/.objectiveai:$PATH" ;;
esac
EOF
}

add_to_path() {
  local shell_rc="$1"
  local line='. "$HOME/.objectiveai/env"'
  if [ -f "$shell_rc" ] && grep -qF '.objectiveai/env' "$shell_rc"; then
    return
  fi
  {
    echo ""
    echo "# ObjectiveAI"
    echo "$line"
  } >> "$shell_rc"
  echo "Added to PATH in $shell_rc"
}

write_env_file

case "$PLATFORM" in
  windows)
    INSTALL_DIR_WIN="$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r' || true)
    if echo "$CURRENT_PATH" | grep -qiF '.objectiveai'; then
      echo "PATH already contains $INSTALL_DIR_WIN"
    else
      powershell.exe -NoProfile -Command \
        "[Environment]::SetEnvironmentVariable('Path', '$INSTALL_DIR_WIN;' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')" 2>/dev/null
      echo "Added $INSTALL_DIR_WIN to user PATH (restart cmd/PowerShell to use it)."
    fi
    # Also wire up Git Bash / MSYS via the env file.
    [ -f "$HOME/.bashrc" ] && add_to_path "$HOME/.bashrc"
    ;;
  macos)
    add_to_path "$HOME/.zshrc"
    ;;
  linux)
    [ -f "$HOME/.bashrc" ] && add_to_path "$HOME/.bashrc"
    [ -f "$HOME/.zshrc" ]  && add_to_path "$HOME/.zshrc"
    ;;
esac

echo ""
echo "Done!"
echo ""
echo "To use the objectiveai binaries in your current shell, run:"
echo '  . "$HOME/.objectiveai/env"'
echo ""
echo "(New shells will pick it up automatically.)"
