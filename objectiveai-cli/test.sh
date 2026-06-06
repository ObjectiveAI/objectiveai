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
RUNTIME_DIR="$SCRIPT_DIR/.objectiveai-tests"

mkdir -p "$LOG_DIR"
: > "$LOG_FILE"

# Tear-down: wipe the runtime staging dir, kill anything we spawned.
# Runs on every exit path (success, failure, interrupt). We bail any
# half-finished prepare-step or api-spawn child before deleting the
# runtime dir so neither holds a handle open across the cleanup.
PREP_PID=""
SPAWN_PID=""
CLI_TEST_API_PID=""
PORT_FILE=""
cleanup() {
  if [ -n "$PREP_PID" ]; then
    kill "$PREP_PID" 2>/dev/null || true
    wait "$PREP_PID" 2>/dev/null || true
  fi
  if [ -n "$SPAWN_PID" ]; then
    kill "$SPAWN_PID" 2>/dev/null || true
    wait "$SPAWN_PID" 2>/dev/null || true
  fi
  if [ -n "$PORT_FILE" ]; then
    rm -f "$PORT_FILE"
  fi
  rm -rf "$RUNTIME_DIR"
  if [ -n "$CLI_TEST_API_PID" ]; then
    kill "$CLI_TEST_API_PID" 2>/dev/null || true
    wait "$CLI_TEST_API_PID" 2>/dev/null || true
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

# Stage the runtime test tree: a fresh copy of the committed
# `objectiveai-tests/` source. The copied tree includes a self-
# removing `prepare.sh` which cargo-builds the cli + every fixture
# crate and slots binaries into the right per-test directories.
rm -rf "$RUNTIME_DIR"
cp -R "$SCRIPT_DIR/objectiveai-tests" "$RUNTIME_DIR"
bash "$RUNTIME_DIR/prepare.sh" >>"$LOG_FILE" 2>&1 &
PREP_PID=$!

# Spawn the test api server in parallel — unless a parent harness has
# already provided `OBJECTIVEAI_TEST_PORT` (e.g. the root `test.sh`
# shares one server across all suites).
if [ -z "${OBJECTIVEAI_TEST_PORT:-}" ]; then
  PORT_FILE="$(mktemp)"
  bash "$REPO_ROOT/test-spawn-api-server.sh" > "$PORT_FILE" 2>>"$LOG_FILE" &
  SPAWN_PID=$!
  wait "$SPAWN_PID"
  SPAWN_PID=""
  read -r PORT CLI_TEST_API_PID < "$PORT_FILE"
  rm -f "$PORT_FILE"
  PORT_FILE=""
  export OBJECTIVEAI_TEST_PORT="$PORT"
fi

wait "$PREP_PID"
PREP_PID=""

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml. Output is appended to $LOG_FILE so any
# prepare-step or api-spawn errors that landed there survive.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
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
