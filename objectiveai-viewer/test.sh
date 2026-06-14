#!/usr/bin/env bash
# Runs objectiveai-viewer tests.
# Output is captured to .logs/test/objectiveai-viewer.txt.
#
# No preparation and no server lifecycle: the `cli_command`
# integration test drives the cli shim from the repo's committed
# `.objectiveai` test root (a pointer to the pre-built
# `target/debug/objectiveai-cli` — `test-build.sh` builds it), and
# any servers the cli needs self-spawn behind lockfile singletons.
# `test-cleanup.sh` brackets the run unless the root test.sh owns
# the bracketing (OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT).
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
: > "$LOG_FILE"

if [ -z "${OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT:-}" ]; then
  # Standalone run: reset the shared test root and (re)build the shim
  # binaries, in parallel — cleanup kills every lockfile-owning
  # process first thing, so nothing is left running the binaries the
  # build may relink.
  bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 & _CLEANUP_PID=$!
  bash "$REPO_ROOT/test-build.sh" >>"$LOG_FILE" 2>&1 & _BUILD_PID=$!
  wait "$_CLEANUP_PID"
  wait "$_BUILD_PID"
  # Post-test cleanup is kill-only: processes die but `state/` survives
  # so the run's db can be re-spawned and inspected. (Pre-test cleanup
  # above ran without the env var, so it still wiped a stale tree.)
  trap 'OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY=1 bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 || true' EXIT INT TERM
fi

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

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
if "$NEXTEST" nextest run -p objectiveai-viewer --lib --tests "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
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
