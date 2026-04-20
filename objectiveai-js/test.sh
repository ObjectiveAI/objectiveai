#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-js.txt.
#
# If OBJECTIVEAI_TEST_PORT is already set, uses that server as-is.
# Otherwise, spawns a new server via test-spawn-api-server.sh and kills it on exit.
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

# Spawn test server only if not already provided
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE")
  trap 'kill "$PID" 2>/dev/null || true' EXIT
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

# Run tests, capture all output
if pnpm --filter objectiveai run test -- --reporter=verbose >> "$LOG_FILE" 2>&1; then
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
