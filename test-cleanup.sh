#!/usr/bin/env bash
# Reset the repo's shared test OBJECTIVEAI_DIR (.objectiveai/): kill
# every process owning ANY lockfile under it (api server, per-state
# db supervisors + their postmasters, viewers, mcp servers, leftover
# agent-holding clis), then delete the transient state/ tree.
#
# Every suite's test.sh runs this at start (in parallel with
# test-build.sh) and at the very end — unless
# OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT is set, in which case the root
# test.sh owns the bracketing and the inner scripts skip it.
#
# Runs the PRE-BUILT binary when it exists so the kill pass never
# queues behind a concurrent cargo build's target-dir lock (the
# start-of-run bracket runs cleanup and test-build.sh in parallel —
# leftover servers must die before the build relinks the binaries
# they're running). Falls back to cargo run only on a fresh clone
# where nothing is built yet (and nothing can be running either).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN="$REPO_ROOT/target/debug/test-cleanup"
if [ -x "$BIN" ] || [ -x "$BIN.exe" ]; then
  exec "$BIN" "$@"
fi
exec cargo run -q -p objectiveai-tests --bin test-cleanup \
  --manifest-path "$REPO_ROOT/Cargo.toml" -- "$@"
