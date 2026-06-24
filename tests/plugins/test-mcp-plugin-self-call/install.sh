#!/usr/bin/env bash
# Install the test-mcp-plugin-self-call fixture into the shared test
# OBJECTIVEAI_DIR. Expects the binary to be already built (build.sh, or
# `cargo build -p test-mcp-plugin-self-call`). Copies it into the
# coordinate's cli/ dir and writes the matching objectiveai.json with
# four mcp_servers (one per test surface).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

BIN="test-mcp-plugin-self-call"
case "$(uname -s)" in CYGWIN*|MINGW*|MSYS*) EXE=".exe" ;; *) EXE="" ;; esac
if [ -n "${OBJECTIVEAI_BUILD_RELEASE:-}" ] && [ "${OBJECTIVEAI_BUILD_RELEASE}" != "0" ]; then
  PROFILE="release"
else
  PROFILE="debug"
fi
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

SRC="$REPO_ROOT/target/$PROFILE/$BIN$EXE"
[ -f "$SRC" ] || { echo "install: binary not built: $SRC (build it first)" >&2; exit 1; }

VDIR="$OBJECTIVEAI_DIR/bin/plugins/testorg/test-mcp-plugin-self-call/1.0.0"
mkdir -p "$VDIR/cli"
cp -f "$SRC" "$VDIR/cli/$BIN$EXE"
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "testorg",
  "name": "test-mcp-plugin-self-call",
  "version": "1.0.0",
  "description": "test fixture",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {},
  "mcp_servers": [
    {"name":"call-other","authorization":false},
    {"name":"list-tools","authorization":false},
    {"name":"list-resources","authorization":false},
    {"name":"read-resource","authorization":false}
  ]
}
JSON
echo "install: plugins/testorg/test-mcp-plugin-self-call/1.0.0 (${BIN}${EXE}, ${PROFILE})"
