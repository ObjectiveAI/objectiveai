#!/usr/bin/env bash
# Runs tests for objectiveai-cocoindex.
# Output is captured to .logs/test/objectiveai-cocoindex.txt.
#
# Usage:
#   bash objectiveai-cocoindex/test.sh                  # run all tests
#   bash objectiveai-cocoindex/test.sh -- -k foo -vv    # pass args to pytest

set -euo pipefail

MODULE="objectiveai-cocoindex"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

PYTEST_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --)          shift; PYTEST_ARGS=("$@"); break ;;
    *)           PYTEST_ARGS+=("$1"); shift ;;
  esac
done

if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

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
