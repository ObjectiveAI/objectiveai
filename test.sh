#!/usr/bin/env bash
# Runs all test suites in parallel.
# Spawns a shared API server for suites that need one.
#
# Usage:
#   bash test.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
SERVER_LOG="$LOG_DIR/server.txt"

mkdir -p "$LOG_DIR"

# Spawn shared API server, capturing all output
SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

read -r PORT SERVER_PID < <(bash "$REPO_ROOT/test-spawn-api-server.sh" 2>"$SERVER_LOG") || {
  echo "Failed to spawn API server. Log:" >&2
  cat "$SERVER_LOG" >&2
  exit 1
}
export OBJECTIVEAI_TEST_PORT="$PORT"

# Run all test suites in parallel.
# Each script prints exactly one line: "$MODULE: PASS N/N" or "$MODULE: FAIL N/N".
PIDS=()
for suite in \
  objectiveai-sdk-rs \
  objectiveai-api \
  objectiveai-json-schema \
  objectiveai-cli \
  objectiveai-mcp-proxy \
  objectiveai-sdk-js \
  objectiveai-sdk-py \
  objectiveai-sdk-go \
  objectiveai-viewer \
  objectiveai-tests \
; do
  bash "$REPO_ROOT/$suite/test.sh" &
  PIDS+=($!)
done

# Wait for all suites, collect exit codes
FAILED=false
for pid in "${PIDS[@]}"; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi
