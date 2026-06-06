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
# Dedicated target dir for the cli binary only. The cli build uses
# `--no-default-features --features rustpython`, which produces a
# differently-featured `objectiveai-cli` artifact than the default
# `target/debug` (which nextest populates when it compiles the
# integration tests linking against `objectiveai-cli` as a lib).
# Co-locating those two builds would force-rebuild the cli lib on
# every flip between prepare.sh and nextest, so the cli build keeps
# its own slot. Sub-crates (SDK, mcp, fixtures) compile under
# default features and CAN share the workspace target.
CLI_TARGET_DIR="$REPO_ROOT/target/objectiveai-tests"

# Two concurrent cargo invocations:
#   - The cli build uses CLI_TARGET_DIR so the rustpython-featured
#     artifact stays isolated from nextest's default-featured one.
#   - The fixture builds use the workspace's default target/, so the
#     shared dep tree (objectiveai-sdk, objectiveai-mcp, etc.) is
#     compiled exactly once and reused by nextest.
(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p objectiveai-cli \
    --no-default-features --features rustpython \
    --target-dir "$CLI_TARGET_DIR") &
PID_CLI=$!

(cargo build \
    --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p hello-tool -p error-tool -p count-tool \
    -p hello-plugin -p test-mcp-plugin \
    -p test-mcp-plugin-named -p test-mcp-plugin-foo-headers) &
PID_FIX=$!

wait "$PID_CLI" "$PID_FIX"

CLI_BIN_DIR="$CLI_TARGET_DIR/debug"
FIX_BIN_DIR="$REPO_ROOT/target/debug"
if [ -f "$CLI_BIN_DIR/objectiveai-cli.exe" ]; then EXE=".exe"; else EXE=""; fi

slot() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

slot "$CLI_BIN_DIR/objectiveai-cli$EXE"               "$ROOT/objectiveai-cli$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin$EXE"               "$ROOT/plugin_mcp_dispatch_round_trip/plugins/test-mcp-plugin/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-plugin$EXE"                  "$ROOT/hello_plugin_dispatch_produces_expected_output/plugins/hello/plugin$EXE" &
slot "$FIX_BIN_DIR/hello-tool$EXE"                    "$ROOT/hello_tool_dispatch_snapshot/tools/hello-tool$EXE" &
slot "$FIX_BIN_DIR/error-tool$EXE"                    "$ROOT/error_tool_dispatch_snapshot/tools/error-tool$EXE" &
slot "$FIX_BIN_DIR/test-mcp-plugin-foo-headers$EXE"   "$ROOT/function_swarm_writes_per_agent_files/plugins/test-mcp-plugin-foo-headers/plugin$EXE" &
slot "$FIX_BIN_DIR/count-tool$EXE"                    "$ROOT/two_agents_continuations_count_persists_per_session/tools/count-tool$EXE" &

for name in dup-alpha dup-bravo dup-charlie dup-delta dup-echo; do
  slot "$FIX_BIN_DIR/test-mcp-plugin-named$EXE" \
       "$ROOT/duplicate_tool_names_routed_across_turns/plugins/$name/plugin$EXE" &
done

wait

rm -- "$0"
