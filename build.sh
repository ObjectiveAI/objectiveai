#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 0 (sequential, first): install build/dev tools into ./bin/
#   (inlined below; wasm-pack, maturin, cargo-nextest, pinned from
#   [workspace.metadata.tools] in Cargo.toml). Runs first and alone —
#   the wasm/cffi (phase 2) and SDK (phase 3) builds invoke these tools.
# Phase 1: objectiveai-json-schema (its output feeds the phase-3 SDK codegen)
# Background: objectiveai-cli + mcp + claude-agent-sdk runners (after phase 1, concurrent with phases 2+3)
# Phase 2 (parallel): objectiveai-sdk-rs-wasm-js + objectiveai-sdk-rs-cffi
# Phase 3 (parallel): objectiveai-sdk-js + objectiveai-sdk-py + objectiveai-sdk-go
#                     (objectiveai-sdk-py builds its bundled Rust extension via maturin)
#                     (objectiveai-dotnet is disconnected from the root build for now;
#                     run `bash objectiveai-dotnet/build.sh` directly if you need it.)
# The viewer is NOT built here. Nothing consumes
# objectiveai-viewer/embed/ anymore (the cli stopped embedding the
# viewer binary; its build.rs only sets linker flags), and the GitHub
# Release viewer legs build their own binaries via
# objectiveai-viewer/install.sh. Run `bash objectiveai-viewer/build.sh`
# directly if you want a local embed build.
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

# ── Phase 0: build/dev tools ────────────────────────────────────────────
# Installs wasm-pack, maturin, and cargo-nextest into ./bin/ using the
# versions pinned in [workspace.metadata.tools] in Cargo.toml. Must come
# first — phases 2 and 3 invoke these tools. Output is captured to
# .logs/build/build-bin.txt to match the other build legs.
build_bin() {
  local WASM_PACK_VERSION MATURIN_VERSION CARGO_NEXTEST_VERSION BIN_DIR
  WASM_PACK_VERSION=$(sed -n 's/^wasm-pack *= *"\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml")
  MATURIN_VERSION=$(sed -n 's/^maturin *= *"\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml")
  CARGO_NEXTEST_VERSION=$(sed -n 's/^cargo-nextest *= *"\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml")

  [ -n "$WASM_PACK_VERSION" ] || { echo "ERROR: Could not read wasm-pack version from Cargo.toml" >&2; return 1; }
  [ -n "$MATURIN_VERSION" ] || { echo "ERROR: Could not read maturin version from Cargo.toml" >&2; return 1; }
  [ -n "$CARGO_NEXTEST_VERSION" ] || { echo "ERROR: Could not read cargo-nextest version from Cargo.toml" >&2; return 1; }

  BIN_DIR="$REPO_ROOT/bin"

  install_if_needed() {
    local name="$1" version="$2"
    local bin="$BIN_DIR/$name"
    if [ -x "$bin" ]; then
      local installed
      installed=$("$bin" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
      if [ "$installed" = "$version" ]; then
        echo "$name $version already installed, skipping."
        return
      fi
    fi
    echo "Installing $name $version..."
    cargo install "$name" --version "$version" --locked --root "$REPO_ROOT"
  }

  install_if_needed wasm-pack "$WASM_PACK_VERSION"
  install_if_needed maturin "$MATURIN_VERSION"
  install_if_needed cargo-nextest "$CARGO_NEXTEST_VERSION"

  echo "Done. Tools at $BIN_DIR/"
}

LOG_DIR="$REPO_ROOT/.logs/build"
mkdir -p "$LOG_DIR"
if build_bin > "$LOG_DIR/build-bin.txt" 2>&1; then
  echo "build-bin: SUCCESS"
else
  echo "build-bin: ERROR (see $LOG_DIR/build-bin.txt)"
  exit 1
fi

# Phase 1: json schema (its output feeds the phase-3 SDK codegen).
run_phase objectiveai-json-schema/build.sh

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

# Phase 2: wasm + cffi (need build tools from phase 0)
run_phase objectiveai-sdk-rs-wasm-js/build.sh objectiveai-sdk-rs-cffi/build.sh

# Phase 3: js + py + go + function-tree (js/py/go need wasm/cffi from
# phase 2; function-tree is a dependency-free React lib that just has to
# exist in dist/ before the viewer compiles in phase 4). objectiveai-dotnet
# is intentionally NOT part of this phase — its codegen has a duplicate-
# variant-property bug that breaks on newly-added internally-tagged enums;
# run `bash objectiveai-dotnet/build.sh` directly if you need it.
# objectiveai-sdk-py compiles its own Rust extension (_pyo3) via maturin as part of its build.
run_phase objectiveai-sdk-js/build.sh objectiveai-sdk-py/build.sh objectiveai-sdk-go/build.sh objectiveai-function-tree/build.sh

# Wait for the background embedded-binary builds.
FAILED=false
for pid in $MCP_FILESYSTEM_PID $CLAUDE_RUNNER_PID $CODEX_RUNNER_PID; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi
