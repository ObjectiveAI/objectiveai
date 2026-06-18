#!/usr/bin/env bash
# Install the test-mcp-plugin-named fixture into the shared test
# OBJECTIVEAI_DIR. One binary backs ten coordinates — the dup-*/same-*
# sets the duplicate-detection tests rely on — so this writes ten
# manifests, all execing the same copied-in binary.
#
# Expects the binary to be already built (build.sh, or `cargo build -p
# test-mcp-plugin-named`). Self-contained on purpose — each fixture's
# installer is independent so it can diverge as needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

BIN="test-mcp-plugin-named"
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

# dup-* and same-* across alpha..echo, all testorg/1.0.0.
for suffix in alpha bravo charlie delta echo; do
  for prefix in dup same; do
    name="${prefix}-${suffix}"
    VDIR="$OBJECTIVEAI_DIR/bin/plugins/testorg/$name/1.0.0"
    mkdir -p "$VDIR/cli"
    cp -f "$SRC" "$VDIR/cli/$BIN$EXE"
    # exec uses a leading ./ so it resolves against the cli/ CWD at run
    # time rather than being PATH-looked-up as a bare name.
    cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "testorg",
  "name": "$name",
  "version": "1.0.0",
  "description": "$name fixture",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {},
  "mcp_servers": [{"name":"demo","authorization":false}]
}
JSON
    echo "install: plugins/testorg/$name/1.0.0 (${BIN}${EXE}, ${PROFILE})"
  done
done
