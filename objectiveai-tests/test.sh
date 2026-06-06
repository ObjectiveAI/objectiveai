#!/usr/bin/env bash
# Runs objectiveai-tests (repo-wide invariant tests).
# Output is captured to .logs/test/objectiveai-tests.txt.
#
# Usage:
#   bash objectiveai-tests/test.sh
#   bash objectiveai-tests/test.sh -- --test-threads=1   # pass args to nextest

set -euo pipefail

MODULE="objectiveai-tests"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

mkdir -p "$LOG_DIR"

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
