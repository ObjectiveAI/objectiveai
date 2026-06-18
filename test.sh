#!/usr/bin/env bash
# Runs all test suites in parallel against the repo's committed
# shared test root (.objectiveai/). No server orchestration: suites
# that need the api run `objectiveai api spawn` themselves, and the
# api lockfile singleton guarantees exactly one server materializes
# no matter how many suites ask. This script owns the bracketing —
# at start it resets the shared root (kill lockfile owners + wipe
# state/) via test-cleanup.sh while test-build.sh builds the five
# shim-target binaries in parallel, at the very end it always runs
# test-cleanup.sh again, and it tells the inner scripts to skip
# their own bracketing through OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT.
#
# Usage:
#   bash test.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$REPO_ROOT/.logs/test"
CLEANUP_LOG="$LOG_DIR/test-cleanup.txt"
BUILD_LOG="$LOG_DIR/test-build.txt"

mkdir -p "$LOG_DIR"
: > "$CLEANUP_LOG"
: > "$BUILD_LOG"

export OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT=1

# Reset the shared root and build the shim binaries, in parallel.
# Cleanup kills every lockfile-owning process first thing, so nothing
# is left running the binaries the build is about to relink.
bash "$REPO_ROOT/test-cleanup.sh" >>"$CLEANUP_LOG" 2>&1 & CLEANUP_PID=$!
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --workspace --bins >>"$BUILD_LOG" 2>&1 & BUILD_PID=$!
wait "$CLEANUP_PID"
wait "$BUILD_PID"

# Pre-build every suite's TEST binaries up front — BEFORE the parallel
# test phase spawns any server — so no suite relinks a running `.exe`
# (the Windows "Access is denied" relink race). test-build.sh above
# builds the shim bins; this builds the per-suite test binaries cargo
# would otherwise relink during the parallel phase. Sequential: cargo's
# target-dir build lock serializes them anyway. Each build-tests.sh
# runs the suite's `nextest list`, so the later `nextest run` is a pure
# cache hit. A build failure here aborts before the test phase (set -e).
for suite in \
  objectiveai-sdk-rs \
  objectiveai-api \
  objectiveai-json-schema \
  objectiveai-cli \
  objectiveai-viewer \
  objectiveai-tests \
; do
  bash "$REPO_ROOT/$suite/build-tests.sh" >>"$BUILD_LOG" 2>&1
done

# The root ALWAYS reruns test-cleanup.sh at the very end, exactly
# once — the EXIT trap covers success, failure, and interruption.
# It's kill-only: every lockfile-owning process dies but `state/`
# survives, so a failed run's db can be re-spawned and its logs read
# back with the cli (`OBJECTIVEAI_STATE=<test-fn> objectiveai agents
# instances list` / `agents logs read`). The START cleanup above ran
# WITHOUT the env var, so it still wiped a stale tree for a clean
# slate. Mirrors the per-suite test.sh bracketing.
FINAL_CLEANUP_DONE=false
final_cleanup() {
  $FINAL_CLEANUP_DONE && return 0
  FINAL_CLEANUP_DONE=true
  OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY=1 \
    bash "$REPO_ROOT/test-cleanup.sh" >>"$CLEANUP_LOG" 2>&1 || true
}
trap final_cleanup EXIT INT TERM

# Run all test suites in parallel.
# Each script prints exactly one line: "$MODULE: PASS N/N" or "$MODULE: FAIL N/N".
PIDS=()
for suite in \
  objectiveai-sdk-rs \
  objectiveai-api \
  objectiveai-json-schema \
  objectiveai-cli \
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
