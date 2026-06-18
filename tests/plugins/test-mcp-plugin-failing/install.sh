#!/usr/bin/env bash
# Install the test-mcp-plugin-failing fixture into the shared test
# OBJECTIVEAI_DIR.
#
# Expects the binary to be already built (build.sh, or `cargo build -p
# test-mcp-plugin-failing`). Copies it into the coordinate's cli/ dir and
# writes the matching objectiveai.json. Self-contained on purpose — each
# fixture's installer is independent so it can diverge as needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

BIN="test-mcp-plugin-failing"
case "$(uname -s)" in CYGWIN*|MINGW*|MSYS*) EXE=".exe" ;; *) EXE="" ;; esac
# Profile mirrors build.sh: --release exports OBJECTIVEAI_BUILD_RELEASE=1.
if [ -n "${OBJECTIVEAI_BUILD_RELEASE:-}" ] && [ "${OBJECTIVEAI_BUILD_RELEASE}" != "0" ]; then
  PROFILE="release"
else
  PROFILE="debug"
fi
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

SRC="$REPO_ROOT/target/$PROFILE/$BIN$EXE"
[ -f "$SRC" ] || { echo "install: binary not built: $SRC (build it first)" >&2; exit 1; }

VDIR="$OBJECTIVEAI_DIR/bin/plugins/testorg/test-mcp-plugin-failing/1.0.0"
mkdir -p "$VDIR/cli"
cp -f "$SRC" "$VDIR/cli/$BIN$EXE"
# exec uses a leading ./ so it resolves against the cli/ CWD at run time
# rather than being PATH-looked-up as a bare name.
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "testorg",
  "name": "test-mcp-plugin-failing",
  "version": "1.0.0",
  "description": "test fixture: RMCP plugin that fails at a chosen stage (connect/list/call)",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {},
  "mcp_servers": [{"name":"demo","authorization":false}]
}
JSON
echo "install: plugins/testorg/test-mcp-plugin-failing/1.0.0 (${BIN}${EXE}, ${PROFILE})"
