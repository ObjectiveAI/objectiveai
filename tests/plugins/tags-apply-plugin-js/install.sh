#!/usr/bin/env bash
# Install the tags-apply-plugin-js fixture (a JavaScript plugin) into the shared
# test OBJECTIVEAI_DIR. Unlike the Rust fixtures there is no binary to build: we
# copy plugin.mjs into the coordinate's cli/ dir and stage the committed JS SDK
# (dist + package.json) into cli/node_modules/@objectiveai/sdk so the plugin's
# `import "@objectiveai/sdk"` resolves at run time (no pnpm symlink exists, and
# dist/ is committed — so no JS build step is required here). Self-contained on
# purpose, like the other fixture installers.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OBJECTIVEAI_DIR="${OBJECTIVEAI_DIR:-$REPO_ROOT/.objectiveai}"

SDK_JS="$REPO_ROOT/objectiveai-sdk-js"
[ -d "$SDK_JS/dist" ] || {
  echo "install: JS SDK dist not found at $SDK_JS/dist (build the JS SDK first)" >&2
  exit 1
}

VDIR="$OBJECTIVEAI_DIR/bin/plugins/objectiveai/tags-apply-js/0.0.1"
SDK_DEST="$VDIR/cli/node_modules/@objectiveai/sdk"
rm -rf "$SDK_DEST"
mkdir -p "$SDK_DEST"
cp -f "$SCRIPT_DIR/plugin.mjs" "$VDIR/cli/plugin.mjs"
cp -R "$SDK_JS/dist" "$SDK_DEST/"
cp -f "$SDK_JS/package.json" "$SDK_DEST/package.json"

# `node`/`./plugin.mjs`: a bare program name keeps PATH-lookup; the relative
# script resolves against the cli/ CWD at run time.
cat > "$VDIR/objectiveai.json" <<JSON
{
  "owner": "objectiveai",
  "name": "tags-apply-js",
  "version": "0.0.1",
  "description": "E2E fixture: JS plugin that applies a tag via the SDK plugin executor",
  "exec": {
    "windows": ["node", "./plugin.mjs"],
    "linux": ["node", "./plugin.mjs"],
    "macos": ["node", "./plugin.mjs"]
  },
  "cli_zip": {}
}
JSON
echo "install: plugins/objectiveai/tags-apply-js/0.0.1 (node plugin.mjs)"
