#!/usr/bin/env bash
# Build-only: compile this suite's test binaries without running them,
# so the root `test.sh` can pre-build them up front (before any suite
# spawns a server) and the parallel test phase never relinks a running
# `.exe` (the relink race). `nextest list` builds exactly what this
# suite's `test.sh` `nextest run` builds. Mirror that invocation
# (`-p objectiveai-viewer --lib --tests`; the bin target is skipped —
# Tauri's deps can't link under cargo test's bin pass) so the
# fingerprints match. Assumes the frontend dist is already built (the
# root `build.sh` builds it before tests run).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

"$NEXTEST" nextest list --manifest-path "$REPO_ROOT/Cargo.toml" \
  -p objectiveai-viewer --lib --tests >/dev/null
