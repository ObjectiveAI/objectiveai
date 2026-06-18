#!/usr/bin/env bash
# ObjectiveAI installer — installs the pre-built release binaries from the
# per-platform release zip (objectiveai-<version>-<os>-<arch>.zip), which bundles
# the CLI, api, viewer, mcp, db, and the two SDK runners.
#
#   curl -fsSL https://raw.githubusercontent.com/ObjectiveAI/objectiveai/main/install.sh | bash
#
# Options:
#   --objectiveai-dir <dir>   Install root. Falls back to $OBJECTIVEAI_DIR,
#                             then $HOME/.objectiveai.
#   --no-export-path          Don't add the bin dir to PATH / shell rc.
#   --from-source             Build the binaries locally with build.sh
#                             (debug) and install those instead of
#                             downloading a release zip.
#   --from-source-release     Same, but build in release mode.
#
# Zip resolution, in order:
#   1. ./<asset>            (current working directory)
#   2. <dir>/bin/<asset>    (a previously-downloaded copy)
#   3. download from the v<VERSION> GitHub Release into <dir>/bin/<asset>
#      (left in place — not cleaned up, so a re-run reuses it via step 2).
#
# The zip is unpacked into <dir>/bin, replacing any existing binaries.
# The replacement is all-or-nothing: every file is staged first, then
# swapped in with rollback, so a failure never leaves bin/ half-updated.
#
# Layout on disk (bin/ is machine-wide; per-state data lives under
# <dir>/state/<OBJECTIVEAI_STATE>, default "default"):
#   <dir>/bin/objectiveai{.exe}        ← CLI
#   <dir>/bin/objectiveai-api{.exe}
#   <dir>/bin/objectiveai-viewer{.exe}
#   <dir>/bin/objectiveai-mcp{.exe}
#   <dir>/bin/objectiveai-db{.exe}
#   <dir>/bin/objectiveai-claude-agent-sdk-runner{.exe}
#   <dir>/bin/objectiveai-codex-sdk-runner{.exe}
#
# No toolchain required for the default (download) path. To build from a
# repo checkout instead, pass --from-source (or --from-source-release):
# this runs build.sh, then stages the packaged zip so the resolution
# above picks it up (step 2) — no download.

set -euo pipefail

# Release version this installer pulls. Kept in lockstep by version.sh.
VERSION="2.2.3"
REPO="ObjectiveAI/objectiveai"

# ── Parse arguments ───────────────────────────────────────────────────

NO_EXPORT_PATH=0
DIR_ARG=""
FROM_SOURCE=0
FROM_SOURCE_RELEASE=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --no-export-path)
      NO_EXPORT_PATH=1
      shift
      ;;
    --from-source)
      FROM_SOURCE=1
      shift
      ;;
    --from-source-release)
      FROM_SOURCE=1
      FROM_SOURCE_RELEASE=1
      shift
      ;;
    --objectiveai-dir)
      if [ "$#" -lt 2 ]; then
        echo "--objectiveai-dir requires a value" >&2
        exit 1
      fi
      DIR_ARG="$2"
      shift 2
      ;;
    --objectiveai-dir=*)
      DIR_ARG="${1#*=}"
      shift
      ;;
    -h|--help)
      sed -n '2,36p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      echo "run with --help for usage" >&2
      exit 1
      ;;
  esac
done

# --objectiveai-dir wins, then $OBJECTIVEAI_DIR, then the default.
INSTALL_DIR="${DIR_ARG:-${OBJECTIVEAI_DIR:-$HOME/.objectiveai}}"
BIN_DIR="$INSTALL_DIR/bin"

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

# Only these platform/arch combos have release zips.
SUPPORTED=0
case "$PLATFORM-$ARCH" in
  linux-x86_64|linux-aarch64|macos-x86_64|macos-aarch64|windows-x86_64|windows-aarch64)
    SUPPORTED=1 ;;
esac
if [ "$SUPPORTED" = "0" ]; then
  echo "no release zip for $PLATFORM-$ARCH" >&2
  exit 1
fi

ASSET="objectiveai-${VERSION}-${PLATFORM}-${ARCH}.zip"

