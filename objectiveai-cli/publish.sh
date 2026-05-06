#!/usr/bin/env bash
# Publishes objectiveai-cli to GitHub Releases as compiled binaries.
#
# This is a thin wrapper around `.github/workflows/release.yml`, which is the
# canonical CLI release pipeline. That workflow normally fires automatically
# on push to main: if `objectiveai-cli/Cargo.toml` declares a version that
# has no matching `vX.Y.Z` GitHub release, it builds the 10-target binary
# matrix (5 platforms × {viewer-bundled, viewer-omitted}) and creates a draft
# release.
#
# This `publish.sh` exists for the rare case of a manual re-trigger after
# deleting a draft release, or for sanity-checking a build locally.
#
# Usage:
#   bash objectiveai-cli/publish.sh                # dispatch release.yml manually
#   bash objectiveai-cli/publish.sh --build-only   # local cargo build --release smoke test
#
# `--test` is not supported.
#
# Output is captured to .logs/publish/objectiveai-cli.txt.
#
# No new GHA workflow file — release.yml already supports workflow_dispatch.

set -euo pipefail

MODULE="objectiveai-cli"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"
WORKFLOW_FILE=".github/workflows/release.yml"

mkdir -p "$LOG_DIR"

BUILD_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)         echo "ERROR: --test is not supported for the CLI release flow." >&2; exit 1 ;;
    --build-only)   BUILD_ONLY=true; shift ;;
    *)              echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

if $BUILD_ONLY; then
  run_local() {
    echo "Running cargo build --release -p objectiveai-cli (smoke test)..."
    cargo build --release -p objectiveai-cli || return $?
    echo "--build-only specified; skipping release. (Real publishes go through release.yml.)"
  }

  if run_local > "$LOG_FILE" 2>&1; then
    echo "$MODULE: BUILT (local)"
  else
    echo "$MODULE: ERROR (see $LOG_FILE)"
    exit 1
  fi
  exit 0
fi

run_remote() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "ERROR: gh CLI not found. Install it (https://cli.github.com/) or use --build-only." >&2
    return 1
  fi

  if ! gh auth status >/dev/null 2>&1; then
    echo "ERROR: gh CLI not authenticated. Run 'gh auth login' first." >&2
    return 1
  fi

  echo "Triggering $WORKFLOW_FILE on default branch..."
  gh workflow run "$WORKFLOW_FILE" \
    --repo "$(gh repo view --json nameWithOwner -q .nameWithOwner)"

  echo
  echo "Workflow dispatched. Watch progress with:"
  echo "  gh run list --workflow=$WORKFLOW_FILE"
  echo "  gh run watch \$(gh run list --workflow=$WORKFLOW_FILE --limit=1 --json databaseId -q '.[0].databaseId')"
}

if run_remote 2>&1 | tee "$LOG_FILE"; then
  echo "$MODULE: WORKFLOW DISPATCHED"
else
  echo "$MODULE: ERROR (see $LOG_FILE)"
  exit 1
fi
