#!/usr/bin/env bash
# Seeds the shared CLI tool-fixtures registry the CLI integration tests
# rely on:
#
#   objectiveai-cli/.objectiveai-tests/_mcp_session/tools/
#     ├── echo-arglen{,.exe}        # fixture executable
#     ├── tool0.json … tool9.json   # ten manifests pointing at it
#
# After the in-process `objectiveai-mcp` refactor the conduit dials its
# own embedded server, so a standalone `objectiveai-mcp` process is no
# longer needed — but the on-disk tool registry still is. Test agents
# that declare `client_objectiveai_mcp.tools = ["testorg/tool0/1.0.0",
# …]` resolve those names against this directory, and the cli child
# subprocess discovers them via `filesystem::Client::list_tools`.
#
# Idempotent: re-running overwrites the binary copy and the manifests.
#
# Usage:
#   bash test-seed-tool-fixtures.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# Build the fixture executable. `echo-arglen` prints one line:
# `args.len() + sum(s.len() for s in args)`. Cheap to build; produced
# by its own crate at `objectiveai-cli/tests/fixtures/echo-arglen/`.
cargo build --package echo-arglen --quiet >&2

# Shared MCP-session scratch dir — must match
# `cli_test_util::mcp_session_shared_dir()` on the Rust side.
TEST_CONFIG_BASE_DIR="$REPO_ROOT/objectiveai-cli/.objectiveai-tests/_mcp_session"
mkdir -p "$TEST_CONFIG_BASE_DIR/tools"

ECHO_ARGLEN="$REPO_ROOT/target/debug/echo-arglen"
EXEC_NAME="echo-arglen"
if [ -f "$ECHO_ARGLEN.exe" ]; then
  ECHO_ARGLEN="$ECHO_ARGLEN.exe"
  EXEC_NAME="echo-arglen.exe"
fi
cp "$ECHO_ARGLEN" "$TEST_CONFIG_BASE_DIR/tools/$EXEC_NAME"

# Lay down `testorg/tool{0..9}/1.0.0` manifests, each pointing at the
# shared `echo-arglen` exec. Agents declaring these names in
# `client_objectiveai_mcp.tools` resolve through here.
for i in $(seq 0 9); do
  printf '{"description":"Test fixture tool %d","version":"1.0.0","owner":"testorg","exec":"%s"}\n' \
    "$i" "$EXEC_NAME" \
    > "$TEST_CONFIG_BASE_DIR/tools/tool${i}.json"
done
