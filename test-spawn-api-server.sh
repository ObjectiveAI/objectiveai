#!/usr/bin/env bash
# Spawns the objectiveai-api server on a random free port for testing.
# MOCK_DELAY_MS=0 so tests run fast.
#
# Prints "PORT PID" to stdout once the server is ready, then exits.
# The server continues running as a background process.
# Caller is responsible for killing it.
#
# Usage:
#   read PORT PID < <(bash test-spawn-api-server)
#   OBJECTIVEAI_TEST_PORT=$PORT pytest tests/
#   kill $PID

set -euo pipefail

# Find a free port
get_free_port() {
  python3 -c "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()"
}

PORT=$(get_free_port)

# Build the server binary, then run from a copy so the original is not locked.
# Windows locks running executables, which blocks cargo test from relinking.
cargo build --package objectiveai-api --quiet >&2
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
BINARY="$REPO_ROOT/target/debug/objectiveai-api"
if [ -f "$BINARY.exe" ]; then BINARY="$BINARY.exe"; fi
TMPDIR="$(mktemp -d)"
TMPBIN="$TMPDIR/$(basename "$BINARY")"
cp "$BINARY" "$TMPBIN"
# Env-var rationale (mirrors what the api integration tests used to
# set per-binary in tests/common/server.rs before we hoisted server
# spawn into this script):
#
# - CLAUDE_AGENT_SDK_ENABLED / CODEX_SDK_ENABLED=false: disable the
#   subprocess upstreams. Tests only ever drive the mock upstream,
#   and those SDK clients try to find a `node` binary at startup
#   when enabled.
# - MOCK_DELAY_MS=0 / MOCK_MAX_TOOL_CALLS=1000: speed up mock paths.
# - MCP_*_TIMEOUT=1800000 (30 min): generous so slow CI doesn't time
#   out spuriously.
# - MCP_BACKOFF_*=0 / MCP_BACKOFF_MULTIPLIER=1: kill all retry/backoff
#   so a real first-try MCP failure surfaces instead of being masked
#   by silent retry storms.
# - AGENT_COMPLETIONS_FIRST/OTHER_CHUNK_TIMEOUT=1800000 + the matching
#   BACKOFF_*=0: same idea for the agent_completions path.
# - FUNCTIONS_INVENTIONS_SUBSCRIBE_TOOLS_TIMEOUT=300000 (5 min): bump
#   from the 30s default so contention-induced flakes during heavy
#   parallel `test.sh` loads don't trip the retry loop in
#   objectiveai-api/src/functions/inventions/client.rs:1346 (which
#   would append an extra `completion` block to the stream and
#   diverge from the snapshot).
ADDRESS=127.0.0.1 \
PORT="$PORT" \
CLAUDE_AGENT_SDK_ENABLED=false \
CODEX_SDK_ENABLED=false \
MOCK_DELAY_MS=0 \
MOCK_MAX_TOOL_CALLS=1000 \
MCP_CONNECT_TIMEOUT=1800000 \
MCP_CALL_TIMEOUT=1800000 \
MCP_BACKOFF_CURRENT_INTERVAL=0 \
MCP_BACKOFF_INITIAL_INTERVAL=0 \
MCP_BACKOFF_RANDOMIZATION_FACTOR=0 \
MCP_BACKOFF_MULTIPLIER=1 \
MCP_BACKOFF_MAX_INTERVAL=0 \
MCP_BACKOFF_MAX_ELAPSED_TIME=0 \
AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT=1800000 \
AGENT_COMPLETIONS_OTHER_CHUNK_TIMEOUT=1800000 \
AGENT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL=0 \
AGENT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL=0 \
AGENT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR=0 \
AGENT_COMPLETIONS_BACKOFF_MULTIPLIER=1 \
AGENT_COMPLETIONS_BACKOFF_MAX_INTERVAL=0 \
AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME=0 \
FUNCTIONS_INVENTIONS_SUBSCRIBE_TOOLS_TIMEOUT=300000 \
"$TMPBIN" &
SERVER_PID=$!

# Wait for the server to accept connections (up to 120s)
TIMEOUT=120
ELAPSED=0
while ! (echo > /dev/tcp/127.0.0.1/"$PORT") 2>/dev/null; do
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server process died" >&2
    exit 1
  fi
  if [ "$ELAPSED" -ge "$TIMEOUT" ]; then
    echo "Server did not become ready within ${TIMEOUT}s" >&2
    kill "$SERVER_PID" 2>/dev/null
    exit 1
  fi
  sleep 0.1
  ELAPSED=$((ELAPSED + 1))
done

echo "$PORT $SERVER_PID"
