#!/usr/bin/env bash
# Publishes objectiveai-cocoindex to PyPI as the `objectiveai-cocoindex`
# distribution.
#
# Pure-Python package (hatchling backend, no Rust extension), so a single
# universal wheel + sdist is sufficient. The default flow dispatches the
# GitHub Actions workflow for consistency with the rest of the publish
# scripts; --build-only does a local sdist+wheel sanity check.
#
# Usage:
#   bash objectiveai-cocoindex/publish.sh                # PyPI (via GHA)
#   bash objectiveai-cocoindex/publish.sh --test         # TestPyPI (via GHA)
#   bash objectiveai-cocoindex/publish.sh --build-only   # local sdist + wheel
#
# Output is captured to .logs/publish/objectiveai-cocoindex.txt.
#
# Dependency note: this package pins `objectiveai==<X.Y.Z>` in pyproject.toml.
# When the root `publish.sh` dispatches everything in parallel, this workflow
# uploads its own wheel/sdist regardless of whether the matching `objectiveai`
# version is already on PyPI. Downstream `pip install` will fail until the
# matching `objectiveai` release lands on PyPI — which is fine, just rerun
# the affected pip install once it does.
#
# Setup (one-time):
#   - PYPI_API_TOKEN must be set as a repo secret (TEST_PYPI_API_TOKEN for
#     --test). Generate at https://pypi.org/manage/account/token/.
#   - `gh` CLI must be authenticated (gh auth login).

set -euo pipefail

MODULE="objectiveai-cocoindex"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
DIST_DIR="$SCRIPT_DIR/dist"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"
WORKFLOW_FILE=".github/workflows/publish-$MODULE.yml"

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
    # ── ensure venv via build.sh ─────────────────────────────────────────────────
    bash "$SCRIPT_DIR/build.sh" || return $?

    if [ -d "$VENV_DIR/Scripts" ]; then
      PYTHON="$VENV_DIR/Scripts/python.exe"
      PIP="$VENV_DIR/Scripts/pip.exe"
    else
      PYTHON="$VENV_DIR/bin/python"
      PIP="$VENV_DIR/bin/pip"
    fi

    # ── install build + twine if missing ─────────────────────────────────────────
    if ! "$PYTHON" -c "import build" 2>/dev/null; then
      "$PIP" install --quiet build || return $?
    fi
    if ! "$PYTHON" -c "import twine" 2>/dev/null; then
      "$PIP" install --quiet twine || return $?
    fi

    # ── clean previous build artifacts ───────────────────────────────────────────
    rm -rf "$DIST_DIR" "$SCRIPT_DIR/build" "$SCRIPT_DIR"/*.egg-info

    # ── build sdist + wheel ──────────────────────────────────────────────────────
    echo "Building local sdist + wheel..."
    ( cd "$SCRIPT_DIR" && "$PYTHON" -m build --sdist --wheel --outdir "$DIST_DIR" ) || return $?

    echo "Built artifacts:"
    ls -1 "$DIST_DIR"

    echo "Validating distributions..."
    "$PYTHON" -m twine check "$DIST_DIR"/* || return $?

    echo "--build-only specified; skipping upload. (Real publishes go through GHA.)"
  }

  if run_local > "$LOG_FILE" 2>&1; then
    echo "$MODULE: BUILT (local)"
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
