#!/usr/bin/env bash
# Runs objectiveai-viewer tests.
# Output is captured to .logs/test/objectiveai-viewer.txt.
#
# The viewer's `cli_command` integration test needs the workspace's
# API server. If OBJECTIVEAI_TEST_PORT is already set (e.g. the root
# test.sh shares one across suites) this script inherits it; otherwise
# it spawns its own via test-spawn-api-server.sh and reaps it on exit
# (kill+wait, resilient to Ctrl-C / SIGTERM). Either way the
# integration test runs.
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

# Spawn ONE test api server only if a parent harness hasn't already
# provided OBJECTIVEAI_TEST_PORT (the root test.sh shares one across
# all suites). Capture the pid so the cleanup trap can reap it on exit.
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT SERVER_PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE") || {
    echo "$MODULE: FATAL — failed to spawn API server (see $LOG_FILE)" >&2
    exit 1
  }
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

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
