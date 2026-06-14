#!/usr/bin/env bash
# Build-only: compile this suite's test binaries without running them,
# so the root `test.sh` can pre-build them up front (before any suite
# spawns a server) and the parallel test phase never relinks a running
# `.exe` (the relink race). `nextest list` builds exactly what this
# suite's `test.sh` `nextest run` builds, so the later run is a cache
# hit. Mirror that invocation so the fingerprints match.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

"$NEXTEST" nextest list --manifest-path "$SCRIPT_DIR/Cargo.toml" >/dev/null
