#!/usr/bin/env bash
# Reset the repo's shared test OBJECTIVEAI_DIR (.objectiveai/): kill
# every process owning ANY lockfile under it (api server, per-state
# db supervisors + their postmasters, viewers, mcp servers, leftover
# agent-holding clis), then delete the transient state/ tree.
#
# Every suite's test.sh runs this at start and end — unless
# OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT is set, in which case the root
# test.sh owns the bracketing and the inner scripts skip it.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
exec cargo run -q -p objectiveai-tests --bin test-cleanup \
  --manifest-path "$REPO_ROOT/Cargo.toml" -- "$@"
