#!/usr/bin/env bash
# Build-only: compile this suite's test binaries without running them.
#
# The root `test.sh` runs every suite's `build-tests.sh` up front —
# BEFORE any suite spawns a server — so the parallel test phase finds
# every binary fresh and never relinks one that's currently running
# (the Windows "Access is denied" relink race: cargo can't overwrite a
# running `.exe`). `nextest list` builds exactly what this suite's
# `test.sh` `nextest run` builds, then lists instead of running, so the
# later run is a pure cache hit. Mirror the nextest invocation in this
# suite's `test.sh` so the fingerprints match.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NEXTEST="$REPO_ROOT/bin/cargo-nextest"

"$NEXTEST" nextest list --manifest-path "$SCRIPT_DIR/Cargo.toml" >/dev/null