# ── Locate the zip ────────────────────────────────────────────────────
# 1. CWD, 2. <dir>/bin, 3. download into <dir>/bin (and leave it there).

mkdir -p "$BIN_DIR"

# ── From-source build (optional) ──────────────────────────────────────
# --from-source / --from-source-release: build the binaries locally with
# build.sh (binaries only — `--no-sdk` runs phase 1 + packaging), then
# stage the resulting zip into BIN_DIR. The zip resolution below then
# finds it as a "cached" copy (step 2) and unpacks it — no download.
if [ "$FROM_SOURCE" = "1" ]; then
  SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
  if [ ! -f "$SCRIPT_DIR/build.sh" ]; then
    echo "--from-source requires a repo checkout (build.sh not found beside install.sh)" >&2
    exit 1
  fi
  BUILD_ARGS=(--no-sdk)
  if [ "$FROM_SOURCE_RELEASE" = "1" ]; then
    BUILD_ARGS+=(--release)
  fi
  echo "Building from source: bash build.sh ${BUILD_ARGS[*]}"
  bash "$SCRIPT_DIR/build.sh" "${BUILD_ARGS[@]}"
  # build.sh packages the host zip into
  # ${OBJECTIVEAI_DIR:-$HOME/.objectiveai}/bin/<asset> — the same dir
  # resolution build.sh itself uses (it doesn't know --objectiveai-dir).
  BUILT_ZIP="${OBJECTIVEAI_DIR:-$HOME/.objectiveai}/bin/$ASSET"
  if [ ! -f "$BUILT_ZIP" ]; then
    echo "build.sh did not produce $BUILT_ZIP" >&2
    exit 1
  fi
  if [ "$BUILT_ZIP" != "$BIN_DIR/$ASSET" ]; then
    cp -f "$BUILT_ZIP" "$BIN_DIR/$ASSET"
    echo "Staged $ASSET into $BIN_DIR."
  fi
fi

download() {
  local url="$1" dst="$2"
  # Download to a temp sibling, then move into place, so a failed/partial
  # download never leaves a corrupt <asset> that step 2 would treat as
  # a valid cached copy on the next run.
  local tmp="$dst.partial.$$"
  echo "Downloading $ASSET (v$VERSION)..."
  if command -v curl >/dev/null 2>&1; then
    # -L follows the release redirect; -f fails hard on 4xx/5xx.
    curl -fSL --progress-bar "$url" -o "$tmp"
  elif command -v wget >/dev/null 2>&1; then
    wget -O "$tmp" "$url"
  else
    echo "need curl or wget to download" >&2
    return 1
  fi
  if [ ! -s "$tmp" ]; then
    rm -f "$tmp"
    echo "download produced an empty file" >&2
    return 1
  fi
  mv -f "$tmp" "$dst"
}

if [ -f "$PWD/$ASSET" ]; then
  ZIP="$PWD/$ASSET"
  echo "Using $ASSET from the current directory."
elif [ -f "$BIN_DIR/$ASSET" ]; then
  ZIP="$BIN_DIR/$ASSET"
  echo "Using cached $ASSET from $BIN_DIR."
else
  ZIP="$BIN_DIR/$ASSET"
  download "https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}" "$ZIP"
fi

# ── Unpack (all-or-nothing) ───────────────────────────────────────────
# Extract everything to a staging dir on the same filesystem as bin/,
# then swap each file into place with rollback. A failure at any point
# leaves bin/ exactly as it was.

STAGING="$BIN_DIR/.objectiveai-install-staging.$$"
rm -rf "$STAGING"
mkdir -p "$STAGING"
trap 'rm -rf "$STAGING"' EXIT

echo "Unpacking into $BIN_DIR..."
if command -v unzip >/dev/null 2>&1; then
  unzip -o -q "$ZIP" -d "$STAGING"
elif command -v tar >/dev/null 2>&1; then
  # bsdtar (macOS, modern Windows) reads zips; GNU tar does not.
  tar -xf "$ZIP" -C "$STAGING"
else
  echo "need unzip or tar to unpack $ASSET" >&2
  exit 1
