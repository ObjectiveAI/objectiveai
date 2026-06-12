#!/usr/bin/env bash
# Builds the five binaries the committed `.objectiveai/bin` shims
# point at (`target/debug/objectiveai{-cli,-api,-viewer,-mcp,-db}`),
# the `objectiveai-test-cleanup` binary `test-cleanup.sh` execs, and
# the fixture tool/plugin crates whose committed manifests `cargo
# run` them at test runtime — pre-building keeps every runtime
# invocation off cargo entirely.
#
# One cargo invocation = one feature resolution and maximal internal
# parallelism across the graphs. The shims themselves never invoke
# cargo, so this is the ONLY place test infrastructure builds these
# binaries — concurrent test processes can never trigger rebuilds or
# relink races against binaries that are currently running. Every
# suite's `test.sh` runs this at start (in parallel with
# `test-cleanup.sh`) unless the root `test.sh` already did.
#
# Note: `objectiveai-viewer` needs the frontend dist built once
# (`pnpm run build` in objectiveai-viewer/) before its crate can
# compile.
#
# Usage:
#   bash test-build.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

cargo build \
  --manifest-path "$REPO_ROOT/Cargo.toml" \
  --bin objectiveai-cli \
  --bin objectiveai-api \
  --bin objectiveai-viewer \
  --bin objectiveai-mcp \
  --bin objectiveai-db \
  --bin objectiveai-test-cleanup \
  --bin count-tool \
  --bin test-mcp-plugin-named \
  --bin hello-plugin \
  --bin test-mcp-plugin \
  --bin test-mcp-plugin-foo-headers \
  --bin hello-tool \
  --bin error-tool
