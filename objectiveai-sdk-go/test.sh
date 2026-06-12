#!/usr/bin/env bash
# Spawns the API server (if needed) and runs tests.
# Output is captured to .logs/test/objectiveai-sdk-go.txt.
#
# Self-resolving server: runs `objectiveai api spawn` against the
# repo's committed .objectiveai test root (lockfile singleton — only
# one server ever materializes across every suite) and reads the
# published address. test-cleanup.sh brackets standalone runs.
#
# Usage:
#   bash objectiveai-sdk-go/test.sh
#   bash objectiveai-sdk-go/test.sh -- -run TestRoundtrip   # pass args to go test

set -euo pipefail

MODULE="objectiveai-sdk-go"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
LOG_FILE="$LOG_DIR/$MODULE.txt"

mkdir -p "$LOG_DIR"
> "$LOG_FILE"

# Parse flags
GO_TEST_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift; GO_TEST_ARGS=("$@"); break ;;
    *)  GO_TEST_ARGS+=("$1"); shift ;;
  esac
done

# Reset the shared test root (kill lockfile owners + wipe state/) at
# start and end when running standalone; the root test.sh brackets
# the whole multi-suite run itself and tells us to skip via the env.
if [ -z "${OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT:-}" ]; then
  bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1
  trap 'bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 || true' EXIT INT TERM
fi

# Spawn-or-discover THE shared api server through the committed
# `.objectiveai` test root: `api spawn` is idempotent behind the api
# lockfile singleton — whoever asks first spawns it (the bin entry is
# a cargo-run shim, so the server reflects the working tree),
# everyone else gets the already-published URL back. The port feeds
# the suite's in-language gate var.
OAI_DIR="$REPO_ROOT/.objectiveai"
LISTENING=$(OBJECTIVEAI_DIR="$OAI_DIR" bash "$OAI_DIR/bin/objectiveai" api spawn 2>>"$LOG_FILE") || {
  echo "$MODULE: FATAL — objectiveai api spawn failed (see $LOG_FILE)" >&2
  exit 1
}
PORT=$(printf '%s' "$LISTENING" | python3 -c "import sys,json;print(json.loads(sys.stdin.readline())['listening'].rsplit(':',1)[1])")
export OBJECTIVEAI_TEST_PORT="$PORT"

# go test -v output:  --- PASS: TestFoo (0.01s)  /  --- FAIL: TestBar (0.02s)
parse_summary() {
  PASSED=$(grep -c '^--- PASS:' "$LOG_FILE" || true)
  FAILED=$(grep -c '^--- FAIL:' "$LOG_FILE" || true)
  TOTAL=$((PASSED + FAILED))
}

# Run tests across both packages, capture all output
if (cd "$SCRIPT_DIR" && go test ./tests/ ./ -v -count=1 "${GO_TEST_ARGS[@]}") >> "$LOG_FILE" 2>&1; then
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: PASS $PASSED/$TOTAL"
  else
    echo "$MODULE: PASS"
  fi
else
  parse_summary
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
