#!/usr/bin/env bash
# ObjectiveAI installer — downloads the pre-built release binaries.
#
# Installs `objectiveai` (CLI), `objectiveai-api` (server),
# `objectiveai-viewer` (Tauri desktop app), and `objectiveai-mcp`
# (MCP server) from the latest GitHub Release. Takes no arguments.
#
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
#
# Layout on disk (bin/ is machine-wide; per-state data lives under
# ~/.objectiveai/state/<OBJECTIVEAI_STATE>, default "default"):
#   ~/.objectiveai/bin/objectiveai{.exe}        ← CLI
#   ~/.objectiveai/bin/objectiveai-api{.exe}
#   ~/.objectiveai/bin/objectiveai-viewer{.exe}
#   ~/.objectiveai/bin/objectiveai-mcp{.exe}
#
# ~/.objectiveai/bin is added to PATH.
# No toolchain required.
#
# For a from-source install, clone the repo and run the per-crate
# install.sh scripts under objectiveai-cli/, objectiveai-api/,
# objectiveai-viewer/, objectiveai-mcp/.

set -euo pipefail

REPO="ObjectiveAI/objectiveai"
INSTALL_DIR="$HOME/.objectiveai"

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

# install_binary <asset_filename> <dst_dir> <dst_filename>
#
# Fetches the asset from /releases/latest/download/ and installs it at
# <dst_dir>/<dst_filename> with the executable bit set.
install_binary() {
  local asset="$1" dst_dir="$2" dst_name="$3"
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

  mkdir -p "$dst_dir"
  dst="$dst_dir/$dst_name"
  # `mv` onto a running Windows exe fails ("in use"); prefer `cp` so a
  # later install over an in-use binary degrades to a clearer error.
  cp "$tmp" "$dst"
  chmod +x "$dst"
  echo "Installed $dst"
}

# ── Install binaries ──────────────────────────────────────────────────
# Every binary lands in bin/ — machine-wide, shared by every state —
# so the cli's own `objectiveai update` has one stable place to
# refresh them all.

BIN_DIR="$INSTALL_DIR/bin"

# CLI — always installed.
install_binary \
  "objectiveai-${PLATFORM}-${ARCH}${EXE_SUFFIX}" \
  "$BIN_DIR" \
  "objectiveai${EXE_SUFFIX}"

# API server — standalone objectiveai-api binary.
install_binary \
  "objectiveai-${PLATFORM}-${ARCH}-api${EXE_SUFFIX}" \
  "$BIN_DIR" \
  "objectiveai-api${EXE_SUFFIX}"

# Viewer — standalone Tauri desktop app.
install_binary \
  "objectiveai-${PLATFORM}-${ARCH}-viewer${EXE_SUFFIX}" \
  "$BIN_DIR" \
  "objectiveai-viewer${EXE_SUFFIX}"

# MCP — standalone MCP (Model Context Protocol) server.
install_binary \
  "objectiveai-${PLATFORM}-${ARCH}-mcp${EXE_SUFFIX}" \
  "$BIN_DIR" \
  "objectiveai-mcp${EXE_SUFFIX}"

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
    *:"$HOME/.objectiveai/bin":*) ;;
    *) export PATH="$HOME/.objectiveai/bin:$PATH" ;;
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
    BIN_DIR_WIN="$(cygpath -w "$BIN_DIR" 2>/dev/null || echo "$BIN_DIR")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r' || true)
    NEED_PREPEND=""
    if ! echo "$CURRENT_PATH" | grep -qiF "$INSTALL_DIR_WIN"; then
      NEED_PREPEND="$INSTALL_DIR_WIN;"
    fi
    if ! echo "$CURRENT_PATH" | grep -qiF "$BIN_DIR_WIN"; then
      NEED_PREPEND="$NEED_PREPEND$BIN_DIR_WIN;"
    fi
    if [ -n "$NEED_PREPEND" ]; then
      powershell.exe -NoProfile -Command \
        "[Environment]::SetEnvironmentVariable('Path', '$NEED_PREPEND' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')" 2>/dev/null
      echo "Added $NEED_PREPEND to user PATH (restart cmd/PowerShell to use it)."
    else
      echo "PATH already contains $INSTALL_DIR_WIN and $BIN_DIR_WIN"
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
