#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 0 (sequential, first): install build/dev tools into ./bin/
#   (inlined below; wasm-pack, maturin, cargo-nextest, pinned from
#   [workspace.metadata.tools] in Cargo.toml). Runs first and alone —
#   the wasm/cffi (phase 2) and SDK (phase 3) builds invoke these tools.
# Phase 1: objectiveai-json-schema (its output feeds the phase-4 SDK codegen)
# Phase 2 (background, concurrent with phases 3+4):
#   - claude-agent-sdk + codex-sdk runners (PyInstaller binaries the api
#     spawns at runtime from <OBJECTIVEAI_DIR>/bin/)
#   - a single cargo build of the five product crates (viewer, cli, api,
#     db, mcp) in ONE invocation so they share the compile cache
# Phase 3 (parallel): objectiveai-sdk-rs-wasm-js + objectiveai-sdk-rs-cffi
# Phase 4 (parallel): objectiveai-sdk-js + objectiveai-sdk-py + objectiveai-sdk-go
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
# Build profile defaults to debug. Pass --release for optimized builds;
# this propagates (via OBJECTIVEAI_BUILD_RELEASE) to the cffi, wasm-js,
# and pyo3 builds, which compile debug otherwise.
#
# Usage:
#   bash build.sh [--release]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# ── Build profile ───────────────────────────────────────────────────────
# --release → optimized. Exported as OBJECTIVEAI_BUILD_RELEASE so the
# sub-builds (cffi, wasm-js, pyo3) pick it up — run_phase launches them
# with no args, so the env var is how the profile reaches them.
RELEASE=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --release) RELEASE=1; shift ;;
    -h|--help) echo "Usage: bash build.sh [--release]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; echo "Usage: bash build.sh [--release]" >&2; exit 1 ;;
  esac
done
if [ "$RELEASE" = "1" ]; then
  export OBJECTIVEAI_BUILD_RELEASE=1
  echo "Build profile: release"
else
  echo "Build profile: debug (pass --release for optimized builds)"
fi

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

# Phase 1: json schema (its output feeds the phase-4 SDK codegen).
run_phase objectiveai-json-schema/build.sh

# Phase 2 (background, concurrent with phases 3+4):
#   (a) the two PyInstaller SDK runners (claude + codex), and
#   (b) one cargo build of the five product crates — viewer, cli, api, db,
#       mcp — in a SINGLE invocation so they share the compile cache (the
#       per-crate builds would otherwise recompile common dependencies and
#       serialize on the cargo target lock). Honors --release.
# mcp-proxy is NOT built here — objectiveai-api consumes it in-process as a
# regular cargo path dep, folded into the api's own cargo build.
bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" &
CLAUDE_RUNNER_PID=$!
bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" &
CODEX_RUNNER_PID=$!

CARGO_WORKSPACE_FLAG=""
[ "$RELEASE" = "1" ] && CARGO_WORKSPACE_FLAG="--release"
(
  cd "$REPO_ROOT"
  if cargo build $CARGO_WORKSPACE_FLAG \
       -p objectiveai-viewer \
       -p objectiveai-cli \
       -p objectiveai-api \
       -p objectiveai-db \
       -p objectiveai-mcp \
       > "$LOG_DIR/cargo-workspace.txt" 2>&1; then
    echo "cargo-workspace: SUCCESS"
  else
    echo "cargo-workspace: ERROR (see .logs/build/cargo-workspace.txt)"
    exit 1
  fi
) &
CARGO_WORKSPACE_PID=$!

# Phase 3: wasm + cffi (need build tools from phase 0)
run_phase objectiveai-sdk-rs-wasm-js/build.sh objectiveai-sdk-rs-cffi/build.sh

# Phase 4: js + py + go (all need wasm/cffi from phase 3). objectiveai-dotnet
# is intentionally NOT part of this phase — its codegen has a duplicate-
# variant-property bug that breaks on newly-added internally-tagged enums;
# run `bash objectiveai-dotnet/build.sh` directly if you need it.
# objectiveai-sdk-py compiles its own Rust extension (_pyo3) via maturin as part of its build.
run_phase objectiveai-sdk-js/build.sh objectiveai-sdk-py/build.sh objectiveai-sdk-go/build.sh

# Wait for the background phase-2 jobs (runners + the 5-crate cargo build).
FAILED=false
for pid in $CLAUDE_RUNNER_PID $CODEX_RUNNER_PID $CARGO_WORKSPACE_PID; do
  if ! wait "$pid"; then
    FAILED=true
  fi
done

if $FAILED; then
  exit 1
fi
