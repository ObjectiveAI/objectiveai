#!/usr/bin/env bash
# Runs objectiveai-cli tests.
# Output is captured to .logs/test/objectiveai-cli.txt.
#
# No preparation step: tests run against the repo's committed shared
# test root (`.objectiveai/`). The cli under test is the shim at
# `.objectiveai/bin/objectiveai` (a pointer to the pre-built
# `target/debug/objectiveai-cli` — `test-build.sh` builds it),
# fixtures are committed manifests whose exec entries cargo-run their
# crates in place, and servers (api/db) self-spawn behind lockfile
# singletons when a test first needs them. `test-cleanup.sh` brackets
# the run — killing every lockfile-owning process under
# `.objectiveai` and wiping the transient `state/` tree — unless the
# root `test.sh` owns the bracketing
# (OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT).
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
  # Post-test cleanup is kill-only: processes die but `state/` survives
  # so the run's db can be re-spawned and inspected. (Pre-test cleanup
  # above ran without the env var, so it still wiped a stale tree.)
  trap 'OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY=1 bash "$REPO_ROOT/test-cleanup.sh" >>"$LOG_FILE" 2>&1 || true' EXIT INT TERM
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
if "$NEXTEST" nextest run --manifest-path "$SCRIPT_DIR/Cargo.toml" "${CARGO_ARGS[@]}" >>"$LOG_FILE" 2>&1; then
  NEXTEST_OK=1
else
  NEXTEST_OK=0
fi

# Parse counts from nextest's own summary line ONLY, e.g.
#   "Summary [ 72.9s] 146 tests run: 146 passed (16 leaky), 0 skipped"
#   "Summary [ ... ] 148 tests run: 146 passed, 2 failed, 0 skipped"
# Anchoring on the "N tests run:" line is essential: the rest of
# $LOG_FILE carries cargo build output that also contains the words
# "passed"/"failed" and would otherwise be miscounted as test
# results. `tail -1` takes the final run's summary; `|| true` keeps
# `set -o pipefail` from aborting when grep finds no match.
SUMMARY_LINE="$(grep -E '[0-9]+ tests run:' "$LOG_FILE" | tail -1 || true)"
PASSED=$(printf '%s\n' "$SUMMARY_LINE" | sed -n 's/.* \([0-9][0-9]*\) passed.*/\1/p')
FAILED=$(printf '%s\n' "$SUMMARY_LINE" | sed -n 's/.* \([0-9][0-9]*\) failed.*/\1/p')
PASSED=${PASSED:-0}
FAILED=${FAILED:-0}
TOTAL=$((PASSED + FAILED))

if [ "$NEXTEST_OK" -eq 1 ]; then
  echo "$MODULE: PASS $PASSED/$TOTAL"
else
  if [ "$TOTAL" -gt 0 ]; then
    echo "$MODULE: FAIL $PASSED/$TOTAL"
  else
    echo "$MODULE: FAIL"
  fi
  exit 1
fi
