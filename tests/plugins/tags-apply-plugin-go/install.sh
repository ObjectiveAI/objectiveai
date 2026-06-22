#!/usr/bin/env bash
# Install the tags-apply-plugin-go fixture (a Go plugin) into the shared test
# OBJECTIVEAI_DIR. Like the Rust hello-plugin it is a compiled binary, but the
# repo's fixture co-build is Cargo-only, so this installer builds it itself with
# `go build` (against the local Go SDK via the fixture's go.mod `replace`). The
# SDK is codegen'd + built earlier in the suite, so its generated execute fns
# and deps are already present. Copies the binary into the coordinate's cli/ dir
# and writes the matching objectiveai.json. Self-contained, like the JS/Py
# fixture installers.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

BIN="tags-apply-go"
case "$(uname -s)" in CYGWIN*|MINGW*|MSYS*) EXE=".exe" ;; *) EXE="" ;; esac

VDIR="$OBJECTIVEAI_DIR/bin/plugins/objectiveai/tags-apply-go/0.0.1"
mkdir -p "$VDIR/cli"
( cd "$SCRIPT_DIR" && go build -o "$VDIR/cli/$BIN$EXE" . )

# exec uses a leading ./ so it resolves against the cli/ CWD at run time rather
# than being PATH-looked-up as a bare name.
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "objectiveai",
  "name": "tags-apply-go",
  "version": "0.0.1",
  "description": "E2E fixture: Go plugin that applies a tag via the SDK plugin executor",
  "exec": {
    "windows": ["./${BIN}.exe"],
    "linux": ["./${BIN}"],
    "macos": ["./${BIN}"]
  },
  "cli_zip": {}
}
JSON
echo "install: plugins/objectiveai/tags-apply-go/0.0.1 (${BIN}${EXE})"
