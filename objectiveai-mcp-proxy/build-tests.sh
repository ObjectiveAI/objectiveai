#!/usr/bin/env bash
# Build-only: compile this suite's binaries without running anything,
# so the root `test.sh` can pre-build them up front (before any suite
# spawns a server) and the parallel test phase never relinks a running
# `.exe` (the relink race). Builds the (debug) proxy + test-upstream
# the TS rig spawns, then `nextest list` builds the Rust integration
# test binaries — exactly what this suite's `test.sh` builds. The TS
# `pnpm` install/run stays in `test.sh` (not a relink concern).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

cargo build \
  --manifest-path "$REPO_ROOT/Cargo.toml" \
  -p objectiveai-mcp-proxy \
  -p test-upstream
"$NEXTEST" nextest list \
  --manifest-path "$REPO_ROOT/Cargo.toml" \
  -p objectiveai-mcp-proxy \
  --tests >/dev/null
