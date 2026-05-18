#!/usr/bin/env bash
# Runs objectiveai-viewer tests.
# Output is captured to .logs/test/objectiveai-viewer.txt.
#
# The viewer tests need the workspace's shared API server, so this
# script expects OBJECTIVEAI_TEST_PORT to be set in the environment
# (the root test.sh does this). When invoked standalone without the
# port, the integration tests print a "skipping" line and pass; only
# the in-src tests run.
#
# Usage:
#   bash objectiveai-viewer/test.sh
#   bash objectiveai-viewer/test.sh -- --test-threads=1

set -euo pipefail

MODULE="objectiveai-viewer"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# --features cli: integration tests drive the cli/api_call command
# impls that only exist when the `cli` feature is enabled.
# --lib --tests: skip the bin target. Tauri's deps (tauri, windows,
# encoding_rs, objectiveai_mcp_proxy) can't be linked under cargo test's
# bin-target compile pass — cargo can't satisfy the bin's cdylib-flavoured
# linkage AND the rlib form the test pass would need. Only the library +
# integration tests need to run; main.rs is a thin shim with no
# `#[cfg(test)]` coverage worth keeping. Release builds use cargo build /
# tauri build and are unaffected.
if cargo test -p objectiveai-viewer --features cli --lib --tests "${CARGO_ARGS[@]}" > "$LOG_FILE" 2>&1; then
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
