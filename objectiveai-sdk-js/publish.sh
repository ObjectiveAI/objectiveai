#!/usr/bin/env bash
# Publishes objectiveai-sdk-js to npm as the `@objectiveai/sdk` package.
#
# By default, dispatches the GitHub Actions workflow that runs
# `pnpm publish --access public` against the public npm registry.
#
# Local --build-only mode runs the build pipeline and `pnpm pack` so you
# can inspect the tarball without uploading.
#
# Usage:
#   bash objectiveai-sdk-js/publish.sh                # npm (via GHA)
#   bash objectiveai-sdk-js/publish.sh --build-only   # local build + pnpm pack
#
# `--test` is not supported (npm has no test registry).
#
# Output is captured to .logs/publish/objectiveai-sdk-js.txt.
#
# Setup (one-time):
#   - NPM_TOKEN must be set as a repo secret. Generate an automation token
#     at https://www.npmjs.com/settings/<user>/tokens.
#   - `gh` CLI must be authenticated (gh auth login).

set -euo pipefail

MODULE="objectiveai-sdk-js"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/publish"
LOG_FILE="$LOG_DIR/$MODULE.txt"
WORKFLOW_FILE=".github/workflows/publish-$MODULE.yml"

mkdir -p "$LOG_DIR"

BUILD_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)         echo "ERROR: npm has no test registry; --test is not supported." >&2; exit 1 ;;
    --build-only)   BUILD_ONLY=true; shift ;;
    *)              echo "Unknown flag: $1" >&2; exit 1 ;;
  esac
done

if $BUILD_ONLY; then
  run_local() {
    # ── ensure build ─────────────────────────────────────────────────────────────
    bash "$SCRIPT_DIR/build.sh" || return $?

    # ── pack tarball ─────────────────────────────────────────────────────────────
    rm -f "$SCRIPT_DIR"/objectiveai-sdk-*.tgz
    ( cd "$SCRIPT_DIR" && pnpm pack ) || return $?

    echo "Built artifact:"
    ls -1 "$SCRIPT_DIR"/objectiveai-sdk-*.tgz

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
