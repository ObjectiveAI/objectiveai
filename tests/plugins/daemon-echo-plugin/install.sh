#!/usr/bin/env bash
# Install the daemon-echo-plugin fixture (daemon: true) into the shared
# test OBJECTIVEAI_DIR. Expects the binary to be already built (build.sh,
# or `cargo build -p daemon-echo-plugin`).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

BIN="daemon-echo-plugin"
case "$(uname -s)" in CYGWIN*|MINGW*|MSYS*) EXE=".exe" ;; *) EXE="" ;; esac
if [ -n "${OBJECTIVEAI_BUILD_RELEASE:-}" ] && [ "${OBJECTIVEAI_BUILD_RELEASE}" != "0" ]; then
  PROFILE="release"
else
  PROFILE="debug"
fi
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

SRC="$REPO_ROOT/target/$PROFILE/$BIN$EXE"
[ -f "$SRC" ] || { echo "install: binary not built: $SRC (build it first)" >&2; exit 1; }

VDIR="$OBJECTIVEAI_DIR/bin/plugins/objectiveai/daemon-echo/0.0.1"
mkdir -p "$VDIR/cli"
cp -f "$SRC" "$VDIR/cli/$BIN$EXE"
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "objectiveai",
  "name": "daemon-echo",
  "version": "0.0.1",
  "description": "E2E daemon test fixture",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {},
  "daemon": true
}
JSON
echo "install: plugins/objectiveai/daemon-echo/0.0.1 (${BIN}${EXE}, ${PROFILE})"
