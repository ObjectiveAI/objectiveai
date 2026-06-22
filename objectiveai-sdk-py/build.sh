#!/usr/bin/env bash
# Sets up venv, installs requirements, and builds pyo3.
# Output is captured to .logs/build/objectiveai-sdk-py.txt.
#
# Build profile defaults to debug. Pass --release (or set
# OBJECTIVEAI_BUILD_RELEASE=1, which the root build.sh exports) to compile
# the _pyo3 extension optimized (maturin develop --release). This only
# affects the local editable install; published wheels are built release
# by the publish-objectiveai-sdk-py.yml workflow.
#
# Usage:
#   bash objectiveai-sdk-py/build.sh [--release]

set -euo pipefail

MODULE="objectiveai-sdk-py"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
LOG_DIR="$REPO_ROOT/.logs/build"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

# ── Resolve build profile ──────────────────────────────────────────────
# --release flag or OBJECTIVEAI_BUILD_RELEASE=1 → release; default debug.
PROFILE="debug"
[ "${OBJECTIVEAI_BUILD_RELEASE:-}" = "1" ] && PROFILE="release"
for arg in "$@"; do
  case "$arg" in
    --release) PROFILE="release" ;;
    *) echo "$MODULE: unknown argument: $arg" >&2; exit 1 ;;
  esac
done
if [ "$PROFILE" = "release" ]; then
  MATURIN_PROFILE_FLAG="--release"
else
  MATURIN_PROFILE_FLAG=""
fi

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
      local pkg
      pkg=$(echo "$line" | sed 's/[><=!].*//' | tr '-' '_')
      if ! "$PYTHON" -c "import $pkg" 2>/dev/null; then
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

  if ! "$PYTHON" -c "import pytest" 2>/dev/null || ! "$PYTHON" -c "import maturin" 2>/dev/null; then
    echo "Installing dev requirements..."
    "$PIP" install -r "$SCRIPT_DIR/requirements-dev.txt" --quiet
  fi

  # ── pydantic type generation ────────────────────────────────────────────────────

  "$PYTHON" "$SCRIPT_DIR/scripts/install_pydantic.py"

  # ── cli/command execute functions (the "CLI running" layer) ─────────────────────
  # Mirrors the JS order (install-zod → install-command-execute): runs AFTER
  # install_pydantic (it consumes the types tree + the per-leaf `_generated.py`
  # barrels) and emits one `_execute.py` per command leaf, surfaced through each
  # barrel. Pure-Python codegen — no Rust dependency, so it runs before maturin.
  "$PYTHON" "$SCRIPT_DIR/scripts/install_command_execute.py"

  # ── stage README + LICENSE (pyproject.toml references them; sdists can't
  # include `../`, so we copy the canonical files from repo root). Gitignored,
  # never committed.
  cp "$REPO_ROOT/README.md" "$SCRIPT_DIR/README.md"
  cp "$REPO_ROOT/LICENSE" "$SCRIPT_DIR/LICENSE"

  # ── maturin build + editable install (compiles _pyo3 into the venv) ─────────────
  # maturin auto-discovers a virtualenv named .venv, but ours is named venv,
  # so we point it explicitly via VIRTUAL_ENV.

  # The Rust crate lives in sibling objectiveai-sdk-rs-pyo3/. maturin reads its
  # location from `[tool.maturin] manifest-path` in pyproject.toml. Run from
  # this directory so maturin picks up *our* pyproject.toml (which provides
  # the `objectiveai-sdk` package name + manifest-path); passing --manifest-path
  # to the sibling crate would make maturin think it's building that crate
  # standalone (no pyproject.toml → wrong package name).
  echo "Building _pyo3 ($PROFILE) via maturin develop..."
  ( cd "$SCRIPT_DIR" && VIRTUAL_ENV="$VENV_DIR" "$PYTHON" -m maturin develop $MATURIN_PROFILE_FLAG )
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS ($PROFILE)"
else
  echo "$MODULE: ERROR"
  exit 1
fi
