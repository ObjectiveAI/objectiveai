#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-js.txt.
#
# If OBJECTIVEAI_TEST_PORT is already set, uses that server as-is.
# Otherwise, spawns a new server via test-spawn-api-server.sh and reaps it on
# exit (kill+wait, resilient to Ctrl-C / SIGTERM).
#
# Usage:
#   bash objectiveai-js/test.sh

set -euo pipefail

MODULE="objectiveai-js"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Reap the api server we spawn (if any). SERVER_PID stays empty when a
# parent harness provided OBJECTIVEAI_TEST_PORT, so the trap leaves the
# parent's server alone. kill+wait guarantees no orphaned objectiveai-api
# process survives a standalone run; trapping INT/TERM (not just EXIT)
# keeps cleanup resilient to Ctrl-C / kill.
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Spawn ONE test api server only if a parent harness hasn't already
# provided OBJECTIVEAI_TEST_PORT. Capture the pid so the cleanup trap
# can reap it on exit.
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT SERVER_PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE") || {
    echo "$MODULE: FATAL — failed to spawn API server (see $LOG_FILE)" >&2
    exit 1
  }
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

# Run tests, capture all output
if pnpm --filter @objectiveai/sdk run test -- --reporter=verbose >> "$LOG_FILE" 2>&1; then
  # vitest summary: "Tests  959 passed | 6 todo (965)" or "Tests  3 failed | 959 passed | 6 todo (965)"
  # Strip ANSI codes; parse passed + failed, ignore todo/skipped
  CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG_FILE")
  PASSED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) passed.*/\1/p' | tail -1 || true)
  FAILED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) failed.*/\1/p' | tail -1 || true)
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG_FILE")
  PASSED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) passed.*/\1/p' | tail -1 || true)
  FAILED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) failed.*/\1/p' | tail -1 || true)
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
