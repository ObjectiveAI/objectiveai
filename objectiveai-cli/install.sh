#!/usr/bin/env bash
# Builds and installs the ObjectiveAI CLI.
#
# - Builds objectiveai-cli in release mode (skips if fingerprint unchanged)
# - Copies the binary to ~/.objectiveai/bin/ as 'objectiveai' (or 'objectiveai.exe' on Windows)
# - Adds ~/.objectiveai/bin to PATH if not already present
#
# Usage:
#   bash objectiveai-cli/install.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_DIR="$HOME/.objectiveai"
BIN_DIR="$INSTALL_DIR/bin"

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
    # objectiveai-cli sources
    find "$SCRIPT_DIR/src" -type f -name '*.rs' | sort
    echo "$SCRIPT_DIR/Cargo.toml"

    # objectiveai-sdk-rs (core SDK)
    find "$REPO_ROOT/objectiveai-sdk-rs/src" -type f -name '*.rs' | sort
    echo "$REPO_ROOT/objectiveai-sdk-rs/Cargo.toml"

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
  if [ "$CURRENT_FP" = "$STORED_FP" ] && [ -f "$BIN_DIR/$DST_NAME" ]; then
    echo "objectiveai is up to date (fingerprint: ${CURRENT_FP:0:12}...)"
    exit 0
  fi
fi

# ── Build CLI ──────────────────────────────────────────────────────────

echo "Building objectiveai-cli (release)..."
cargo build --release -p objectiveai-cli \
  --manifest-path "$REPO_ROOT/Cargo.toml"

SRC="$REPO_ROOT/target/release/$SRC_NAME"
if [ ! -f "$SRC" ]; then
  echo "ERROR: expected binary at $SRC" >&2
  exit 1
fi

# ── Install ────────────────────────────────────────────────────────────

mkdir -p "$BIN_DIR"
cp "$SRC" "$BIN_DIR/$DST_NAME"
chmod +x "$BIN_DIR/$DST_NAME"
echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
echo "Installed $BIN_DIR/$DST_NAME"

# ── PATH ───────────────────────────────────────────────────────────────
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
  echo "" >> "$shell_rc"
  echo "# ObjectiveAI CLI" >> "$shell_rc"
  echo "$line" >> "$shell_rc"
  echo "Added to PATH in $shell_rc"
}

write_env_file

case "$PLATFORM" in
  windows)
    INSTALL_DIR_WIN="$(cygpath -w "$INSTALL_DIR")"
    BIN_DIR_WIN="$(cygpath -w "$INSTALL_DIR/bin")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r')
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
    if [ -f "$HOME/.bashrc" ]; then
      add_to_path "$HOME/.bashrc"
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
echo "Done!"
echo ""
echo "To use objectiveai in your current shell, run:"
echo '  . "$HOME/.objectiveai/env"'
echo ""
echo "(New shells will pick it up automatically.)"
