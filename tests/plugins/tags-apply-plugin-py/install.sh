#!/usr/bin/env bash
# Install the tags-apply-plugin-py fixture (a Python plugin) into the shared
# test OBJECTIVEAI_DIR. No binary to build: copy plugin.py into the coordinate's
# cli/ dir and write a manifest whose `exec` runs the script under the
# objectiveai-sdk-py venv python — that's where the compiled `_pyo3` extension +
# the editable install make `objectiveai_sdk` importable. The venv python's
# absolute path is baked into the (gitignored) manifest as a forward-slash path
# (cygpath -m on Windows) so the JSON is backslash-free and CreateProcess-safe.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

WIN_PY="$REPO_ROOT/objectiveai-sdk-py/venv/Scripts/python.exe"
NIX_PY="$REPO_ROOT/objectiveai-sdk-py/venv/bin/python"
if [ -f "$WIN_PY" ]; then
  PY=$(cygpath -m "$WIN_PY" 2>/dev/null || echo "$WIN_PY")
elif [ -f "$NIX_PY" ]; then
  PY="$NIX_PY"
else
  echo "install: sdk-py venv python not found ($WIN_PY or $NIX_PY) — build objectiveai-sdk-py first" >&2
  exit 1
fi

VDIR="$OBJECTIVEAI_DIR/bin/plugins/objectiveai/tags-apply-py/0.0.1"
mkdir -p "$VDIR/cli"
cp -f "$SCRIPT_DIR/plugin.py" "$VDIR/cli/plugin.py"

# Only the current platform's `exec` is read (platform_exec); all three point at
# the resolved venv python so this install works regardless of which entry the
# host selects. The manifest is per-machine (under the gitignored .objectiveai).
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "objectiveai",
  "name": "tags-apply-py",
  "version": "0.0.1",
  "description": "E2E fixture: Python plugin that applies a tag via the SDK plugin executor",
  "exec": {
    "windows": ["${PY}", "./plugin.py"],
    "linux": ["${PY}", "./plugin.py"],
    "macos": ["${PY}", "./plugin.py"]
  },
  "cli_zip": {}
}
JSON
echo "install: plugins/objectiveai/tags-apply-py/0.0.1 ($PY plugin.py)"
