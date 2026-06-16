#!/usr/bin/env bash
# Builds objectiveai-gemini-sdk-runner and places the binary in embed/<target>/<profile>/.
# Skips the build if the source fingerprint hasn't changed.
# Output is captured to .logs/build/objectiveai-gemini-sdk-runner.txt.
#
# Usage:
#   bash objectiveai-gemini-sdk-runner/build.sh [--release] [--target <triple>]

set -euo pipefail

MODULE="objectiveai-gemini-sdk-runner"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

run() {
  # ── venv setup ──────────────────────────────────────────────────────────────────

  if [ ! -d "$VENV_DIR" ]; then
    echo "Creating virtual environment..."
    python3 -m venv "$VENV_DIR"
  fi

  # Detect venv layout AFTER the venv exists. On a fresh checkout the
  # directory doesn't exist yet, so detection-by-existence-of-Scripts
  # picks the Linux paths on Windows and the build crashes later with
  # "No such file or directory".
  if [ -d "$VENV_DIR/Scripts" ]; then
    PYTHON="$VENV_DIR/Scripts/python.exe"
    PIP="$VENV_DIR/Scripts/pip.exe"
  else
    PYTHON="$VENV_DIR/bin/python"
    PIP="$VENV_DIR/bin/pip"
  fi

  # ── install requirements if missing ─────────────────────────────────────────────

  install_if_missing() {
    local req_file="$1"
    local missing=false
    while IFS= read -r line; do
      [[ -z "$line" || "$line" == \#* || "$line" == -r* || "$line" == ../* ]] && continue
      # Strip URL spec ('pkg @ url'), version spec, and whitespace.
      local pkg
      pkg=$(echo "$line" | sed -E 's/[[:space:]]*@.*//; s/[><=!~].*//' | xargs)
      if [ -z "$pkg" ]; then
        continue
      fi
      # pip show is the single source of truth: works for PyPI and URL installs.
      if ! "$PIP" show "$pkg" >/dev/null 2>&1; then
        missing=true
        break
      fi
    done < "$req_file"

    if $missing; then
      echo "Installing requirements from $req_file..."
      "$PIP" install -r "$req_file" --quiet
    fi
  }

  install_if_missing "$SCRIPT_DIR/requirements.txt"

  if ! "$PYTHON" -c "import PyInstaller" 2>/dev/null; then
    echo "Installing dev requirements..."
    "$PIP" install -r "$SCRIPT_DIR/requirements-dev.txt" --quiet
  fi

  # ── check fingerprint ──────────────────────────────────────────────────────────
  # Returns 1 if embed/ is up to date (not an error).

  if ! source "$SCRIPT_DIR/fingerprint.sh" "$@"; then
    return 0
  fi

  # ── build with PyInstaller ─────────────────────────────────────────────────────

  if [[ "$TARGET" == *"windows"* ]]; then
    BINARY_NAME="$MODULE.exe"
  else
    BINARY_NAME="$MODULE"
  fi

  WORK_DIR="$SCRIPT_DIR/.pyinstaller-work"
  echo "Building $MODULE ($PROFILE, $TARGET)..."
  "$PYTHON" -m PyInstaller \
    --onefile \
    --name "$MODULE" \
    --distpath "$WORK_DIR/dist" \
    --workpath "$WORK_DIR/build" \
    --specpath "$WORK_DIR" \
    --clean \
    "$SCRIPT_DIR/main.py"

  BUILT="$WORK_DIR/dist/$BINARY_NAME"
  if [ ! -f "$BUILT" ]; then
    echo "ERROR: expected binary at $BUILT" >&2
    return 1
  fi

  # Copy binary to embed/<target>/<profile>/
  EMBED_DIR="$SCRIPT_DIR/embed/$TARGET/$PROFILE"
  mkdir -p "$EMBED_DIR"
  cp "$BUILT" "$EMBED_DIR/$BINARY_NAME"

  # Stamp the fingerprint only after successful build.
  echo "$CURRENT_FP" > "$FINGERPRINT_FILE"
  echo "Build complete (fingerprint: ${CURRENT_FP:0:12}...)"
}

if run "$@" > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi
