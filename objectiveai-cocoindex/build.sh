#!/usr/bin/env bash
# Sets up venv and installs the objectiveai-cocoindex package (editable).
# Output is captured to .logs/build/objectiveai-cocoindex.txt.
#
# Usage:
#   bash objectiveai-cocoindex/build.sh

set -euo pipefail

MODULE="objectiveai-cocoindex"
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

  # Detect venv layout AFTER the venv exists so a fresh checkout picks the right paths.
  if [ -d "$VENV_DIR/Scripts" ]; then
    PYTHON="$VENV_DIR/Scripts/python.exe"
    PIP="$VENV_DIR/Scripts/pip.exe"
  else
    PYTHON="$VENV_DIR/bin/python"
    PIP="$VENV_DIR/bin/pip"
  fi

  # ── stage README + LICENSE (pyproject.toml references them; gitignored,
  # never committed in objectiveai-cocoindex/).
  cp "$REPO_ROOT/README.md" "$SCRIPT_DIR/README.md"
  cp "$REPO_ROOT/LICENSE"   "$SCRIPT_DIR/LICENSE"

  # ── install dev requirements (pytest, pytest-asyncio) ──────────────────────────
  if ! "$PYTHON" -c "import pytest" 2>/dev/null; then
    echo "Installing dev requirements..."
    "$PIP" install -r "$SCRIPT_DIR/requirements-dev.txt" --quiet
  fi

  # ── editable install of sibling objectiveai-py FIRST. This lands `objectiveai`
  # in the venv at the version specified in objectiveai-py/pyproject.toml. The
  # subsequent `pip install -e .` for objectiveai-cocoindex sees the version
  # already satisfies its `objectiveai==X.Y.Z` pin and leaves the editable
  # install alone — so local edits to ../objectiveai-py are picked up live.
  # When users `pip install objectiveai-cocoindex` from PyPI, this redirect
  # doesn't fire; pip pulls `objectiveai==X.Y.Z` from PyPI as a normal dep.
  if ! "$PYTHON" -c "import objectiveai" 2>/dev/null; then
    echo "Installing objectiveai from sibling source ($REPO_ROOT/objectiveai-py)..."
    "$PIP" install -e "$REPO_ROOT/objectiveai-py" --quiet
  fi

  # ── editable install of objectiveai-cocoindex itself. Pulls `cocoindex` from
  # PyPI (declared in pyproject.toml [project.dependencies]) and confirms the
  # `objectiveai==X.Y.Z` pin against the already-installed editable sibling.
  if ! "$PYTHON" -c "import objectiveai_cocoindex" 2>/dev/null; then
    echo "Editable-installing objectiveai-cocoindex..."
    "$PIP" install -e "$SCRIPT_DIR" --quiet
  fi

  # ── back-compat: also ensure plain requirements.txt entries are installed
  # (e.g., for callers that pip install -r requirements.txt directly). The
  # objectiveai pin is filtered out — that's the dev redirect we already did.
  REQS_FILTERED=$(mktemp)
  grep -v '^objectiveai[[:space:]]*==' "$SCRIPT_DIR/requirements.txt" > "$REQS_FILTERED" || true
  if [ -s "$REQS_FILTERED" ]; then
    "$PIP" install -r "$REQS_FILTERED" --quiet
  fi
  rm -f "$REQS_FILTERED"
}

if run > "$LOG_FILE" 2>&1; then
  echo "$MODULE: SUCCESS"
else
  echo "$MODULE: ERROR"
  exit 1
fi
