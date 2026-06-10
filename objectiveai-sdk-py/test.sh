#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-sdk-py.txt.
#
# If OBJECTIVEAI_TEST_PORT is already set, uses that server as-is.
# Otherwise, spawns a new server via test-spawn-api-server.sh and reaps it on
# exit (kill+wait, resilient to Ctrl-C / SIGTERM).
#
# Usage:
#   bash objectiveai-sdk-py/test.sh                  # run all tests
#   bash objectiveai-sdk-py/test.sh -- -k mock_7 -vv # pass args to pytest

set -euo pipefail

MODULE="objectiveai-sdk-py"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Parse flags
PYTEST_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --)          shift; PYTEST_ARGS=("$@"); break ;;
    *)           PYTEST_ARGS+=("$1"); shift ;;
  esac
done

# Platform-independent venv path
if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

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
# pytest summary: "1060 passed, 31 skipped, 2 warnings in 17.66s" or "3 failed, 1057 passed ..."
parse_summary() {
  local summary
  summary=$(tail -1 "$LOG_FILE")
  PASSED=$(echo "$summary" | sed -n 's/.* \([0-9]*\) passed.*/\1/p')
  FAILED=$(echo "$summary" | sed -n 's/.* \([0-9]*\) failed.*/\1/p')
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
}

if "$PYTHON" -m pytest "$SCRIPT_DIR/tests/" -v --tb=long "${PYTEST_ARGS[@]}" >> "$LOG_FILE" 2>&1; then
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
