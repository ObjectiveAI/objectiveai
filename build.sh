#!/usr/bin/env bash
# Builds all packages in dependency order with parallelism.
#
# Phase 1 (background, NOT awaited until the very end): the Rust product
#   binaries — one cargo build of viewer, cli, api, db, mcp in a SINGLE
#   invocation so they share the compile cache — plus the two PyInstaller
#   SDK runners (claude + codex). None of these need the phase-2 build
#   tools or the json schemas, so they start immediately and run
#   concurrently with everything below. (build_bin's `cargo install` uses a
#   throwaway target dir, so it doesn't fight this build for the workspace
#   target lock; the json-schema/cffi/pyo3 cargo steps do share it and so
#   serialize behind this build — fine, they're quick once they get it.)
# Phase 2 (parallel): build/dev tools (wasm-pack, maturin, cargo-nextest,
#   into ./bin/) + objectiveai-json-schema. Independent of each other; both
#   must finish before phases 3+4, which need the tools + the schemas.
# Phase 3 (parallel): objectiveai-sdk-rs-wasm-js + objectiveai-sdk-rs-cffi
# Phase 4 (parallel): objectiveai-sdk-js + objectiveai-sdk-py + objectiveai-sdk-go
#                     (objectiveai-sdk-py builds its bundled Rust extension via maturin)
#                     (objectiveai-dotnet is disconnected from the root build for now;
#                     run `bash objectiveai-dotnet/build.sh` directly if you need it.)
# Final: wait for phase 1, then package the HOST platform's 7 binaries into
#        the same per-platform zip the release ships
#        (objectiveai-<os>-<arch>.zip) and drop it in <OBJECTIVEAI_DIR>/bin
#        so the installer / `objectiveai update` can use it locally. Host
#        only — not the other five platforms.
# The viewer is NOT built (as a Tauri bundle) here. Nothing consumes
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
# Pass --no-zip to skip phase 1 (the product-binary + runner compilation)
# AND the final packaging — useful when you only want the schemas/tools/SDKs
# and don't need the per-platform zip. --no-sdk is the inverse: it skips
# phases 2-4 (build/dev tools incl. nextest, json schema, wasm/cffi, and the
# JS/Py/Go SDKs), leaving only phase 1 + packaging. Passing both skips every
# phase — by construction, nothing happens.
#
# Usage:
#   bash build.sh [--release] [--no-zip] [--no-sdk]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

# ── Build profile ───────────────────────────────────────────────────────
# --release → optimized. Exported as OBJECTIVEAI_BUILD_RELEASE so the
# sub-builds (cffi, wasm-js, pyo3) pick it up — run_phase launches them
# with no args, so the env var is how the profile reaches them.
RELEASE=0
NO_ZIP=0
NO_SDK=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --release) RELEASE=1; shift ;;
    --no-zip)  NO_ZIP=1; shift ;;
    --no-sdk)  NO_SDK=1; shift ;;
    -h|--help) echo "Usage: bash build.sh [--release] [--no-zip] [--no-sdk]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; echo "Usage: bash build.sh [--release] [--no-zip] [--no-sdk]" >&2; exit 1 ;;
  esac
done
if [ "$RELEASE" = "1" ]; then
  export OBJECTIVEAI_BUILD_RELEASE=1
  echo "Build profile: release"
else
  echo "Build profile: debug (pass --release for optimized builds)"
fi
if [ "$NO_ZIP" = "1" ]; then
  echo "Skipping phase 1 (product binaries + runners) and packaging (--no-zip)."
fi
if [ "$NO_SDK" = "1" ]; then
  echo "Skipping phases 2-4 (build tools, json schema, wasm/cffi, SDKs) (--no-sdk)."
fi

LOG_DIR="$REPO_ROOT/.logs/build"
mkdir -p "$LOG_DIR"

# PROFILE_FLAG is shared by the runners and the cargo build so the
# embed/<profile>/ runner path lines up with target/<profile>/ for packaging.
PROFILE_FLAG=""
[ "$RELEASE" = "1" ] && PROFILE_FLAG="--release"

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

# Installs wasm-pack, maturin, and cargo-nextest into ./bin/ using the
# versions pinned in [workspace.metadata.tools] in Cargo.toml. Runs in
# phase 2 (parallel with json-schema); phases 3 and 4 invoke these tools,
# so it must finish before them. Output captured to .logs/build/build-bin.txt.
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

