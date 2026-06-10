#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 1 (parallel): build-bin + objectiveai-json-schema
# Background: objectiveai-cli + mcp + claude-agent-sdk runners (after phase 1, concurrent with phases 2+3)
# Phase 2 (parallel): objectiveai-sdk-rs-wasm-js + objectiveai-sdk-rs-cffi
# Phase 3 (parallel): objectiveai-sdk-js + objectiveai-sdk-py + objectiveai-sdk-go
#                     (objectiveai-sdk-py builds its bundled Rust extension via maturin)
#                     (objectiveai-dotnet is disconnected from the root build for now;
#                     run `bash objectiveai-dotnet/build.sh` directly if you need it.)
# Phase 4 (sequential): objectiveai-viewer release (cross-platform)
#                       then host-target debug. Sequential because both invoke
#                       `tauri build`, which holds the workspace cargo target/
#                       lock — running them in parallel deadlocks. Both
#                       artifacts are required: cargo test / cargo build of
#                       objectiveai-cli compiles in debug and its build.rs
#                       validates the viewer's host-target debug embed.
#
# Usage:
#   bash build.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# Run a phase: launch all given scripts in parallel, wait for all, fail if any failed.
run_phase() {
  local pids=()
  for script in "$@"; do
    bash "$REPO_ROOT/$script" &
    pids+=($!)
  done

  local failed=false
  for pid in "${pids[@]}"; do
    if ! wait "$pid"; then
      failed=true
    fi
  done

  if $failed; then
    exit 1
  fi
}

# Phase 1: build tools + json schema
run_phase build-bin.sh objectiveai-json-schema/build.sh

# Embedded binaries (depend on phase 1, run concurrently with phases 2+3).
# mcp-filesystem is a cargo build pinned to linux-musl (Docker container
# injection); claude-agent-sdk-runner and codex-sdk-runner are PyInstaller.
# mcp-proxy is NOT embedded — objectiveai-api consumes it in-process as a
# regular cargo path dep, so its build is folded into the api's own cargo
# build.
bash "$REPO_ROOT/objectiveai-mcp-filesystem/build.sh" --target "$(uname -m)-unknown-linux-musl" &
MCP_FILESYSTEM_PID=$!
bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" &
CLAUDE_RUNNER_PID=$!
bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" &
CODEX_RUNNER_PID=$!

# Phase 2: wasm + cffi (need build tools from phase 1)
run_phase objectiveai-sdk-rs-wasm-js/build.sh objectiveai-sdk-rs-cffi/build.sh

# Phase 3: js + py + go + function-tree (js/py/go need wasm/cffi from
# phase 2; function-tree is a dependency-free React lib that just has to
# exist in dist/ before the viewer compiles in phase 4). objectiveai-dotnet
# is intentionally NOT part of this phase — its codegen has a duplicate-
# variant-property bug that breaks on newly-added internally-tagged enums;
# run `bash objectiveai-dotnet/build.sh` directly if you need it.
# objectiveai-sdk-py compiles its own Rust extension (_pyo3) via maturin as part of its build.
run_phase objectiveai-sdk-js/build.sh objectiveai-sdk-py/build.sh objectiveai-sdk-go/build.sh objectiveai-function-tree/build.sh

# Wait for background builds before running viewer (viewer depends on objectiveai-sdk-js)
FAILED=false
for pid in $MCP_FILESYSTEM_PID $CLAUDE_RUNNER_PID $CODEX_RUNNER_PID; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi

# Phase 4: viewer (depends on objectiveai-sdk-js package being built in phase 3).
# Two artifacts are produced: a cross-platform release embed (consumed by
# published CLI binaries) and a host-target debug embed (consumed by
# `cargo test`/`cargo build` of objectiveai-cli during local development).
# Run sequentially — both invoke `tauri build` which holds the workspace
# cargo target/ lock; parallel invocations deadlock.
bash "$REPO_ROOT/objectiveai-viewer/build.sh" --release

HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"
bash "$REPO_ROOT/objectiveai-viewer/build.sh" --target "$HOST_TARGET"