fi

commit_files() {
  local placed=() backups=() entry target bak f base ok=1

  rollback() {
    local t e tgt bk
    for t in "${placed[@]}"; do rm -f "$t"; done
    for e in "${backups[@]}"; do
      tgt="${e%%|*}"; bk="${e##*|}"
      mv -f "$bk" "$tgt" 2>/dev/null || true
    done
  }

  shopt -s nullglob
  for f in "$STAGING"/*; do
    [ -f "$f" ] || continue
    base=$(basename "$f")
    target="$BIN_DIR/$base"
    if [ -e "$target" ]; then
      bak="$BIN_DIR/.bak.$$.$base"
      if ! mv -f "$target" "$bak"; then ok=0; break; fi
      backups+=("$target|$bak")
    fi
    if ! mv -f "$f" "$target"; then ok=0; break; fi
    placed+=("$target")
  done

  if [ "$ok" != "1" ]; then
    rollback
    echo "unpack failed — rolled back; $BIN_DIR left unchanged" >&2
    return 1
  fi

  # Commit: make the new binaries executable, drop the backups.
  for target in "${placed[@]}"; do chmod +x "$target" 2>/dev/null || true; done
  for entry in "${backups[@]}"; do
    bak="${entry##*|}"
    rm -f "$bak" 2>/dev/null || true
  done
  for target in "${placed[@]}"; do echo "Installed $target"; done
}

commit_files

# ── PATH ──────────────────────────────────────────────────────────────
# A child process can't mutate its parent shell's environment, so we add a
# guarded `export PATH` for the BIN dir directly to the user's shell rc
# files (and the Windows user PATH). We do NOT write an `env` file into the
# install dir — and we remove a stale one a previous installer may have left.

# Never leave an `env` file behind. Older installers dropped a sourceable
# "$INSTALL_DIR/env"; delete it so nothing lingers in the install dir.
rm -f "$INSTALL_DIR/env"

add_to_path() {
  local shell_rc="$1"
  # Idempotent: skip if this rc already puts the bin dir on PATH.
  if [ -f "$shell_rc" ] && grep -qF "$BIN_DIR" "$shell_rc"; then
    return
  fi
  # Write the export directly (no env file to source). Expands $BIN_DIR at
  # write time; keeps $PATH literal so it re-resolves on each shell start.
  cat >> "$shell_rc" <<EOF

# ObjectiveAI
case ":\${PATH}:" in
    *:"$BIN_DIR":*) ;;
    *) export PATH="$BIN_DIR:\$PATH" ;;
esac
EOF
  echo "Added $BIN_DIR to PATH in $shell_rc"
}

if [ "$NO_EXPORT_PATH" = "1" ]; then
  echo ""
  echo "Done! (skipped PATH export — --no-export-path)"
  echo "The binaries are in $BIN_DIR."
  exit 0
fi

case "$PLATFORM" in
  windows)
    BIN_DIR_WIN="$(cygpath -w "$BIN_DIR" 2>/dev/null || echo "$BIN_DIR")"
    CURRENT_PATH=$(powershell.exe -NoProfile -Command "[Environment]::GetEnvironmentVariable('Path', 'User')" 2>/dev/null | tr -d '\r' || true)
    NEED_PREPEND=""
    if ! echo "$CURRENT_PATH" | grep -qiF "$BIN_DIR_WIN"; then
      NEED_PREPEND="$BIN_DIR_WIN;"
    fi
    if [ -n "$NEED_PREPEND" ]; then
      powershell.exe -NoProfile -Command \
        "[Environment]::SetEnvironmentVariable('Path', '$NEED_PREPEND' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')" 2>/dev/null
      echo "Added $NEED_PREPEND to user PATH (restart cmd/PowerShell to use it)."
    else
      echo "PATH already contains $BIN_DIR_WIN"
    fi
    # Also wire up Git Bash / MSYS via ~/.bashrc.
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
echo "Restart your shell, or run this to use the binaries now:"
echo "  export PATH=\"$BIN_DIR:\$PATH\""
echo ""
echo "(New shells will pick it up automatically.)"
