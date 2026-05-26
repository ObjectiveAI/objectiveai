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
#   --no-viewer        skip the standalone viewer binary.
#   --no-api           skip the standalone API server binary.
#   --no-mcp           skip the standalone MCP server binary.
#   --cli-only         skip viewer, api, and mcp (only install the CLI).
#   --dev,             delegate to
#   --development      objectiveai-development-launcher/install.sh, which
#                      installs launchers that shell out to
#                      `cargo run -p <pkg>` against the local clone.
#                      Requires running this script from a clone (not
#                      via `curl | bash`). --no-*/--cli-only flags are
#                      ignored when --dev is set.
#
# Layout on disk:
#   ~/.objectiveai/objectiveai{.exe}        ← CLI (managed self)
#   ~/.objectiveai/bin/objectiveai-api{.exe}
#   ~/.objectiveai/bin/objectiveai-viewer{.exe}
#   ~/.objectiveai/bin/objectiveai-mcp{.exe}
#
# Both ~/.objectiveai and ~/.objectiveai/bin are added to PATH.
# No toolchain required.
#
# For a from-source install, clone the repo and run the per-crate
# install.sh scripts under objectiveai-cli/, objectiveai-api/,
# objectiveai-viewer/, objectiveai-mcp/.

set -euo pipefail

REPO="ObjectiveAI/objectiveai"
INSTALL_DIR="$HOME/.objectiveai"

INSTALL_API=1
INSTALL_VIEWER=1
INSTALL_MCP=1
DEV=0

for arg in "$@"; do
  case "$arg" in
    --no-viewer)
      INSTALL_VIEWER=0
      ;;
    --no-api)
      INSTALL_API=0
      ;;
    --no-mcp)
      INSTALL_MCP=0
      ;;
    --cli-only)
      INSTALL_API=0
      INSTALL_VIEWER=0
      INSTALL_MCP=0
      ;;
    --dev|--development)
      DEV=1
      ;;
    -h|--help)
      sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown option: $arg" >&2
      exit 2
      ;;
  esac
done

# ── --dev: delegate to the development launcher installer ────────────
# Requires being run from a clone of the repo (BASH_SOURCE must be a
# regular file). curl|bash invocations have no clone to point at and
# error out.
if [ "$DEV" = "1" ]; then
  SCRIPT_PATH="${BASH_SOURCE[0]:-}"
  if [ -z "$SCRIPT_PATH" ] || [ ! -f "$SCRIPT_PATH" ]; then
    cat >&2 <<'MSG'
--dev requires running install.sh from a checkout of the repo.
This invocation looks piped via curl. Clone the repo first:
  git clone https://github.com/ObjectiveAI/objectiveai
  bash objectiveai/install.sh --dev
MSG
    exit 1
  fi
  REPO_ROOT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
  LAUNCHER_INSTALL="$REPO_ROOT_DIR/objectiveai-development-launcher/install.sh"
  if [ ! -f "$LAUNCHER_INSTALL" ]; then
    echo "ERROR: $LAUNCHER_INSTALL missing — is this an objectiveai clone?" >&2
    exit 1
  fi
  echo "--dev: delegating to objectiveai-development-launcher/install.sh"
  exec bash "$LAUNCHER_INSTALL"
fi

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
# CLI sits at the base directory; api/viewer/mcp land in bin/ so the
# cli's own `objectiveai update` has a stable place to refresh them.

BIN_DIR="$INSTALL_DIR/bin"

# CLI — always installed.
install_binary \
  "objectiveai-${PLATFORM}-${ARCH}${EXE_SUFFIX}" \
  "$INSTALL_DIR" \
  "objectiveai${EXE_SUFFIX}"

# API server — standalone objectiveai-api binary.
if [ "$INSTALL_API" = "1" ]; then
  install_binary \
    "objectiveai-${PLATFORM}-${ARCH}-api${EXE_SUFFIX}" \
    "$BIN_DIR" \
    "objectiveai-api${EXE_SUFFIX}"
fi

# Viewer — standalone Tauri desktop app.
if [ "$INSTALL_VIEWER" = "1" ]; then
  install_binary \
    "objectiveai-${PLATFORM}-${ARCH}-viewer${EXE_SUFFIX}" \
    "$BIN_DIR" \
    "objectiveai-viewer${EXE_SUFFIX}"
fi

# MCP — standalone MCP (Model Context Protocol) server.
if [ "$INSTALL_MCP" = "1" ]; then
  install_binary \
    "objectiveai-${PLATFORM}-${ARCH}-mcp${EXE_SUFFIX}" \
    "$BIN_DIR" \
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
