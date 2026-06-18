#!/usr/bin/env bash
# Install the count-tool fixture into the shared test OBJECTIVEAI_DIR. One
# binary backs ten coordinates — tool0..tool9 — so this writes ten
# manifests, all execing the same copied-in binary.
#
# Expects the binary to be already built (build.sh, or `cargo build -p
# count-tool`). Self-contained on purpose — each fixture's installer is
# independent so it can diverge as needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

BIN="count-tool"
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

for i in 0 1 2 3 4 5 6 7 8 9; do
  VDIR="$OBJECTIVEAI_DIR/bin/tools/testorg/tool$i/1.0.0"
  mkdir -p "$VDIR/cli"
  cp -f "$SRC" "$VDIR/cli/$BIN$EXE"
  # exec uses a leading ./ so it resolves against the cli/ CWD at run time
  # rather than being PATH-looked-up as a bare name.
  cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "testorg",
  "name": "tool$i",
  "version": "1.0.0",
  "description": "Counter tool $i",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {}
}
JSON
  echo "install: tools/testorg/tool$i/1.0.0 (${BIN}${EXE}, ${PROFILE})"
done
