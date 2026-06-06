#!/usr/bin/env bash
# Runs objectiveai-api tests.
# Output is captured to .logs/test/objectiveai-api.txt.
#
# Usage:
#   bash objectiveai-api/test.sh
#   bash objectiveai-api/test.sh -- -E 'test(client_tests)'  # pass args to nextest
#   UPDATE_SNAPSHOTS=1 bash objectiveai-api/test.sh           # regenerate all snapshots

set -euo pipefail

MODULE="objectiveai-api"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

mkdir -p "$LOG_DIR"

# If UPDATE_SNAPSHOTS is set, propagate to all 6 snapshot env vars.
if [ "${UPDATE_SNAPSHOTS:-}" = "1" ]; then
  export UPDATE_AGENT_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_AGENT_COMPLETIONS_MOCK_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS=1
fi

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" "${CARGO_ARGS[@]}" > "$LOG_FILE" 2>&1; then
  PASSED=$(sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  FAILED=$(sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  TOTAL=$((PASSED + FAILED))
  echo "$MODULE: PASS $PASSED/$TOTAL"
else
  PASSED=$(sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  FAILED=$(sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p' "$LOG_FILE" | awk '{s+=$1} END {print s+0}')
  TOTAL=$((PASSED + FAILED))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
