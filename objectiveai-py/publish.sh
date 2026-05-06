#!/usr/bin/env bash
# Publishes objectiveai-py to PyPI as the `objectiveai` distribution.
#
# By default, triggers the GitHub Actions workflow that builds wheels for
# Linux (x86_64+aarch64), macOS (x86_64+arm64), and Windows (x86_64) plus an
# sdist, then uploads via PyPI Trusted Publishing.
#
# Local --build-only mode produces only a single-platform wheel for the
# current host — useful for verifying maturin is wired up. Real publishes
# must go through GHA so all platforms get coverage.
#
# Usage:
#   bash objectiveai-py/publish.sh                  # PyPI (cross-platform via GHA)
#   bash objectiveai-py/publish.sh --test           # TestPyPI (cross-platform via GHA)
#   bash objectiveai-py/publish.sh --build-only     # local single-platform sdist+wheel
#
# Output is captured to .logs/publish/objectiveai-py.txt.
#
# Setup (one-time):
#   - PYPI_API_TOKEN must be set as a repo secret (and TEST_PYPI_API_TOKEN
#     for --test). See .github/workflows/publish-objectiveai-py.yml.
#   - `gh` CLI must be authenticated (gh auth login).

set -euo pipefail

MODULE="objectiveai-py"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
DIST_DIR="$SCRIPT_DIR/dist"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"
WORKFLOW_FILE=".github/workflows/publish-objectiveai-py.yml"

mkdir -p "$LOG_DIR"

TARGET="pypi"
BUILD_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)         TARGET="testpypi"; shift ;;
    --build-only)   BUILD_ONLY=true; shift ;;
    *)              echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

if $BUILD_ONLY; then
  run_local() {
    # ── load .env (auto-export every assignment so subprocesses inherit) ──────────
    if [ -f "$SCRIPT_DIR/.env" ]; then
      echo "Loading $SCRIPT_DIR/.env"
      set -a
      # shellcheck disable=SC1091
      source "$SCRIPT_DIR/.env"
      set +a
    fi

    # ── ensure venv + codegen + maturin via build.sh ──────────────────────────────
    bash "$SCRIPT_DIR/build.sh"

    if [ -d "$VENV_DIR/Scripts" ]; then
      PYTHON="$VENV_DIR/Scripts/python.exe"
    else
      PYTHON="$VENV_DIR/bin/python"
    fi

    # ── stage README + LICENSE ────────────────────────────────────────────────────
    cp "$REPO_ROOT/README.md" "$SCRIPT_DIR/README.md"
    cp "$REPO_ROOT/LICENSE"   "$SCRIPT_DIR/LICENSE"

    # ── clean previous build artifacts ────────────────────────────────────────────
    rm -rf "$DIST_DIR" "$SCRIPT_DIR/build" "$SCRIPT_DIR"/*.egg-info

    # ── build sdist + wheel via maturin (PEP 517 backend) ─────────────────────────
    echo "Building local sdist + wheel via maturin..."
    if ! "$PYTHON" -c "import build" 2>/dev/null; then
      "$VENV_DIR/Scripts/pip.exe" install --quiet build 2>/dev/null \
        || "$VENV_DIR/bin/pip" install --quiet build
    fi
    if ! "$PYTHON" -c "import twine" 2>/dev/null; then
      "$VENV_DIR/Scripts/pip.exe" install --quiet twine 2>/dev/null \
        || "$VENV_DIR/bin/pip" install --quiet twine
    fi

    ( cd "$SCRIPT_DIR" && "$PYTHON" -m build --sdist --wheel --outdir "$DIST_DIR" )

    echo "Built artifacts:"
    ls -1 "$DIST_DIR"

    echo "Validating distributions..."
    "$PYTHON" -m twine check "$DIST_DIR"/*

    echo "--build-only specified; skipping upload. (Real publishes go through GHA.)"
  }

  if run_local > "$LOG_FILE" 2>&1; then
    echo "$MODULE: BUILT (local, single-platform)"
  else
    echo "$MODULE: ERROR (see $LOG_FILE)"
    exit 1
  fi
  exit 0
fi

# ── GHA-driven publish ────────────────────────────────────────────────────────────

run_remote() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "ERROR: gh CLI not found. Install it (https://cli.github.com/) or use --build-only." >&2
    return 1
  fi

  if ! gh auth status >/dev/null 2>&1; then
    echo "ERROR: gh CLI not authenticated. Run 'gh auth login' first." >&2
    return 1
  fi

  echo "Triggering $WORKFLOW_FILE on default branch (target=$TARGET)..."
  gh workflow run "$WORKFLOW_FILE" \
    --repo "$(gh repo view --json nameWithOwner -q .nameWithOwner)" \
    -f target="$TARGET"

  echo
  echo "Workflow dispatched. Watch progress with:"
  echo "  gh run list --workflow=$WORKFLOW_FILE"
  echo "  gh run watch \$(gh run list --workflow=$WORKFLOW_FILE --limit=1 --json databaseId -q '.[0].databaseId')"
}

if run_remote 2>&1 | tee "$LOG_FILE"; then
  echo "$MODULE: WORKFLOW DISPATCHED ($TARGET)"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi
