#!/usr/bin/env bash
# Runs objectiveai-cli tests.
# Output is captured to .logs/test/objectiveai-cli.txt.
#
# Usage:
#   bash objectiveai-cli/test.sh
#   bash objectiveai-cli/test.sh -- --test-threads=1   # pass args to nextest

set -euo pipefail

MODULE="objectiveai-cli"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

mkdir -p "$LOG_DIR"

# Deterministically wipe CLI test artifacts on every exit path (success,
# failure, or interrupt). Keeps the tests folder free of gitignored runtime
# state (logs, cached repos, filesystem config) between runs.
#
# Also reap the api server we spawned, if any. CLI_TEST_API_PID is set
# below only when this script spawned the server itself — under the
# root test.sh harness OBJECTIVEAI_TEST_PORT is pre-set and we leave
# the parent's server alone.
CLI_TESTS_SCRATCH="$SCRIPT_DIR/tests/.objectiveai"
cleanup() {
  rm -rf "$CLI_TESTS_SCRATCH"
  if [ -n "${CLI_TEST_API_PID:-}" ]; then
    kill "$CLI_TEST_API_PID" 2>/dev/null || true
    wait "$CLI_TEST_API_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM
cleanup  # start from a clean slate as well

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# Spawn the test api server only if not already provided by a parent
# harness (e.g. the root test.sh). Same pattern as objectiveai-sdk-{py,js,go}.
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  read -r PORT CLI_TEST_API_PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>>"$LOG_FILE")
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

# Seed the shared CLI tool-fixtures registry the integration tests
# rely on. The conduit dials its embedded `objectiveai-mcp` (since the
# in-process refactor) so no standalone server is spawned — but the
# on-disk `tools/` registry is still consumed by
# `filesystem::Client::list_tools` inside every cli child.
bash "$REPO_ROOT/test-seed-tool-fixtures.sh" 2>>"$LOG_FILE"

# Run tests, capture all output.
#
# nextest parallelizes across test binaries (cargo test serialises one
# binary at a time); each `tests/*.rs` integration test gets its own
# binary, so this is a large wall-clock win for the cli's e2e suite.
# Per-test isolation contract is unchanged: every test still gets its
# own `CONFIG_BASE_DIR` via `cli_test_util::test_base_dir`. cargo-nextest
# is installed locally by `build-bin.sh` into `bin/` — see the
# [workspace.metadata.tools] table in the root Cargo.toml.
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
