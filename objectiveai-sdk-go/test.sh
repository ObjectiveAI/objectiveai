#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-sdk-go.txt.
#
# If OBJECTIVEAI_TEST_PORT is already set, uses that server as-is.
# Otherwise, spawns a new server via test-spawn-api-server.sh and kills it on exit.
#
# Usage:
#   bash objectiveai-sdk-go/test.sh
#   bash objectiveai-sdk-go/test.sh -- -run TestRoundtrip   # pass args to go test

set -euo pipefail

MODULE="objectiveai-sdk-go"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Parse flags
GO_TEST_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; GO_TEST_ARGS=("$@"); break ;;
    *)  GO_TEST_ARGS+=("$1"); shift ;;
  esac
done

# Spawn test server if needed
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE")
  trap 'kill "$PID" 2>/dev/null || true' EXIT
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

# go test -v output:  --- PASS: TestFoo (0.01s)  /  --- FAIL: TestBar (0.02s)
parse_summary() {
  PASSED=$(grep -c '^--- PASS:' "$LOG_FILE" || true)
  FAILED=$(grep -c '^--- FAIL:' "$LOG_FILE" || true)
  TOTAL=$((PASSED + FAILED))
}

# Run tests across both packages, capture all output
if (cd "$SCRIPT_DIR" && go test ./tests/ ./ -v -count=1 "${GO_TEST_ARGS[@]}") >> "$LOG_FILE" 2>&1; then
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS $PASSED/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
