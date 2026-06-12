#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-sdk-py.txt.
#
# Self-resolving server: runs `objectiveai api spawn` against the
# repo's committed .objectiveai test root (lockfile singleton — only
# one server ever materializes across every suite) and reads the
# published address. test-cleanup.sh brackets standalone runs.
#
# Usage:
#   bash objectiveai-sdk-py/test.sh                  # run all tests
#   bash objectiveai-sdk-py/test.sh -- -k mock_7 -vv # pass args to pytest

set -euo pipefail

MODULE="objectiveai-sdk-py"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Parse flags
PYTEST_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --)          shift; PYTEST_ARGS=("$@"); break ;;
    *)           PYTEST_ARGS+=("$1"); shift ;;
  esac
done

# Platform-independent venv path
if [ -d "$VENV_DIR/Scripts" ]; then
  PYTHON="$VENV_DIR/Scripts/python.exe"
else
  PYTHON="$VENV_DIR/bin/python"
fi

if [ -z "${OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT:-}" ]; then
  # Standalone run: reset the shared test root and (re)build the shim
  # binaries, in parallel — cleanup kills every lockfile-owning
  # process first thing, so nothing is left running the binaries the
  # build may relink. The trap reruns cleanup at the very end.
  bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 & _CLEANUP_PID=$!
  bash "$REPO_ROOT/test-build.sh" >>"$LOG_FILE" 2>&1 & _BUILD_PID=$!
  wait "$_CLEANUP_PID"
  wait "$_BUILD_PID"
  trap 'bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 || true' EXIT INT TERM
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
# pytest summary: "1060 passed, 31 skipped, 2 warnings in 17.66s" or "3 failed, 1057 passed ..."
parse_summary() {
  local summary
  summary=$(tail -1 "$LOG_FILE")
  PASSED=$(echo "$summary" | sed -n 's/.* \([0-9]*\) passed.*/\1/p')
  FAILED=$(echo "$summary" | sed -n 's/.* \([0-9]*\) failed.*/\1/p')
  TOTAL=$(( ${PASSED:-0} + ${FAILED:-0} ))
}

if "$PYTHON" -m pytest "$SCRIPT_DIR/tests/" -v --tb=long "${PYTEST_ARGS[@]}" >> "$LOG_FILE" 2>&1; then
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL ${PASSED:-0}/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
