#!/usr/bin/env bash
# test-sdk.sh — run the language-SDK test suites (go, py, js) in parallel.
#
# Each SDK owns its test.sh; this just invokes all three concurrently,
# capturing each one's output to
# .logs/tests/<sdk>-tests-<timestamp>.txt, waits for them, and
# aggregates exit codes: 0 iff all passed, 1 if any failed.
#
# These are the SDKs' offline UNIT tests (merge/push, schema roundtrip,
# coverage) — they need no server. The HTTP/snapshot tests that required a
# running server moved to the integration suite as standalone importer
# projects under tests/objectiveai-sdk-*-tests (run by test-integration.sh).
#
# Usage:
#   bash test-sdk.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/tests"
mkdir -p "$LOG_DIR"

# One timestamp for the whole run, so a run's logs sort together.
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"

SDKS=(objectiveai-sdk-go objectiveai-sdk-py objectiveai-sdk-js)

echo "test-sdk: running ${#SDKS[@]} SDK suite(s) -> $LOG_DIR"

# Launch one suite per SDK, all in parallel.
pids=()
pid_sdks=()
for sdk in "${SDKS[@]}"; do
  log="$LOG_DIR/${sdk}-tests-${TIMESTAMP}.txt"
  bash "$REPO_ROOT/$sdk/test.sh" >"$log" 2>&1 &
  pids+=("$!")
  pid_sdks+=("$sdk")
done

# Wait for all; aggregate exit codes (any failure -> overall failure).
failed=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then
    echo "test-sdk: ${pid_sdks[$i]}: PASS"
  else
    echo "test-sdk: ${pid_sdks[$i]}: FAIL"
    failed=1
  fi
done

exit "$failed"
