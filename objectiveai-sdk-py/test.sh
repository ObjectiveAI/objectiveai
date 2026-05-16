#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-sdk-py.txt.
#
# If OBJECTIVEAI_TEST_PORT is already set, uses that server as-is.
# Otherwise, spawns a new server via test-spawn-api-server.sh and kills it on exit.
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

# Spawn test server if needed
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE")
  trap 'kill "$PID" 2>/dev/null || true' EXIT
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
