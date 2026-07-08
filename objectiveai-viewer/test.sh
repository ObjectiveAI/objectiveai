#!/usr/bin/env bash
# Run the viewer frontend unit tests (offline — no server, no Tauri).
#
# Covers the JS side of the Tauri shell — currently the plugin-bridge
# daemon-frame routing (a plugin tab must receive exactly its own
# plugins/run frames and nothing else). The Rust side is covered by
# cargo tests and the objectiveai-cli-tests e2e crate.
#
# Usage:
#   bash objectiveai-viewer/test.sh

set -euo pipefail

pnpm --filter objectiveai-viewer run test -- --reporter=verbose "$@"
