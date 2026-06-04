#!/usr/bin/env bash
# Spawns objectiveai-mcp on a random free port for testing.
#
# IMPORTANT: this is the *production* MCP server binary the CLI's
# `ConduitMcpHandler` is designed to talk to (not `objectiveai-mcp-filesystem`,
# which is a different crate that exposes raw filesystem tools).
#
# CONFIG_BASE_DIR is forced to the shared MCP-session scratch dir
# (`objectiveai-cli/.objectiveai-tests/_mcp_session`) so the spawned
# server never reads or writes the developer's real `~/.objectiveai`
# config. This matches `cli_test_util::mcp_session_shared_dir()` —
# the cli child invoked by `agents_continuation_tool_session_e2e`
# points its CONFIG_BASE_DIR at the same path so the two processes
# share one `tools/` registry.
#
# Prints "URL PID" to stdout once the server is ready, then exits.
# The server continues running as a background process.
# Caller is responsible for killing it.
#
# Usage:
#   read URL PID < <(bash test-spawn-mcp-server.sh)
#   OBJECTIVEAI_MCP_ADDRESS=$URL ...
#   kill $PID

set -euo pipefail

# Find a free port
get_free_port() {
  python3 -c "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()"
}

PORT=$(get_free_port)
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# Build the server binary and the fixture tool binary, then run from a
# copy so the originals are not locked. Windows locks running
# executables, which blocks cargo test from relinking.
cargo build --package objectiveai-mcp --package echo-arglen --quiet >&2
BINARY="$REPO_ROOT/target/debug/objectiveai-mcp"
if [ -f "$BINARY.exe" ]; then BINARY="$BINARY.exe"; fi
TMPDIR="$(mktemp -d)"
TMPBIN="$TMPDIR/$(basename "$BINARY")"
cp "$BINARY" "$TMPBIN"

# CONFIG_BASE_DIR is the shared MCP-session scratch dir — the same
# path `cli_test_util::mcp_session_shared_dir()` returns and the
# tool-session CLI tests stamp on every CLI subprocess. Sharing this
# between the CLI and the MCP server keeps the two views of config /
# plugins / authorization coherent within a single test run, and
# keeps the developer's real config OUT of the test process.
TEST_CONFIG_BASE_DIR="$REPO_ROOT/objectiveai-cli/.objectiveai-tests/_mcp_session"
mkdir -p "$TEST_CONFIG_BASE_DIR/tools"

# Lay down fixture tool manifests (tool0…tool9) backed by a single
# shared executable (`echo-arglen`). objectiveai-mcp reads these at
# startup via `filesystem::Client::list_tools` and exposes one MCP
# tool per discovered CLI tool — invoked as
# `objectiveai tools <name> <args>`. The vector-completion snapshot
# test's `json-schema-10x-tools` agent declares these exact names
# in `client_objectiveai_mcp.tools`; the agent-completions client's
# tool-validation step requires them present.
#
# `echo-arglen` prints one line: `args.len() + sum(s.len() for s in args)`.
ECHO_ARGLEN="$REPO_ROOT/target/debug/echo-arglen"
EXEC_NAME="echo-arglen"
if [ -f "$ECHO_ARGLEN.exe" ]; then
  ECHO_ARGLEN="$ECHO_ARGLEN.exe"
  EXEC_NAME="echo-arglen.exe"
fi
cp "$ECHO_ARGLEN" "$TEST_CONFIG_BASE_DIR/tools/$EXEC_NAME"
for i in $(seq 0 9); do
  printf '{"description":"Test fixture tool %d","version":"1.0.0","owner":"testorg","exec":"%s"}\n' \
    "$i" "$EXEC_NAME" \
    > "$TEST_CONFIG_BASE_DIR/tools/tool${i}.json"
done

ADDRESS=127.0.0.1 PORT="$PORT" SUPPRESS_OUTPUT=1 TEST_MODE=1 \
  CONFIG_BASE_DIR="$TEST_CONFIG_BASE_DIR" \
  "$TMPBIN" &
SERVER_PID=$!

# Wait for the server to accept connections (up to 60s)
TIMEOUT=60
ELAPSED=0
while ! (echo > /dev/tcp/127.0.0.1/"$PORT") 2>/dev/null; do
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "MCP server process died" >&2
    exit 1
  fi
  if [ "$ELAPSED" -ge "$TIMEOUT" ]; then
    echo "MCP server did not become ready within ${TIMEOUT}s" >&2
    kill "$SERVER_PID" 2>/dev/null
    exit 1
  fi
  sleep 0.1
  ELAPSED=$((ELAPSED + 1))
done

echo "http://127.0.0.1:$PORT $SERVER_PID"
