#!/usr/bin/env bash
# Runs objectiveai-api tests.
# Output is captured to .logs/test/objectiveai-api.txt.
#
# No server lifecycle here: the integration tests
# (tests/common/server.rs) run `objectiveai api spawn` against the
# repo's committed `.objectiveai` test root themselves — the api
# lockfile singleton guarantees exactly one server across every
# suite, and the URL is read from the lockfile. `test-cleanup.sh`
# brackets the run (kills lockfile owners + wipes state/) unless the
# root test.sh owns the bracketing.
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

# If UPDATE_SNAPSHOTS is set, propagate to all snapshot env vars.
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

# Run tests, capture all output. cargo-nextest is installed locally by
# `build-bin.sh` into `bin/` — see the [workspace.metadata.tools] table
# in the root Cargo.toml.
#
# --lib --tests: skip the `objectiveai-api` bin target. The server is
# launched from the PRE-BUILT target/debug/objectiveai-api.exe (built by
# test-build.sh, spawned via the `.objectiveai/bin` shim behind the api
# lockfile singleton) — the suite never needs to relink it. Rebuilding
# the bin here races with a sibling suite that has already spawned the
# server: cargo can't remove the running .exe to relink it
# ("failed to remove file ... objectiveai-api.exe: Access is denied").
# All api test coverage lives in the lib unit tests + integration tests
# (tests/); main.rs carries none. Mirrors objectiveai-viewer/test.sh.
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" --lib --tests --no-fail-fast "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
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
