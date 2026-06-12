#!/usr/bin/env bash
# Runs all test suites in parallel against the repo's committed
# shared test root (.objectiveai/). No server orchestration: suites
# that need the api run `objectiveai api spawn` themselves, and the
# api lockfile singleton guarantees exactly one server materializes
# no matter how many suites ask. This script owns the bracketing —
# it resets the shared root (kill lockfile owners + wipe state/) at
# start and end via test-cleanup.sh, and tells the inner scripts to
# skip their own bracketing through OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT.
#
# Usage:
#   bash test.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
CLEANUP_LOG="$LOG_DIR/test-cleanup.txt"

mkdir -p "$LOG_DIR"
: > "$CLEANUP_LOG"

export OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT=1

bash "$REPO_ROOT/test-cleanup.sh" >>"$CLEANUP_LOG" 2>&1
trap 'bash "$REPO_ROOT/test-cleanup.sh" >>"$CLEANUP_LOG" 2>&1 || true' EXIT INT TERM

# Run all test suites in parallel.
# Each script prints exactly one line: "$MODULE: PASS N/N" or "$MODULE: FAIL N/N".
PIDS=()
for suite in \
  objectiveai-sdk-rs \
  objectiveai-api \
  objectiveai-json-schema \
  objectiveai-cli \
  objectiveai-mcp-proxy \
  objectiveai-sdk-js \
  objectiveai-sdk-py \
  objectiveai-sdk-go \
  objectiveai-viewer \
  objectiveai-tests \
; do
  bash "$REPO_ROOT/$suite/test.sh" &
  PIDS+=($!)
done

# Wait for all suites, collect exit codes
FAILED=false
for pid in "${PIDS[@]}"; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi
