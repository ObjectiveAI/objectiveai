#!/usr/bin/env bash
# Builds the cli + every test fixture binary and slots each into the
# right per-test directory under this folder. Removes itself on
# success. Multi-platform: the `.exe` suffix is detected from the
# produced cli binary, not from $OSTYPE heuristics.
#
# Run once per checkout — the deposited binaries are gitignored and
# the script itself self-removes from the working tree (the committed
# copy reappears on the next `git checkout` / `git pull`).
#
# Path layout is computed from the script's own location, so this
# works regardless of the caller's working directory.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"             # objectiveai-tests/
REPO_ROOT="$(cd "$ROOT/../.." && pwd)"             # workspace root
TARGET_DIR="$REPO_ROOT/target/objectiveai-tests"   # shared target dir

# Two concurrent cargo invocations. Cargo serialises them on the
# shared target-dir lock, but each one parallelises rustc workers
# internally and shares dep compilation through the same target.
(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p objectiveai-cli \
    --no-default-features --features rustpython \
    --target-dir "$TARGET_DIR") &
PID_CLI=$!

(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p hello-tool -p error-tool -p count-tool \
    -p hello-plugin -p test-mcp-plugin \
    -p test-mcp-plugin-named -p test-mcp-plugin-foo-headers \
    --target-dir "$TARGET_DIR") &
PID_FIX=$!

wait "$PID_CLI" "$PID_FIX"

BIN_DIR="$TARGET_DIR/debug"
if [ -f "$BIN_DIR/objectiveai-cli.exe" ]; then EXE=".exe"; else EXE=""; fi

slot() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

slot "$BIN_DIR/objectiveai-cli$EXE"               "$ROOT/objectiveai-cli$EXE" &
slot "$BIN_DIR/test-mcp-plugin$EXE"               "$ROOT/plugin_mcp_dispatch_round_trip/plugins/test-mcp-plugin/plugin$EXE" &
slot "$BIN_DIR/hello-plugin$EXE"                  "$ROOT/hello_plugin_dispatch_produces_expected_output/plugins/hello/plugin$EXE" &
slot "$BIN_DIR/hello-tool$EXE"                    "$ROOT/hello_tool_dispatch_snapshot/tools/hello-tool$EXE" &
slot "$BIN_DIR/error-tool$EXE"                    "$ROOT/error_tool_dispatch_snapshot/tools/error-tool$EXE" &
slot "$BIN_DIR/test-mcp-plugin-foo-headers$EXE"   "$ROOT/function_swarm_writes_per_agent_files/plugins/test-mcp-plugin-foo-headers/plugin$EXE" &
slot "$BIN_DIR/count-tool$EXE"                    "$ROOT/two_agents_continuations_count_persists_per_session/tools/count-tool$EXE" &

for name in dup-alpha dup-bravo dup-charlie dup-delta dup-echo; do
  slot "$BIN_DIR/test-mcp-plugin-named$EXE" \
       "$ROOT/duplicate_tool_names_routed_across_turns/plugins/$name/plugin$EXE" &
done

wait

rm -- "$0"
