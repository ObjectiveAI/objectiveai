#!/usr/bin/env bash
# ObjectiveAI CLI installer — downloads a pre-built release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash -s -- --no-viewer
#
# - Detects platform + architecture.
# - Fetches the latest published release asset from GitHub and drops it
#   at ~/.objectiveai/objectiveai (or objectiveai.exe on Windows).
# - Adds ~/.objectiveai to PATH.
#
# No toolchain required. For a from-source install, clone the repo and
# run `objectiveai-cli/install.sh` instead.

set -euo pipefail

REPO="ObjectiveAI/objectiveai"
INSTALL_DIR="$HOME/.objectiveai"
NO_VIEWER=0

for arg in "$@"; do
  case "$arg" in
    --no-viewer) NO_VIEWER=1 ;;
    -h|--help)
      sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'
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

# ── Asset name + destination ──────────────────────────────────────────

# Must match the `ASSET_NAME` table in objectiveai-cli/src/update.rs.
ASSET="objectiveai-${PLATFORM}-${ARCH}"
if [ "$NO_VIEWER" = "1" ]; then
  ASSET="${ASSET}-no-viewer"
fi
if [ "$PLATFORM" = "windows" ]; then
  ASSET="${ASSET}.exe"
  DST_NAME="objectiveai.exe"
else
  DST_NAME="objectiveai"
fi

URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
TMP=$(mktemp -t objectiveai.XXXXXX)
trap 'rm -f "$TMP"' EXIT

# ── Download ──────────────────────────────────────────────────────────

echo "Downloading $ASSET..."
if command -v curl >/dev/null 2>&1; then
  # -L follows the redirect from /releases/latest/download/... to the
  # actual asset URL; -f fails hard on 4xx/5xx instead of writing HTML.
  curl -fSL --progress-bar "$URL" -o "$TMP"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$TMP" "$URL"
else
  echo "need curl or wget to download" >&2
  exit 1
fi

if [ ! -s "$TMP" ]; then
  echo "download produced an empty file" >&2
  exit 1
fi

# ── Install ───────────────────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
DST="$INSTALL_DIR/$DST_NAME"
# `mv` onto a running Windows exe fails ("in use"); prefer `cp` so a
# later install over an in-use binary degrades to a clearer error.
cp "$TMP" "$DST"
chmod +x "$DST"
echo "Installed $DST"

# ── PATH ──────────────────────────────────────────────────────────────

add_to_path() {
  local shell_rc="$1"
  local line='export PATH="$HOME/.objectiveai:$PATH"'
  if [ -f "$shell_rc" ] && grep -qF '.objectiveai' "$shell_rc"; then
    return
  fi
  {
    echo ""
    echo "# ObjectiveAI CLI"
    echo "$line"
  } >> "$shell_rc"
  echo "Added to PATH in $shell_rc"
}

case "$PLATFORM" in
  windows)
    INSTALL_DIR_WIN="$(cygpath -w "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r' || true)
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
    [ -f "$HOME/.bashrc" ] && add_to_path "$HOME/.bashrc"
    [ -f "$HOME/.zshrc" ]  && add_to_path "$HOME/.zshrc"
    ;;
esac

echo ""
echo "Done! Run 'objectiveai --help' to get started."
echo "You may need to restart your terminal for PATH changes to take effect."
