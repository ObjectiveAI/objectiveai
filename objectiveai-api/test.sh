#!/usr/bin/env bash
# Runs objectiveai-api tests.
# Output is captured to .logs/test/objectiveai-api.txt.
#
# Spawns a single shared api server (via test-spawn-api-server.sh)
# and exports OBJECTIVEAI_TEST_PORT. The integration tests
# (tests/common/server.rs) read that env var instead of spawning a
# server per cargo test binary — avoids the parallel-spawn
# ConnectionRefused flakes that hit the agent_completions cluster
# under heavy load.
#
# If OBJECTIVEAI_TEST_PORT is already set (e.g. the root test.sh
# shares one server across all suites), inherit it.
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
: > "$LOG_FILE"

# Tear-down: clean up the api-spawn port file only.
#
# We do NOT kill the spawned api server here — the root test.sh and
# the api server itself are responsible for that. Leaked processes
# at script exit are a harness bug to fix, not paper over.
PORT_FILE=""
cleanup() {
  if [ -n "$PORT_FILE" ]; then
    rm -f "$PORT_FILE"
  fi
}
trap cleanup EXIT INT TERM

# If UPDATE_SNAPSHOTS is set, propagate to all 6 snapshot env vars.
if [ "${UPDATE_SNAPSHOTS:-}" = "1" ]; then
  export UPDATE_AGENT_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_AGENT_COMPLETIONS_MOCK_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS=1
  export UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS=1
fi

# Parse flags
CARGO_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; CARGO_ARGS=("$@"); break ;;
    *)  CARGO_ARGS+=("$1"); shift ;;
  esac
done

# Spawn the shared test api server — unless a parent harness has
# already provided OBJECTIVEAI_TEST_PORT (e.g. the root test.sh
# shares one server across all suites).
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  PORT_FILE="$(mktemp)"
  bash "$REPO_ROOT/test-spawn-api-server.sh" > "$PORT_FILE" 2>>"$LOG_FILE" &
  SPAWN_PID=$!
  wait "$SPAWN_PID"
  # test-spawn-api-server.sh prints "<port> <pid>"; we only need the
  # port. The api server keeps running in the background and is
  # expected to clean itself up (or be reaped by a parent harness).
  read -r PORT _ < "$PORT_FILE"
  rm -f "$PORT_FILE"
  PORT_FILE=""
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml. Output is appended to $LOG_FILE so any
# api-spawn errors that landed there survive.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" --no-fail-fast "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
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
