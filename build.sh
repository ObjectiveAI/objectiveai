#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 1 (parallel): build-bin + objectiveai-json-schema
# Background: objectiveai-cli + mcp + claude-agent-sdk runners (after phase 1, concurrent with phases 2+3)
# Phase 2 (parallel): objectiveai-rs-wasm-js + objectiveai-rs-pyo3 + objectiveai-rs-cffi
# Phase 3 (parallel): objectiveai-js + objectiveai-py + objectiveai-go + objectiveai-dotnet
# Phase 4 (after all): objectiveai-viewer (depends on objectiveai-js being built)
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

# CLI schema codegen (depends on phase 1, runs concurrently with phases 2+3)
bash "$REPO_ROOT/objectiveai-cli/build.sh" &
CLI_PID=$!

# Embedded binaries (depend on phase 1, run concurrently with phases 2+3).
# mcp is a cargo build; claude-agent-sdk-runner is PyInstaller.
# mcp always linux-musl (for Docker).
bash "$REPO_ROOT/objectiveai-mcp/build.sh" --target "$(uname -m)-unknown-linux-musl" &
MCP_PID=$!
bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner-py/build.sh" &
SDK_RUNNER_PY_PID=$!

# Phase 2: wasm + pyo3 + cffi (need build tools from phase 1)
run_phase objectiveai-rs-wasm-js/build.sh objectiveai-rs-pyo3/build.sh objectiveai-rs-cffi/build.sh

# Phase 3: js + py + go + dotnet (need wasm/pyo3/cffi from phase 2)
run_phase objectiveai-js/build.sh objectiveai-py/build.sh objectiveai-go/build.sh objectiveai-dotnet/build.sh

# Wait for background builds before running viewer (viewer depends on objectiveai-js)
FAILED=false
for pid in $CLI_PID $MCP_PID $SDK_RUNNER_PY_PID; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi

# Phase 4: viewer (depends on objectiveai-js package being built in phase 3)
bash "$REPO_ROOT/objectiveai-viewer/build.sh" --release
