#!/usr/bin/env bash
# Reset the repo's shared test OBJECTIVEAI_DIR (.objectiveai/): kill
# every process owning ANY lockfile under it (api server, per-state
# db supervisors + their postmasters, viewers, mcp servers, leftover
# agent-holding clis), sweep leaked repo-built processes, then delete
# the transient state/ tree. The work lives in the
# `objectiveai-test-cleanup` crate (objectiveai-tests/test-cleanup);
# this script just invokes its PRE-BUILT binary — `test-build.sh`
# compiles it, and keeping cargo out of here means the kill pass
# never queues behind a concurrent build's target-dir lock.
#
# Every suite's test.sh runs this at start (in parallel with
# test-build.sh) and at the very end — unless
# OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT is set, in which case the root
# test.sh owns the bracketing and the inner scripts skip it.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN="$REPO_ROOT/target/debug/objectiveai-test-cleanup"
if [ -x "$BIN" ] || [ -x "$BIN.exe" ]; then
  exec "$BIN" "$@"
fi
# Not built yet = fresh tree: nothing can be running and there is no
# transient state to wipe. test-build.sh (running in parallel in the
# same bracket) builds the binary for every later invocation.
echo "test-cleanup: $BIN not built (fresh tree); nothing to do"