# ── Phase 1 (background; awaited at the very end) ───────────────────────
# The five product crates in one cargo build (shared cache) + the two
# PyInstaller SDK runners. mcp-proxy is NOT built here — objectiveai-api
# consumes it in-process as a regular cargo path dep, folded into the api's
# own cargo build. Skipped entirely under --no-zip (these binaries exist
# only to be packaged; nothing in phases 2-4 depends on them).
if [ "$NO_ZIP" != "1" ]; then
  bash "$REPO_ROOT/objectiveai-claude-agent-sdk-runner/build.sh" $PROFILE_FLAG &
  CLAUDE_RUNNER_PID=$!
  bash "$REPO_ROOT/objectiveai-codex-sdk-runner/build.sh" $PROFILE_FLAG &
  CODEX_RUNNER_PID=$!

  (
    cd "$REPO_ROOT"
    if cargo build $PROFILE_FLAG \
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
fi

# ── Phases 2-4 (the SDK toolchain) ──────────────────────────────────────
# Skipped entirely under --no-sdk: the build/dev tools (including
# cargo-nextest), the json schema, the wasm/cffi bindings, and the
# JS/Py/Go SDKs. Phase 1 + packaging don't depend on any of these.
if [ "$NO_SDK" != "1" ]; then
  # Phase 2 (parallel): build/dev tools + json schema.
  (
    if build_bin > "$LOG_DIR/build-bin.txt" 2>&1; then
      echo "build-bin: SUCCESS"
    else
      echo "build-bin: ERROR (see .logs/build/build-bin.txt)"
      exit 1
    fi
  ) &
  BUILD_BIN_PID=$!
  bash "$REPO_ROOT/objectiveai-json-schema/build.sh" &
  JSON_SCHEMA_PID=$!

  PHASE2_FAILED=false
  for pid in $BUILD_BIN_PID $JSON_SCHEMA_PID; do
    if ! wait "$pid"; then
      PHASE2_FAILED=true
    fi
  done
  if $PHASE2_FAILED; then
    exit 1
  fi

  # Phase 3: wasm + cffi (need the build tools from phase 2)
  run_phase objectiveai-sdk-rs-wasm-js/build.sh objectiveai-sdk-rs-cffi/build.sh

  # Phase 4: js + py + go (js/go need wasm/cffi from phase 3; all need the
  # json schemas from phase 2). objectiveai-dotnet is intentionally NOT part
  # of this phase — its codegen has a duplicate-variant-property bug that
  # breaks on newly-added internally-tagged enums; run
  # `bash objectiveai-dotnet/build.sh` directly if you need it.
  # objectiveai-sdk-py compiles its own Rust extension (_pyo3) via maturin as part of its build.
  run_phase objectiveai-sdk-js/build.sh objectiveai-sdk-py/build.sh objectiveai-sdk-go/build.sh
fi

# Wait for the background phase-1 jobs (the 5-crate cargo build + runners).
# Skipped under --no-zip (phase 1 never launched).
if [ "$NO_ZIP" != "1" ]; then
  FAILED=false
  for pid in $CLAUDE_RUNNER_PID $CODEX_RUNNER_PID $CARGO_WORKSPACE_PID; do
    if ! wait "$pid"; then
      FAILED=true
    fi
  done

  if $FAILED; then
    exit 1
  fi
fi

# ── Package the host's 7 binaries into <dir>/bin/<release-asset>.zip ─────
# Bundles the freshly-built binaries into the same per-platform zip the
# GitHub Release ships (objectiveai-<os>-<arch>.zip) and drops it in
# <OBJECTIVEAI_DIR>/bin so the installer / `objectiveai update` can pick it
# up locally. Host platform only — not the other 5. Uses `python -m
# zipfile` (cross-platform; `zip(1)` is absent in Git Bash). The cli crate
# builds as `objectiveai-cli` but ships as `objectiveai`.
package_host_zip() {
  local os arch ext profile host_triple
  case "$(uname -s)" in
    Linux*)               os="linux"   ;;
    Darwin*)              os="macos"   ;;
    CYGWIN*|MINGW*|MSYS*) os="windows" ;;
    *) echo "package: unsupported OS: $(uname -s)" >&2; return 1 ;;
  esac
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64)  arch="x86_64"  ;;
    arm64|aarch64) arch="aarch64" ;;
    *) echo "package: unsupported architecture: $arch" >&2; return 1 ;;
  esac
  [ "$os" = "windows" ] && ext=".exe" || ext=""
  profile="debug"; [ "$RELEASE" = "1" ] && profile="release"
  host_triple=$(rustc -vV | awk '/^host:/{print $2}')

  local py
  py=$(command -v python3 || command -v python || true)
  [ -n "$py" ] || { echo "package: need python3 to build the zip" >&2; return 1; }

  local asset="objectiveai-${os}-${arch}.zip"
  local install_dir="${OBJECTIVEAI_DIR:-$HOME/.objectiveai}"
  local bin_dir="$install_dir/bin"
  mkdir -p "$bin_dir"

  # Stage the 7 binaries under their shipped names (built-name -> ship-name).
  local stage="$REPO_ROOT/target/.package-stage.$$"
  rm -rf "$stage"; mkdir -p "$stage"

  local cargo_dir="$REPO_ROOT/target/$profile"
  # built basename | shipped basename
  local pairs="objectiveai-cli|objectiveai objectiveai-api|objectiveai-api objectiveai-viewer|objectiveai-viewer objectiveai-mcp|objectiveai-mcp objectiveai-db|objectiveai-db"
  local entry built ship src
  for entry in $pairs; do
    built="${entry%%|*}"; ship="${entry##*|}"
    src="$cargo_dir/$built$ext"
    if [ ! -f "$src" ]; then
      echo "package: missing $src" >&2; rm -rf "$stage"; return 1
    fi
    cp "$src" "$stage/$ship$ext"
  done

  local r
  for r in objectiveai-claude-agent-sdk-runner objectiveai-codex-sdk-runner; do
    src="$REPO_ROOT/$r/embed/$host_triple/$profile/$r$ext"
    if [ ! -f "$src" ]; then
      echo "package: missing $src" >&2; rm -rf "$stage"; return 1
    fi
    cp "$src" "$stage/$r$ext"
  done

  # Build the zip to a temp path, then move into place (no partial asset).
  local out="$bin_dir/$asset" tmp="$bin_dir/$asset.partial.$$"
  rm -f "$tmp"
  if ! ( cd "$stage" && "$py" -m zipfile -c "$tmp" * ); then
    rm -f "$tmp"; rm -rf "$stage"
    echo "package: zip creation failed" >&2; return 1
  fi
  mv -f "$tmp" "$out"
  rm -rf "$stage"
  echo "Packaged $out ($profile)"
}

if [ "$NO_ZIP" != "1" ]; then
  if ! package_host_zip; then
    exit 1
  fi
fi
