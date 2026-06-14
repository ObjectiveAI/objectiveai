#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-js.txt.
#
# Self-resolving server: runs `objectiveai api spawn` against the
# repo's committed .objectiveai test root (lockfile singleton — only
# one server ever materializes across every suite) and reads the
# published address. test-cleanup.sh brackets standalone runs.
#
# Usage:
#   bash objectiveai-js/test.sh

set -euo pipefail

MODULE="objectiveai-js"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Reset the shared test root (kill lockfile owners + wipe state/) at
# start and end when running standalone; the root test.sh brackets
# the whole multi-suite run itself and tells us to skip via the env.
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

# Spawn-or-discover THE shared api server through the committed
# `.objectiveai` test root: `api spawn` is idempotent behind the api
# lockfile singleton — whoever asks first spawns it (the bin entry
# points at the pre-built target/debug binary from test-build.sh),
# everyone else gets the already-published URL back. The port feeds
# the suite's in-language gate var.
OAI_DIR="$REPO_ROOT/.objectiveai"
LISTENING=$(OBJECTIVEAI_DIR="$OAI_DIR" bash "$OAI_DIR/bin/objectiveai" api spawn 2>>"$LOG_FILE") || {
  echo "$MODULE: FATAL — objectiveai api spawn failed (see $LOG_FILE)" >&2
  exit 1
}
PORT=$(printf '%s' "$LISTENING" | python3 -c "import sys,json;print(json.loads(sys.stdin.readline())['listening'].rsplit(':',1)[1])")
export OBJECTIVEAI_TEST_PORT="$PORT"

# Run tests, capture all output
if pnpm --filter @objectiveai/sdk run test -- --reporter=verbose >> "$LOG_FILE" 2>&1; then
  # vitest summary: "Tests  959 passed | 6 todo (965)" or "Tests  3 failed | 959 passed | 6 todo (965)"
  # Strip ANSI codes; parse passed + failed, ignore todo/skipped
  CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG_FILE")
  PASSED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) passed.*/\1/p' | tail -1 || true)
  FAILED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) failed.*/\1/p' | tail -1 || true)
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  CLEAN=$(sed 's/\x1b\[[0-9;]*m//g' "$LOG_FILE")
  PASSED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) passed.*/\1/p' | tail -1 || true)
  FAILED=$(echo "$CLEAN" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) failed.*/\1/p' | tail -1 || true)
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
