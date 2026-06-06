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

# The cli_command integration test spawns a real cli binary through
# the SDK's BinaryExecutor. Reuse the same artifact the cli suite's
# `prepare.sh` builds — `--no-default-features --features rustpython`
# into `target/objectiveai-tests/` — so a parallel `test.sh` run
# builds the cli exactly once and both suites share it. `systempython`
# is an empty marker feature, so the rustpython-only build is
# functionally identical to the default. Cargo's per-target-dir build
# lock serializes any concurrent invocations safely.
CLI_TARGET_DIR="$REPO_ROOT/target/objectiveai-tests"
if ! cargo build \
    -p objectiveai-cli \
    --no-default-features --features rustpython \
    --target-dir "$CLI_TARGET_DIR" \
    > "$LOG_FILE" 2>&1; then
  echo "$MODULE: FAIL"
  exit 1
fi
CLI_BIN="$CLI_TARGET_DIR/debug/objectiveai-cli"
if [ -f "$CLI_BIN.exe" ]; then CLI_BIN="$CLI_BIN.exe"; fi
export OBJECTIVEAI_CLI_BINARY="$CLI_BIN"

# --lib --tests: skip the bin target. Tauri's deps (tauri, windows,
# encoding_rs, objectiveai_mcp_proxy) can't be linked under cargo test's
# bin-target compile pass — cargo can't satisfy the bin's cdylib-flavoured
# linkage AND the rlib form the test pass would need. Only the library +
# integration tests need to run; main.rs is a thin shim with no
# `#[cfg(test)]` coverage worth keeping. Release builds use cargo build /
# tauri build and are unaffected.
#
# cargo-nextest is installed locally by `build-bin.sh` into `bin/` — see
# the [workspace.metadata.tools] table in the root Cargo.toml.
if "$NEXTEST" nextest run -p objectiveai-viewer --lib --tests "${CARGO_ARGS[@]}" > "$LOG_FILE" 2>&1; then
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
