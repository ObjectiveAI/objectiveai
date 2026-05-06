#!/usr/bin/env bash
# Publishes objectiveai-api to crates.io as the `objectiveai-api` crate.
#
# By default, dispatches the GitHub Actions workflow that runs
# `cargo publish -p objectiveai-api` against crates.io.
#
# Local --build-only mode runs `cargo publish --dry-run` so you can
# verify the package metadata + tarball without uploading.
#
# Usage:
#   bash objectiveai-api/publish.sh                # crates.io (via GHA)
#   bash objectiveai-api/publish.sh --build-only   # local cargo publish --dry-run
#
# `--test` is not supported (crates.io has no test registry).
#
# Output is captured to .logs/publish/objectiveai-api.txt.
#
# Dependency note: this crate depends on `objectiveai` at the same version.
# When the root `publish.sh` dispatches everything in parallel, this workflow
# may race-fail on a fresh version bump because crates.io can't yet see the
# new `objectiveai` version. The fix is simply to rerun this script once
# `objectiveai` is live on crates.io.
#
# Setup (one-time):
#   - CARGO_REGISTRY_TOKEN must be set as a repo secret. Generate at
#     https://crates.io/me with the `publish-update` scope.
#   - `gh` CLI must be authenticated (gh auth login).

set -euo pipefail

MODULE="objectiveai-api"
CRATE="objectiveai-api"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"
WORKFLOW_FILE=".github/workflows/publish-$MODULE.yml"

mkdir -p "$LOG_DIR"

BUILD_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)         echo "ERROR: crates.io has no test registry; --test is not supported." >&2; exit 1 ;;
    --build-only)   BUILD_ONLY=true; shift ;;
    *)              echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

if $BUILD_ONLY; then
  run_local() {
    echo "Running cargo publish --dry-run --allow-dirty -p $CRATE..."
    cargo publish --dry-run --allow-dirty -p "$CRATE" || return $?
    echo "--build-only specified; skipping upload. (Real publishes go through GHA.)"
  }

  if run_local > "$LOG_FILE" 2>&1; then
    echo "$MODULE: BUILT (cargo publish --dry-run)"
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
